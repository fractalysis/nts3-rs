use core::mem::MaybeUninit;

use nts3_sys::UnitRuntimeHooks;

use crate::allocator::{self, AllocationStats, ArenaError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LifecycleState {
    Uninitialized,
    Initializing,
    Ready,
    Suspended,
    TearingDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeInitError {
    InvalidState,
    MissingMemoryHooks,
    InvalidMemoryBudget,
    MemoryUnavailable,
    MemoryAllocationFailed,
    AddressOverflow,
    ConstructionFailed,
}

impl From<ArenaError> for RuntimeInitError {
    fn from(error: ArenaError) -> Self {
        match error {
            ArenaError::AlreadyActive | ArenaError::Sealed | ArenaError::Inactive => {
                Self::InvalidState
            }
            ArenaError::MissingHooks => Self::MissingMemoryHooks,
            ArenaError::InvalidBudget | ArenaError::BudgetTooLarge => Self::InvalidMemoryBudget,
            ArenaError::Unavailable => Self::MemoryUnavailable,
            ArenaError::AllocationFailed => Self::MemoryAllocationFailed,
            ArenaError::AddressOverflow | ArenaError::InvalidAlignment => Self::AddressOverflow,
        }
    }
}

/// Static-RAM owner for one concrete plugin/runtime value.
///
/// The value is written only after the SDRAM arena is active, published only
/// after allocation is sealed, and dropped before the original SDRAM pointer is
/// returned to Korg.
pub(crate) struct RuntimeState<T> {
    lifecycle: LifecycleState,
    value: MaybeUninit<T>,
    allocation_stats: Option<AllocationStats>,
}

impl<T> RuntimeState<T> {
    pub(crate) const fn new() -> Self {
        Self {
            lifecycle: LifecycleState::Uninitialized,
            value: MaybeUninit::uninit(),
            allocation_stats: None,
        }
    }

    pub(crate) fn lifecycle(&self) -> LifecycleState {
        self.lifecycle
    }

    pub(crate) fn allocation_stats(&self) -> Option<AllocationStats> {
        self.allocation_stats
    }

    pub(crate) fn initialize(
        &mut self,
        hooks: UnitRuntimeHooks,
        sdram_bytes: u32,
        construct: impl FnOnce() -> Result<T, RuntimeInitError>,
    ) -> Result<(), RuntimeInitError> {
        if self.lifecycle != LifecycleState::Uninitialized {
            return Err(RuntimeInitError::InvalidState);
        }

        self.lifecycle = LifecycleState::Initializing;
        if let Err(error) = allocator::activate_target(hooks, sdram_bytes) {
            self.lifecycle = LifecycleState::Uninitialized;
            return Err(error.into());
        }

        let value = match construct() {
            Ok(value) => value,
            Err(error) => {
                let _ = allocator::reset_and_release();
                self.lifecycle = LifecycleState::Uninitialized;
                return Err(error);
            }
        };
        self.value.write(value);

        let stats = match allocator::seal() {
            Ok(stats) => stats,
            Err(error) => {
                // SAFETY: the successful `write` above initialized the value,
                // and readiness has not been published. Drop it before release.
                unsafe { self.value.assume_init_drop() };
                let _ = allocator::reset_and_release();
                self.lifecycle = LifecycleState::Uninitialized;
                return Err(error.into());
            }
        };
        self.allocation_stats = Some(stats);
        self.lifecycle = LifecycleState::Ready;
        Ok(())
    }

    pub(crate) fn ready_mut(&mut self) -> Option<&mut T> {
        if self.lifecycle != LifecycleState::Ready {
            return None;
        }
        // SAFETY: only successful initialization enters `Ready`; teardown
        // leaves it before dropping the value, and `&mut self` is exclusive.
        Some(unsafe { self.value.assume_init_mut() })
    }

    pub(crate) fn active_mut(&mut self) -> Option<&mut T> {
        if !matches!(
            self.lifecycle,
            LifecycleState::Ready | LifecycleState::Suspended
        ) {
            return None;
        }
        // SAFETY: both accepted lifecycle states contain the initialized value,
        // and `&mut self` prevents another callback reference.
        Some(unsafe { self.value.assume_init_mut() })
    }

    pub(crate) fn suspend(&mut self) -> Result<(), RuntimeInitError> {
        if self.lifecycle != LifecycleState::Ready {
            return Err(RuntimeInitError::InvalidState);
        }
        self.lifecycle = LifecycleState::Suspended;
        Ok(())
    }

    pub(crate) fn resume(&mut self) -> Result<(), RuntimeInitError> {
        if self.lifecycle != LifecycleState::Suspended {
            return Err(RuntimeInitError::InvalidState);
        }
        self.lifecycle = LifecycleState::Ready;
        Ok(())
    }

    pub(crate) fn teardown(
        &mut self,
        before_drop: impl FnOnce(&mut T),
    ) -> Result<(), RuntimeInitError> {
        if !matches!(
            self.lifecycle,
            LifecycleState::Ready | LifecycleState::Suspended
        ) {
            return Err(RuntimeInitError::InvalidState);
        }

        self.lifecycle = LifecycleState::TearingDown;
        // SAFETY: Ready/Suspended prove initialization, and TearingDown blocks
        // publication through `ready_mut` while the callback and drop run.
        let value = unsafe { self.value.assume_init_mut() };
        before_drop(value);
        // SAFETY: this is the one initialized value, and lifecycle prevents a
        // second teardown. It is dropped before SDRAM is released below.
        unsafe { self.value.assume_init_drop() };
        self.allocation_stats = None;
        let release = allocator::reset_and_release().map_err(RuntimeInitError::from);
        self.lifecycle = LifecycleState::Uninitialized;
        release
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::alloc::{GlobalAlloc, Layout};
    use core::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::allocator::FrameworkAllocator;

    static SERIAL: Mutex<()> = Mutex::new(());
    static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
    static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static DROPS: AtomicUsize = AtomicUsize::new(0);

    #[repr(align(4096))]
    struct Backing([u8; 4096]);

    static mut BACKING: Backing = Backing([0; 4096]);

    unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
        ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
        if size <= 4096 {
            // SAFETY: tests are serialized and the arena is released before the
            // next lifecycle. `addr_of_mut!` creates no static mutable reference.
            // SAFETY: the serialized test is the only accessor to this static
            // backing storage for the complete arena lifecycle.
            unsafe { core::ptr::addr_of_mut!(BACKING.0).cast::<u8>() }
        } else {
            ptr::null_mut()
        }
    }

    unsafe extern "C" fn allocate_null(_size: usize) -> *mut u8 {
        ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
        ptr::null_mut()
    }

    unsafe extern "C" fn free(pointer: *const u8) {
        // SAFETY: comparing the address does not read the mutable static, and
        // the serialized test owns its arena lifecycle.
        let expected = unsafe { core::ptr::addr_of!(BACKING.0).cast::<u8>() };
        assert_eq!(pointer, expected);
        FREE_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn available() -> usize {
        4096
    }

    fn hooks() -> UnitRuntimeHooks {
        UnitRuntimeHooks::new(ptr::null(), Some(allocate), Some(free), Some(available))
    }

    struct AllocatingValue {
        pointer: *mut u8,
        layout: Layout,
        free_calls_when_created: usize,
    }

    impl AllocatingValue {
        fn new() -> Result<Self, RuntimeInitError> {
            let layout = Layout::from_size_align(257, 256).unwrap();
            // SAFETY: the framework arena is active during the constructor and
            // the returned pointer is retained only until runtime teardown.
            let pointer = unsafe { FrameworkAllocator.alloc(layout) };
            if pointer.is_null() {
                return Err(RuntimeInitError::ConstructionFailed);
            }
            Ok(Self {
                pointer,
                layout,
                free_calls_when_created: FREE_CALLS.load(Ordering::SeqCst),
            })
        }
    }

    impl Drop for AllocatingValue {
        fn drop(&mut self) {
            assert_eq!(
                FREE_CALLS.load(Ordering::SeqCst),
                self.free_calls_when_created
            );
            DROPS.fetch_add(1, Ordering::SeqCst);
            // SAFETY: pointer/layout are the matching arena allocation. Target
            // deallocation is intentionally a no-op before whole-arena release.
            unsafe { FrameworkAllocator.dealloc(self.pointer, self.layout) };
        }
    }

    fn reset_counters() {
        ALLOC_CALLS.store(0, Ordering::SeqCst);
        FREE_CALLS.store(0, Ordering::SeqCst);
        DROPS.store(0, Ordering::SeqCst);
    }

    #[test]
    fn successful_repeated_lifecycles_use_one_alloc_free_pair() {
        let _guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        reset_counters();
        let mut runtime = RuntimeState::<AllocatingValue>::new();

        for lifecycle in 1..=3 {
            runtime
                .initialize(hooks(), 4096, AllocatingValue::new)
                .unwrap();
            assert_eq!(runtime.lifecycle(), LifecycleState::Ready);
            assert!(runtime.ready_mut().is_some());
            assert_eq!(runtime.allocation_stats().unwrap().allocations, 1);
            assert_eq!(allocator::stats(), runtime.allocation_stats());
            runtime.suspend().unwrap();
            assert!(runtime.ready_mut().is_none());
            assert!(runtime.active_mut().is_some());
            runtime.resume().unwrap();
            runtime.teardown(|_| {}).unwrap();
            assert_eq!(runtime.lifecycle(), LifecycleState::Uninitialized);
            assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), lifecycle);
            assert_eq!(FREE_CALLS.load(Ordering::SeqCst), lifecycle);
            assert_eq!(DROPS.load(Ordering::SeqCst), lifecycle);
            assert_eq!(allocator::stats(), None);
        }
    }

    #[test]
    fn allocation_failure_never_constructs_value() {
        let _guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        reset_counters();
        let mut runtime = RuntimeState::<AllocatingValue>::new();
        let mut constructed = false;
        let result = runtime.initialize(hooks(), 4097, || {
            constructed = true;
            AllocatingValue::new()
        });
        assert_eq!(result, Err(RuntimeInitError::MemoryUnavailable));
        assert!(!constructed);
        assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.lifecycle(), LifecycleState::Uninitialized);
    }

    #[test]
    fn null_arena_allocation_fails_without_construction_or_free() {
        let _guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        reset_counters();
        let null_hooks = UnitRuntimeHooks::new(
            ptr::null(),
            Some(allocate_null),
            Some(free),
            Some(available),
        );
        let mut runtime = RuntimeState::<u32>::new();
        let mut constructed = false;
        let result = runtime.initialize(null_hooks, 4096, || {
            constructed = true;
            Ok(7)
        });
        assert_eq!(result, Err(RuntimeInitError::MemoryAllocationFailed));
        assert!(!constructed);
        assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.lifecycle(), LifecycleState::Uninitialized);
    }

    #[test]
    fn construction_failure_releases_arena_and_allows_reinit() {
        let _guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        reset_counters();
        let mut runtime = RuntimeState::<AllocatingValue>::new();
        assert_eq!(
            runtime.initialize(hooks(), 4096, || Err(RuntimeInitError::ConstructionFailed)),
            Err(RuntimeInitError::ConstructionFailed)
        );
        assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.lifecycle(), LifecycleState::Uninitialized);

        runtime
            .initialize(hooks(), 4096, AllocatingValue::new)
            .unwrap();
        runtime.teardown(|_| {}).unwrap();
        assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), 2);
        assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn invalid_transitions_do_not_touch_uninitialized_storage() {
        let _guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let mut runtime = RuntimeState::<u32>::new();
        assert_eq!(runtime.suspend(), Err(RuntimeInitError::InvalidState));
        assert_eq!(runtime.resume(), Err(RuntimeInitError::InvalidState));
        assert_eq!(
            runtime.teardown(|_| {}),
            Err(RuntimeInitError::InvalidState)
        );
        assert!(runtime.ready_mut().is_none());
        assert!(runtime.active_mut().is_none());
    }
}

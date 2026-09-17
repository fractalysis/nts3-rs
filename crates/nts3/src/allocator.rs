use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self, NonNull};

use nts3_sys::{UnitRuntimeHooks, UnitRuntimeSdramFreeFn};

/// NTS-3's documented per-runtime SDRAM ceiling.
pub(crate) const PLATFORM_SDRAM_LIMIT: u32 = 3 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AllocationStats {
    pub budget: u32,
    pub requested_bytes: u32,
    pub alignment_padding: u32,
    pub position: u32,
    pub high_water: u32,
    pub allocations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArenaError {
    AlreadyActive,
    MissingHooks,
    InvalidBudget,
    BudgetTooLarge,
    Unavailable,
    AllocationFailed,
    AddressOverflow,
    InvalidAlignment,
    Sealed,
    Inactive,
}

#[derive(Clone, Copy, Debug)]
struct ArenaCursor {
    base: u32,
    stats: AllocationStats,
}

impl ArenaCursor {
    fn new(base: u32, budget: u32) -> Result<Self, ArenaError> {
        if budget == 0 {
            return Err(ArenaError::InvalidBudget);
        }
        base.checked_add(budget)
            .ok_or(ArenaError::AddressOverflow)?;
        Ok(Self {
            base,
            stats: AllocationStats {
                budget,
                ..AllocationStats::default()
            },
        })
    }

    fn reserve(&mut self, size: u32, align: u32) -> Result<u32, ArenaError> {
        if size == 0 || align == 0 || !align.is_power_of_two() {
            return Err(ArenaError::InvalidAlignment);
        }

        let current_address = self
            .base
            .checked_add(self.stats.position)
            .ok_or(ArenaError::AddressOverflow)?;
        let aligned_address = current_address
            .checked_add(align - 1)
            .ok_or(ArenaError::AddressOverflow)?
            & !(align - 1);
        let aligned_position = aligned_address
            .checked_sub(self.base)
            .ok_or(ArenaError::AddressOverflow)?;
        let padding = aligned_position
            .checked_sub(self.stats.position)
            .ok_or(ArenaError::AddressOverflow)?;
        let end = aligned_position
            .checked_add(size)
            .ok_or(ArenaError::AddressOverflow)?;
        if end > self.stats.budget {
            return Err(ArenaError::AllocationFailed);
        }

        let requested_bytes = self
            .stats
            .requested_bytes
            .checked_add(size)
            .ok_or(ArenaError::AddressOverflow)?;
        let alignment_padding = self
            .stats
            .alignment_padding
            .checked_add(padding)
            .ok_or(ArenaError::AddressOverflow)?;
        let allocations = self
            .stats
            .allocations
            .checked_add(1)
            .ok_or(ArenaError::AddressOverflow)?;

        self.stats.requested_bytes = requested_bytes;
        self.stats.alignment_padding = alignment_padding;
        self.stats.position = end;
        self.stats.high_water = self.stats.high_water.max(end);
        self.stats.allocations = allocations;
        Ok(aligned_position)
    }
}

struct Arena {
    original: NonNull<u8>,
    cursor: ArenaCursor,
    free: Option<UnitRuntimeSdramFreeFn>,
}

impl Arena {
    fn new(
        original: NonNull<u8>,
        budget: u32,
        free: Option<UnitRuntimeSdramFreeFn>,
    ) -> Result<Self, ArenaError> {
        // Only the low 32 bits are relevant to alignment on a 32-bit target.
        // Host probes intentionally use the same model even when pointers are
        // wider, so all offset and exhaustion arithmetic remains target-like.
        let base = original.as_ptr() as usize as u32;
        Ok(Self {
            original,
            cursor: ArenaCursor::new(base, budget)?,
            free,
        })
    }

    fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let Ok(size) = u32::try_from(layout.size()) else {
            return ptr::null_mut();
        };
        let Ok(align) = u32::try_from(layout.align()) else {
            return ptr::null_mut();
        };
        let Ok(offset) = self.cursor.reserve(size, align) else {
            return ptr::null_mut();
        };

        // SAFETY: `reserve` proved that `offset..offset + size` is within the
        // one live arena allocation. It also aligned the resulting address for
        // `layout`, and bump allocation never returns this range again.
        unsafe { self.original.as_ptr().add(offset as usize) }
    }

    fn stats(&self) -> AllocationStats {
        self.cursor.stats
    }

    #[cfg(not(target_os = "none"))]
    fn owns(&self, pointer: *mut u8) -> bool {
        let start = self.original.as_ptr() as usize;
        let Some(end) = start.checked_add(self.cursor.stats.budget as usize) else {
            return false;
        };
        let address = pointer as usize;
        address >= start && address < end
    }
}

enum AllocatorPhase {
    Inactive,
    Active(Arena),
    Sealed(Arena),
}

struct AllocatorState {
    phase: AllocatorPhase,
}

impl AllocatorState {
    const fn new() -> Self {
        Self {
            phase: AllocatorPhase::Inactive,
        }
    }

    fn activate(&mut self, arena: Arena) -> Result<(), ArenaError> {
        if !matches!(self.phase, AllocatorPhase::Inactive) {
            return Err(ArenaError::AlreadyActive);
        }
        self.phase = AllocatorPhase::Active(arena);
        Ok(())
    }

    fn allocate(&mut self, layout: Layout) -> Result<*mut u8, ArenaError> {
        match &mut self.phase {
            AllocatorPhase::Active(arena) => Ok(arena.allocate(layout)),
            AllocatorPhase::Sealed(_) => Err(ArenaError::Sealed),
            AllocatorPhase::Inactive => Err(ArenaError::Inactive),
        }
    }

    fn seal(&mut self) -> Result<AllocationStats, ArenaError> {
        let phase = core::mem::replace(&mut self.phase, AllocatorPhase::Inactive);
        match phase {
            AllocatorPhase::Active(arena) => {
                let stats = arena.stats();
                self.phase = AllocatorPhase::Sealed(arena);
                Ok(stats)
            }
            other => {
                self.phase = other;
                match self.phase {
                    AllocatorPhase::Sealed(_) => Err(ArenaError::Sealed),
                    AllocatorPhase::Inactive => Err(ArenaError::Inactive),
                    AllocatorPhase::Active(_) => unreachable!(),
                }
            }
        }
    }

    fn stats(&self) -> Option<AllocationStats> {
        match &self.phase {
            AllocatorPhase::Active(arena) | AllocatorPhase::Sealed(arena) => Some(arena.stats()),
            AllocatorPhase::Inactive => None,
        }
    }

    fn take(&mut self) -> Result<Arena, ArenaError> {
        let phase = core::mem::replace(&mut self.phase, AllocatorPhase::Inactive);
        match phase {
            AllocatorPhase::Active(arena) | AllocatorPhase::Sealed(arena) => Ok(arena),
            AllocatorPhase::Inactive => Err(ArenaError::Inactive),
        }
    }

    #[cfg(not(target_os = "none"))]
    fn arena_owns(&self, pointer: *mut u8) -> bool {
        match &self.phase {
            AllocatorPhase::Active(arena) | AllocatorPhase::Sealed(arena) => arena.owns(pointer),
            AllocatorPhase::Inactive => false,
        }
    }
}

struct GlobalState(UnsafeCell<AllocatorState>);

// SAFETY: The NTS-3 runtime invokes one unit instance through the same plain,
// mutable callback model as its C++ template. Initialization/allocation and
// teardown are serialized, and rendering occurs only after the allocator is
// sealed. Host probe use is explicitly single-threaded. No lock is introduced
// into the target artifact.
unsafe impl Sync for GlobalState {}

static STATE: GlobalState = GlobalState(UnsafeCell::new(AllocatorState::new()));

#[cfg(test)]
pub(crate) static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_state<R>(f: impl FnOnce(&mut AllocatorState) -> R) -> R {
    // SAFETY: access is serialized by the runtime lifecycle contract described
    // on `GlobalState`; no reference to the state escapes this call.
    unsafe { f(&mut *STATE.0.get()) }
}

/// Allocator installed by the framework on target and by the host probe fixture.
///
/// Target allocations fail before activation and after sealing. Host builds
/// delegate to `System` only while the probe is inactive, allowing an ordinary
/// executable to set up and report a bounded probe arena.
pub struct FrameworkAllocator;

unsafe impl GlobalAlloc for FrameworkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match with_state(|state| state.allocate(layout)) {
            #[cfg(target_os = "none")]
            Ok(pointer) if !pointer.is_null() => pointer,
            #[cfg(target_os = "none")]
            Ok(_) | Err(_) => allocation_fault(),
            #[cfg(not(target_os = "none"))]
            Ok(pointer) => pointer,
            #[cfg(not(target_os = "none"))]
            Err(ArenaError::Inactive) => {
                // SAFETY: this branch delegates the unchanged valid layout to
                // the process allocator while no probe arena is active.
                unsafe { std::alloc::System.alloc(layout) }
            }
            #[cfg(not(target_os = "none"))]
            Err(_) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        #[cfg(target_os = "none")]
        {
            let _ = (pointer, layout);
            // Intentional no-op: every target allocation has arena lifetime and
            // the complete arena is returned to Korg during teardown.
        }

        #[cfg(not(target_os = "none"))]
        {
            if with_state(|state| state.arena_owns(pointer)) {
                // Probe allocations have arena lifetime, matching target.
                return;
            }
            // SAFETY: pointers outside an active host arena were allocated by
            // `System` in the inactive delegation branch above.
            unsafe { std::alloc::System.dealloc(pointer, layout) };
        }
    }
}

#[cfg(target_os = "none")]
#[cold]
#[inline(never)]
fn allocation_fault() -> ! {
    // Stable Rust does not expose `#[alloc_error_handler]`. Fault directly in
    // the global allocator instead, before `alloc` can format or unwind an OOM.
    loop {
        core::hint::spin_loop();
    }
}

pub(crate) fn activate_target(hooks: UnitRuntimeHooks, budget: u32) -> Result<(), ArenaError> {
    if budget == 0 {
        return Err(ArenaError::InvalidBudget);
    }
    if budget > PLATFORM_SDRAM_LIMIT {
        return Err(ArenaError::BudgetTooLarge);
    }
    if with_state(|state| !matches!(state.phase, AllocatorPhase::Inactive)) {
        return Err(ArenaError::AlreadyActive);
    }

    let (Some(allocate), Some(free), Some(available)) =
        (hooks.sdram_alloc(), hooks.sdram_free(), hooks.sdram_avail())
    else {
        return Err(ArenaError::MissingHooks);
    };

    // SAFETY: these function pointers are copied from the validated Korg
    // runtime descriptor and take no Rust references.
    if unsafe { available() } < budget as usize {
        return Err(ArenaError::Unavailable);
    }
    // SAFETY: the validated Korg hook accepts one requested byte count. This is
    // the sole SDRAM allocation made for the runtime lifecycle.
    let pointer = unsafe { allocate(budget as usize) };
    let Some(original) = NonNull::new(pointer) else {
        return Err(ArenaError::AllocationFailed);
    };

    let arena = match Arena::new(original, budget, Some(free)) {
        Ok(arena) => arena,
        Err(error) => {
            // SAFETY: `original` is exactly the pointer returned by the matching
            // Korg allocation hook above and has not previously been freed.
            unsafe { free(original.as_ptr()) };
            return Err(error);
        }
    };
    if let Err(error) = with_state(|state| state.activate(arena)) {
        // A serialized lifecycle cannot reach this branch after the precheck.
        // If it does, recover the just-installed arena and release it below.
        let _ = reset_and_release();
        return Err(error);
    }
    Ok(())
}

pub(crate) fn seal() -> Result<AllocationStats, ArenaError> {
    with_state(AllocatorState::seal)
}

pub(crate) fn stats() -> Option<AllocationStats> {
    with_state(|state| state.stats())
}

pub(crate) fn reset_and_release() -> Result<(), ArenaError> {
    let arena = with_state(AllocatorState::take)?;
    if let Some(free) = arena.free {
        // SAFETY: the arena retains the untouched pointer from the one matching
        // Korg allocation call, and taking it made a second release impossible.
        unsafe { free(arena.original.as_ptr()) };
    }
    Ok(())
}

#[cfg(not(target_os = "none"))]
pub mod host {
    use super::*;
    use core::alloc::GlobalAlloc;

    const BACKING_ALIGNMENT: usize = 4096;

    /// Owns one system allocation used as an SDRAM-equivalent host probe arena.
    pub struct HostArena {
        pointer: NonNull<u8>,
        layout: Layout,
        active: bool,
    }

    impl HostArena {
        pub fn new(budget: u32) -> Result<Self, &'static str> {
            if budget == 0 || budget > PLATFORM_SDRAM_LIMIT {
                return Err("host arena budget must be within 1..=3 MiB");
            }
            let layout = Layout::from_size_align(budget as usize, BACKING_ALIGNMENT)
                .map_err(|_| "invalid host arena layout")?;
            // SAFETY: `layout` is nonzero and valid. Calling `System` directly
            // avoids routing backing storage through the probe allocator.
            let pointer = unsafe { std::alloc::System.alloc(layout) };
            let pointer = NonNull::new(pointer).ok_or("host arena allocation failed")?;
            Ok(Self {
                pointer,
                layout,
                active: false,
            })
        }

        /// Runs a single-threaded probe. Values allocating from the arena must
        /// be dropped (or intentionally forgotten) before the closure returns.
        pub fn run(&mut self, probe: impl FnOnce()) -> Result<AllocationStats, &'static str> {
            if self.active {
                return Err("host arena is already active");
            }
            let arena = Arena::new(self.pointer, self.layout.size() as u32, None)
                .map_err(|_| "host arena address does not fit the 32-bit model")?;
            with_state(|state| state.activate(arena))
                .map_err(|_| "another framework arena is active")?;
            self.active = true;

            probe();

            let result = seal().map_err(|_| "failed to seal host arena");
            let reset = with_state(AllocatorState::take);
            self.active = false;
            reset.map_err(|_| "failed to reset host arena")?;
            result
        }
    }

    impl Drop for HostArena {
        fn drop(&mut self) {
            if self.active {
                let _ = with_state(AllocatorState::take);
                self.active = false;
            }
            // SAFETY: this is the same pointer/layout pair obtained directly
            // from `System` in `new`, after the framework has stopped using it.
            unsafe { std::alloc::System.dealloc(self.pointer.as_ptr(), self.layout) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fit_and_one_byte_overflow_are_deterministic() {
        let mut cursor = ArenaCursor::new(0x2000_0000, 64).unwrap();
        assert_eq!(cursor.reserve(64, 1), Ok(0));
        let before = cursor.stats;
        assert_eq!(cursor.reserve(1, 1), Err(ArenaError::AllocationFailed));
        assert_eq!(cursor.stats, before);
        assert_eq!(cursor.stats.high_water, 64);
    }

    #[test]
    fn alignment_padding_and_high_water_are_exact() {
        let mut cursor = ArenaCursor::new(0x2000_0003, 128).unwrap();
        assert_eq!(cursor.reserve(1, 1), Ok(0));
        assert_eq!(cursor.reserve(4, 4), Ok(1));
        assert_eq!(cursor.reserve(8, 16), Ok(13));
        assert_eq!(cursor.stats.requested_bytes, 13);
        assert_eq!(cursor.stats.alignment_padding, 8);
        assert_eq!(cursor.stats.position, 21);
        assert_eq!(cursor.stats.high_water, 21);
        assert_eq!(cursor.stats.allocations, 3);
    }

    #[test]
    fn checked_32_bit_address_math_rejects_wraparound() {
        assert_eq!(
            ArenaCursor::new(u32::MAX - 7, 8).unwrap_err(),
            ArenaError::AddressOverflow
        );
        let mut cursor = ArenaCursor::new(u32::MAX - 15, 15).unwrap();
        assert_eq!(cursor.reserve(1, 16), Ok(0));
        assert_eq!(cursor.reserve(1, 16), Err(ArenaError::AddressOverflow));
    }

    #[test]
    fn every_power_of_two_alignment_through_one_mebibyte_is_supported() {
        for exponent in 0..=20 {
            let align = 1_u32 << exponent;
            let mut cursor = ArenaCursor::new(0x1000_0003, PLATFORM_SDRAM_LIMIT).unwrap();
            let start = cursor.reserve(1, align).unwrap();
            assert_eq!((cursor.base + start) & (align - 1), 0);
            assert_eq!(cursor.stats.requested_bytes, 1);
            assert_eq!(cursor.stats.position, cursor.stats.alignment_padding + 1);
        }
    }

    #[test]
    fn randomized_layouts_are_aligned_non_overlapping_and_exact() {
        let mut seed = 0x8f31_72a5_u32;
        for case in 0..512_u32 {
            let base = 0x1000_0001 + (case & 0xff);
            let mut cursor = ArenaCursor::new(base, 65_536).unwrap();
            let mut previous_end = 0_u32;
            let mut expected_requested = 0_u32;
            let mut expected_padding = 0_u32;

            for _ in 0..64 {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let size = seed % 127 + 1;
                let align = 1_u32 << ((seed >> 16) % 13); // 1 through 4096.
                let start = cursor.reserve(size, align).unwrap();
                assert!(start >= previous_end);
                assert_eq!((base + start) & (align - 1), 0);
                expected_padding += start - previous_end;
                expected_requested += size;
                previous_end = start + size;
            }

            assert_eq!(cursor.stats.requested_bytes, expected_requested);
            assert_eq!(cursor.stats.alignment_padding, expected_padding);
            assert_eq!(cursor.stats.position, previous_end);
            assert_eq!(cursor.stats.high_water, previous_end);
        }
    }

    #[test]
    fn local_state_rejects_before_activation_and_after_sealing() {
        let mut backing = [0_u8; 128];
        let pointer = NonNull::new(backing.as_mut_ptr()).unwrap();
        let mut state = AllocatorState::new();
        let layout = Layout::from_size_align(8, 8).unwrap();
        assert_eq!(state.allocate(layout), Err(ArenaError::Inactive));
        state
            .activate(Arena::new(pointer, 128, None).unwrap())
            .unwrap();
        assert!(!state.allocate(layout).unwrap().is_null());
        let stats = state.seal().unwrap();
        assert_eq!(stats.requested_bytes, 8);
        assert_eq!(state.allocate(layout), Err(ArenaError::Sealed));
        state.take().unwrap();
        assert_eq!(state.stats(), None);
    }

    #[test]
    fn deallocation_is_a_no_op_until_whole_arena_reset() {
        let mut backing = [0_u8; 128];
        let mut arena = Arena::new(NonNull::new(backing.as_mut_ptr()).unwrap(), 128, None).unwrap();
        let first = arena.allocate(Layout::from_size_align(16, 8).unwrap());
        let before = arena.stats();
        // There is deliberately no per-allocation deallocation operation.
        let second = arena.allocate(Layout::from_size_align(16, 8).unwrap());
        assert_ne!(first, second);
        assert_eq!(arena.stats().position, before.position + 16);
    }
}

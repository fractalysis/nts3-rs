use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use nts3::prelude::*;
use nts3::runtime::RuntimeController;
use nts3_sys::{
    UNIT_API_2_0_0, UNIT_ERR_NONE, UNIT_TARGET_NTS3_KAOSS_GENERICFX, UNIT_TOUCH_PHASE_BEGAN,
    UnitRuntimeDescriptor, UnitRuntimeGenericfxContext, UnitRuntimeHooks,
};

struct CountingAllocator;

static TRACKING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the unchanged valid layout is delegated to the process
        // allocator, and its result is returned directly.
        unsafe { std::alloc::System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: this is the same pointer/layout contract received by this
        // allocator from a prior delegated System allocation.
        unsafe { std::alloc::System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[repr(align(4096))]
struct Arena([u8; 4096]);

static mut ARENA: Arena = Arena([0; 4096]);

unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
    if size <= 4096 {
        // SAFETY: this test owns one serialized lifecycle and `addr_of_mut!`
        // creates no reference to the mutable static.
        unsafe { core::ptr::addr_of_mut!(ARENA.0).cast::<u8>() }
    } else {
        ptr::null_mut()
    }
}

unsafe extern "C" fn free(_pointer: *const u8) {}

unsafe extern "C" fn available() -> usize {
    4096
}

#[derive(Default)]
struct Parameters;
impl nts3::__private::Sealed for Parameters {}
impl Nts3Parameters for Parameters {}

#[derive(Default)]
struct Probe;

impl Nts3Plugin for Probe {
    type Parameters = Parameters;

    fn process(&mut self, _parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        for mut frame in buffer.frames_mut() {
            let input = frame.input();
            frame.write(input);
        }
    }
}

fn assert_no_alloc<R>(operation: impl FnOnce() -> R) -> R {
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);
    let result = operation();
    TRACKING.store(false, Ordering::SeqCst);
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), before);
    result
}

#[test]
fn every_post_init_runtime_path_allocates_zero_bytes() {
    let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
    let descriptor = UnitRuntimeDescriptor::new(
        UNIT_TARGET_NTS3_KAOSS_GENERICFX,
        UNIT_API_2_0_0,
        48_000,
        8,
        2,
        2,
        UnitRuntimeHooks::new(
            (&context as *const UnitRuntimeGenericfxContext).cast(),
            Some(allocate),
            Some(free),
            Some(available),
        ),
    );
    let mut controller = RuntimeController::<Probe>::new();
    // SAFETY: descriptor and context remain readable throughout initialization.
    assert_eq!(
        unsafe { controller.initialize(&descriptor, 4096) },
        UNIT_ERR_NONE
    );

    let input = [0.25, -0.5];
    let mut output = [0.0; 2];
    assert_no_alloc(|| {
        // SAFETY: one separate stereo input/output frame is live for the call.
        unsafe {
            controller
                .render(input.as_ptr(), output.as_mut_ptr(), 1)
                .unwrap()
        }
    });
    assert_eq!(output, input);
    assert!(assert_no_alloc(|| controller.reset()));
    assert!(assert_no_alloc(|| controller.touch_event(
        0,
        UNIT_TOUCH_PHASE_BEGAN,
        0,
        0
    )));
    assert!(assert_no_alloc(
        || controller.set_tempo((120 << 16) | 0x8000)
    ));
    assert!(assert_no_alloc(|| controller.tempo_4ppqn_tick(42)));
    assert!(assert_no_alloc(|| controller.suspend()));
    assert!(assert_no_alloc(|| controller.resume()));
    assert!(assert_no_alloc(|| controller.teardown()));
}

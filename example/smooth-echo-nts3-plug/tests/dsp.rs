use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use nts3_sys::{
    GENERICFX_CURVE_EXP, GENERICFX_CURVE_LINEAR, GENERICFX_PARAM_ASSIGN_DEPTH,
    GENERICFX_PARAM_ASSIGN_X, GENERICFX_PARAM_ASSIGN_Y, UNIT_API_2_0_0, UNIT_ERR_NONE,
    UNIT_PARAM_TYPE_DRYWET, UNIT_TARGET_NTS3_KAOSS_GENERICFX, UnitRuntimeDescriptor,
    UnitRuntimeGenericfxContext, UnitRuntimeHooks,
};
use smooth_echo::{
    unit_get_param_value, unit_header, unit_init, unit_render, unit_reset, unit_set_param_value,
    unit_teardown,
};

struct CountingAllocator;

// 0 = untracked, 1 = initialization, 2 = post-initialization callback.
static ALLOCATION_PHASE: AtomicUsize = AtomicUsize::new(0);
static INIT_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static INIT_BYTES: AtomicUsize = AtomicUsize::new(0);
static POST_INIT_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match ALLOCATION_PHASE.load(Ordering::Relaxed) {
            1 => {
                INIT_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
                INIT_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            }
            2 => {
                POST_INIT_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        // SAFETY: the valid layout is delegated unchanged to the process
        // allocator and its pointer is returned directly.
        unsafe { std::alloc::System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: this is the pointer/layout pair from a prior System allocation.
        unsafe { std::alloc::System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ARENA_BYTES: usize = 1_100_000;
const MAX_FRAMES: usize = 256;
const SAMPLE_RATE: usize = 48_000;

#[repr(align(4096))]
struct AlignedArena([u8; ARENA_BYTES]);

static mut ARENA: AlignedArena = AlignedArena([0; ARENA_BYTES]);
static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn sdram_alloc(size: usize) -> *mut u8 {
    ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    if size <= ARENA_BYTES {
        // SAFETY: this test runs one serialized unit lifecycle at a time. The
        // runtime releases the whole arena before the next lifecycle starts.
        unsafe { ptr::addr_of_mut!(ARENA.0).cast::<u8>() }
    } else {
        ptr::null_mut()
    }
}

unsafe extern "C" fn sdram_free(pointer: *const u8) {
    // SAFETY: taking the static's address does not read or borrow its contents.
    let expected = unsafe { ptr::addr_of!(ARENA.0).cast::<u8>() };
    assert_eq!(pointer, expected);
    FREE_CALLS.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn sdram_avail() -> usize {
    ARENA_BYTES
}

fn initialize() {
    INIT_ALLOCATIONS.store(0, Ordering::SeqCst);
    INIT_BYTES.store(0, Ordering::SeqCst);
    POST_INIT_ALLOCATIONS.store(0, Ordering::SeqCst);
    let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
    let descriptor = UnitRuntimeDescriptor::new(
        UNIT_TARGET_NTS3_KAOSS_GENERICFX,
        UNIT_API_2_0_0,
        48_000,
        MAX_FRAMES as u16,
        2,
        2,
        UnitRuntimeHooks::new(
            (&context as *const UnitRuntimeGenericfxContext).cast(),
            Some(sdram_alloc),
            Some(sdram_free),
            Some(sdram_avail),
        ),
    );
    ALLOCATION_PHASE.store(1, Ordering::SeqCst);
    // SAFETY: the descriptor and genericfx context are readable for the whole
    // call, and all copied hooks remain valid for the test lifecycle.
    let result = unsafe { unit_init(&descriptor) };
    ALLOCATION_PHASE.store(2, Ordering::SeqCst);
    assert_eq!(result, UNIT_ERR_NONE);
    assert_eq!(INIT_ALLOCATIONS.load(Ordering::SeqCst), 2);
    assert_eq!(INIT_BYTES.load(Ordering::SeqCst), 1_048_576);
}

fn teardown() {
    unit_teardown();
    ALLOCATION_PHASE.store(0, Ordering::SeqCst);
    assert_eq!(POST_INIT_ALLOCATIONS.load(Ordering::SeqCst), 0);
}

fn render_separate(input: &[f32], output: &mut [f32]) {
    assert_eq!(input.len(), output.len());
    assert_eq!(input.len() % 2, 0);
    for (input, output) in input
        .chunks(MAX_FRAMES * 2)
        .zip(output.chunks_mut(MAX_FRAMES * 2))
    {
        // SAFETY: each pair contains the same number of complete, disjoint
        // stereo frames and no chunk exceeds the initialized frame maximum.
        unsafe {
            unit_render(
                input.as_ptr(),
                output.as_mut_ptr(),
                (input.len() / 2) as u32,
            )
        };
    }
}

fn render_in_place(samples: &mut [f32]) {
    assert_eq!(samples.len() % 2, 0);
    for chunk in samples.chunks_mut(MAX_FRAMES * 2) {
        let pointer = chunk.as_mut_ptr();
        // SAFETY: the pointer covers complete interleaved stereo frames and is
        // passed as the exact in-place input/output form allowed by the SDK.
        unsafe { unit_render(pointer.cast_const(), pointer, (chunk.len() / 2) as u32) };
    }
}

fn maximum_magnitude(samples: &[f32]) -> f32 {
    samples.iter().copied().map(f32::abs).fold(0.0, f32::max)
}

fn bytes_of<T>(value: &T) -> &[u8] {
    // SAFETY: this immutable byte view covers exactly one fully initialized
    // packed SDK value and cannot outlive the shared borrow.
    unsafe { core::slice::from_raw_parts(ptr::from_ref(value).cast::<u8>(), size_of::<T>()) }
}

#[test]
fn smooth_echo_dsp_metadata_memory_and_allocation_contract() {
    let common = unit_header.common();
    let descriptors = common.params();
    let mappings = unit_header.default_mappings();
    assert_eq!(common.num_params(), 3);
    assert_eq!(descriptors[0].min(), 1);
    assert_eq!(descriptors[0].max(), 2_000);
    assert_eq!(descriptors[0].init(), 500);
    assert_eq!(descriptors[1].min(), 0);
    assert_eq!(descriptors[1].max(), 1_000);
    assert_eq!(descriptors[1].init(), 0);
    assert_eq!(descriptors[2].min(), -1_000);
    assert_eq!(descriptors[2].max(), 1_000);
    assert_eq!(descriptors[2].center(), 0);
    assert_eq!(descriptors[2].init(), 1_000);
    assert_eq!(descriptors[2].parameter_type(), UNIT_PARAM_TYPE_DRYWET);
    assert_eq!(mappings[0].assign(), GENERICFX_PARAM_ASSIGN_X);
    assert_eq!(mappings[0].curve().curve(), GENERICFX_CURVE_EXP);
    assert_eq!(
        (mappings[0].min(), mappings[0].max(), mappings[0].value()),
        (1, 2_000, 500)
    );
    assert_eq!(mappings[1].assign(), GENERICFX_PARAM_ASSIGN_Y);
    assert_eq!(mappings[1].curve().curve(), GENERICFX_CURVE_LINEAR);
    assert_eq!(
        (mappings[1].min(), mappings[1].max(), mappings[1].value()),
        (0, 1_000, 0)
    );
    assert_eq!(mappings[2].assign(), GENERICFX_PARAM_ASSIGN_DEPTH);
    assert_eq!(mappings[2].curve().curve(), GENERICFX_CURVE_EXP);
    assert_eq!(mappings[2].curve().polarity(), 1);
    assert_eq!(
        (mappings[2].min(), mappings[2].max(), mappings[2].value()),
        (-1_000, 1_000, 1_000)
    );

    let mut expected_descriptors = [0_u8; 8 * 32];
    expected_descriptors[0..2].copy_from_slice(&1_i16.to_le_bytes());
    expected_descriptors[2..4].copy_from_slice(&2_000_i16.to_le_bytes());
    expected_descriptors[4..6].copy_from_slice(&1_i16.to_le_bytes());
    expected_descriptors[6..8].copy_from_slice(&500_i16.to_le_bytes());
    expected_descriptors[8] = 9; // milliseconds
    expected_descriptors[10..14].copy_from_slice(b"TIME");
    expected_descriptors[34..36].copy_from_slice(&1_000_i16.to_le_bytes());
    expected_descriptors[40] = 1; // percent
    expected_descriptors[41] = 0x11; // one decimal place
    expected_descriptors[42..50].copy_from_slice(b"FEEDBACK");
    expected_descriptors[64..66].copy_from_slice(&(-1_000_i16).to_le_bytes());
    expected_descriptors[66..68].copy_from_slice(&1_000_i16.to_le_bytes());
    expected_descriptors[70..72].copy_from_slice(&1_000_i16.to_le_bytes());
    expected_descriptors[72] = UNIT_PARAM_TYPE_DRYWET;
    expected_descriptors[73] = 0x11; // one decimal place
    expected_descriptors[74..79].copy_from_slice(b"DEPTH");
    assert_eq!(bytes_of(&descriptors), expected_descriptors);

    let mut expected_mappings = [0_u8; 8 * 8];
    expected_mappings[0] = GENERICFX_PARAM_ASSIGN_X;
    expected_mappings[1] = GENERICFX_CURVE_EXP;
    expected_mappings[2..4].copy_from_slice(&1_i16.to_le_bytes());
    expected_mappings[4..6].copy_from_slice(&2_000_i16.to_le_bytes());
    expected_mappings[6..8].copy_from_slice(&500_i16.to_le_bytes());
    expected_mappings[8] = GENERICFX_PARAM_ASSIGN_Y;
    expected_mappings[9] = GENERICFX_CURVE_LINEAR;
    expected_mappings[12..14].copy_from_slice(&1_000_i16.to_le_bytes());
    expected_mappings[16] = GENERICFX_PARAM_ASSIGN_DEPTH;
    expected_mappings[17] = 0x80 | GENERICFX_CURVE_EXP;
    expected_mappings[18..20].copy_from_slice(&(-1_000_i16).to_le_bytes());
    expected_mappings[20..22].copy_from_slice(&1_000_i16.to_le_bytes());
    expected_mappings[22..24].copy_from_slice(&1_000_i16.to_le_bytes());
    assert_eq!(bytes_of(&mappings), expected_mappings);

    // Silence remains silent and finite at the declared defaults.
    let silence = vec![0.0_f32; MAX_FRAMES * 2];
    let mut silence_out = vec![f32::NAN; silence.len()];
    initialize();
    assert_eq!(unit_get_param_value(0), 500);
    assert_eq!(unit_get_param_value(1), 0);
    assert_eq!(unit_get_param_value(2), 1_000);
    render_separate(&silence, &mut silence_out);
    teardown();
    assert!(silence_out.iter().all(|sample| sample.is_finite()));
    assert!(maximum_magnitude(&silence_out) <= 1.0e-7);

    // The bottom of the FX DEPTH slider is fully dry. DSP still runs, but the
    // final mix must pass the original stereo input through unchanged.
    let dry_input = [0.75_f32, -0.25, -0.5, 0.125];
    let mut dry_output = [0.0_f32; 4];
    initialize();
    unit_set_param_value(2, -1_000);
    unit_reset();
    render_separate(&dry_input, &mut dry_output);
    teardown();
    assert_eq!(dry_output, dry_input);

    // At 100 ms and 50% feedback, the first repeat begins at 4,800 samples.
    let impulse_frames = 6_000;
    let mut impulse = vec![0.0_f32; impulse_frames * 2];
    let mut impulse_out = vec![0.0_f32; impulse.len()];
    impulse[0] = 1.0;
    impulse[1] = -1.0;
    initialize();
    unit_set_param_value(0, 100);
    unit_set_param_value(1, 500);
    unit_reset();
    render_separate(&impulse, &mut impulse_out);
    teardown();
    let repeat = (100..impulse_frames)
        .find(|&frame| impulse_out[frame * 2].abs() > 0.05)
        .expect("delayed impulse repeat");
    assert!(
        repeat.abs_diff(SAMPLE_RATE / 10) <= 2,
        "repeat at sample {repeat}"
    );
    assert!(impulse_out.iter().all(|sample| sample.is_finite()));

    // The strict >0.99 freeze branch ignores new input, while 0.99 remains in
    // the ordinary feedback branch. Both paths remain finite and bounded.
    let boundary_input = [1.0_f32, -1.0];
    let mut at_boundary = [0.0_f32; 2];
    let mut frozen = [0.0_f32; 2];
    initialize();
    unit_set_param_value(1, 990);
    unit_reset();
    render_separate(&boundary_input, &mut at_boundary);
    unit_set_param_value(1, 991);
    unit_reset();
    render_separate(&boundary_input, &mut frozen);
    teardown();
    assert!(maximum_magnitude(&at_boundary) > 0.01);
    assert!(maximum_magnitude(&at_boundary) < 2.0);
    assert!(maximum_magnitude(&frozen) <= 1.0e-7);

    // Reset clears both delay/filter state but retains parameter targets.
    let excite_frames = 2_048;
    let mut excite = vec![0.0_f32; excite_frames * 2];
    let mut excited = vec![0.0_f32; excite.len()];
    let reset_silence = vec![0.0_f32; excite.len()];
    let mut after_reset = vec![1.0_f32; excite.len()];
    excite[0] = 1.0;
    excite[1] = 1.0;
    initialize();
    unit_set_param_value(0, 1);
    unit_set_param_value(1, 700);
    unit_reset();
    render_separate(&excite, &mut excited);
    unit_reset();
    assert_eq!(unit_get_param_value(0), 1);
    assert_eq!(unit_get_param_value(1), 700);
    render_separate(&reset_silence, &mut after_reset);
    teardown();
    assert!(maximum_magnitude(&excited) > 0.1);
    assert!(maximum_magnitude(&after_reset) <= 1.0e-7);

    // Fresh separate and exact in-place lifecycles must be equivalent.
    let equivalence_frames = 8_192;
    let source = (0..equivalence_frames * 2)
        .map(|index| {
            let phase = index as f32 * 0.017;
            phase.sin() * 0.75
        })
        .collect::<Vec<_>>();
    let mut separate = vec![0.0_f32; source.len()];
    let mut in_place = source.clone();
    initialize();
    unit_set_param_value(0, 37);
    unit_set_param_value(1, 800);
    unit_reset();
    render_separate(&source, &mut separate);
    teardown();
    initialize();
    unit_set_param_value(0, 37);
    unit_set_param_value(1, 800);
    unit_reset();
    render_in_place(&mut in_place);
    teardown();
    for (frame, (separate, in_place)) in separate.iter().zip(&in_place).enumerate() {
        assert!((separate - in_place).abs() <= 1.0e-6, "sample {frame}");
    }

    // Long deterministic full-range parameter sweeps and random stereo input
    // exercise smoothing, ordinary feedback, the 0.99 boundary, and freeze.
    let mut random_input = [0.0_f32; 128];
    let mut random_output = [0.0_f32; 128];
    let mut seed = 0x6d2b_79f5_u32;
    let mut peak = 0.0_f32;
    initialize();
    for block in 0..2_000_i32 {
        unit_set_param_value(0, 1 + (block * 37) % 2_000);
        let feedback = match block % 11 {
            0 => 0,
            1 => 990,
            2 => 991,
            3 => 1_000,
            _ => (block * 83) % 1_001,
        };
        unit_set_param_value(1, feedback);
        for sample in &mut random_input {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *sample = (seed as i32 as f32) / i32::MAX as f32;
        }
        render_separate(&random_input, &mut random_output);
        for &sample in &random_output {
            assert!(sample.is_finite(), "non-finite sample in block {block}");
            peak = peak.max(sample.abs());
        }
    }
    teardown();
    assert!(peak < 16.0, "unexpected long-run peak {peak}");

    assert_eq!(ALLOC_CALLS.load(Ordering::SeqCst), 8);
    assert_eq!(FREE_CALLS.load(Ordering::SeqCst), 8);
}

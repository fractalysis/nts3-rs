use core::ffi::c_char;
use core::marker::PhantomData;
use core::ptr;

use nts3_sys::{
    UNIT_ERR_API_VERSION, UNIT_ERR_GEOMETRY, UNIT_ERR_MEMORY, UNIT_ERR_NONE, UNIT_ERR_SAMPLERATE,
    UNIT_ERR_TARGET, UNIT_ERR_UNDEF, UNIT_TARGET_NTS3_KAOSS_GENERICFX, UnitRuntimeDescriptor,
    UnitRuntimeGenericfxContext, UnitRuntimeGenericfxGetRawInputFn, UnitRuntimeHooks,
    unit_api_is_compatible,
};

use crate::allocator::AllocationStats;
use crate::buffer::{BufferError, StereoBuffer};
use crate::parameter::Nts3Parameters;
use crate::plugin::{InitContext, InitError, Nts3Plugin};
use crate::runtime_state::{RuntimeInitError, RuntimeState};
use crate::touch::{TouchEvent, TouchPhase};

const SAMPLE_RATE_HZ: u32 = 48_000;
const STEREO_CHANNELS: u8 = 2;

#[derive(Clone, Copy)]
struct RuntimeContext {
    sample_rate_hz: u32,
    maximum_frames: usize,
    touch_area: [u32; 2],
    get_raw_input: Option<UnitRuntimeGenericfxGetRawInputFn>,
}

impl RuntimeContext {
    fn init_context(&self) -> InitContext<'_> {
        InitContext {
            sample_rate_hz: self.sample_rate_hz,
            maximum_frames: self.maximum_frames,
            touch_area: self.touch_area,
            _lifetime: PhantomData,
        }
    }
}

/// Concrete plugin, parameter, and copied-context ownership for one runtime.
pub struct Runtime<P: Nts3Plugin> {
    plugin: P,
    parameters: P::Parameters,
    context: RuntimeContext,
}

impl<P: Nts3Plugin> Runtime<P> {
    fn construct(context: RuntimeContext) -> Result<Self, InitError> {
        // Keep this order stable: generated parameter defaults precede plugin
        // construction, matching the framework initialization contract.
        let mut parameters = P::Parameters::default();
        parameters.initialize_smoothers(context.sample_rate_hz as f32);
        parameters.reset_smoothers();
        let plugin = P::default();
        let mut runtime = Self {
            plugin,
            parameters,
            context,
        };
        let init_context = runtime.context.init_context();
        runtime.plugin.initialize(&init_context)?;
        Ok(runtime)
    }

    fn reset(&mut self) {
        // SDK reset retains parameter targets but discards an in-flight ramp.
        self.parameters.reset_smoothers();
        self.plugin.reset();
    }

    fn resume(&mut self) {
        self.plugin.resume();
    }

    fn suspend(&mut self) {
        self.plugin.suspend();
    }

    fn teardown(&mut self) {
        self.plugin.teardown();
    }

    unsafe fn render(
        &mut self,
        input: *const f32,
        output: *mut f32,
        frames: u32,
    ) -> Result<(), RenderError> {
        let frames = usize::try_from(frames).map_err(|_| RenderError::FrameCountExceeded)?;
        if frames > self.context.maximum_frames {
            return Err(RenderError::FrameCountExceeded);
        }

        // The hook is intentionally called for every valid render. Only its
        // result, scoped into the local buffer, is retained.
        let raw_input = match self.context.get_raw_input {
            Some(get_raw_input) => {
                // SAFETY: the function pointer came from the validated genericfx
                // runtime context and is invoked only during this render call.
                unsafe { get_raw_input() }
            }
            None => ptr::null(),
        };

        // SAFETY: the caller upholds the SDK render pointer contract. The local
        // buffer cannot escape `process`, and validates shape/overlap before use.
        let mut buffer = unsafe { StereoBuffer::from_raw(input, output, raw_input, frames) }
            .map_err(RenderError::Buffer)?;
        self.parameters.begin_block();
        self.plugin.process(&mut self.parameters, &mut buffer);
        self.parameters.end_block();
        Ok(())
    }

    fn touch_event(&mut self, id: u8, phase: TouchPhase, x: u32, y: u32) {
        self.plugin
            .touch_event(TouchEvent::new(id, phase, [x, y], self.context.touch_area));
    }
}

/// Non-panicking render rejection at the raw callback boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    Inactive,
    FrameCountExceeded,
    Buffer(BufferError),
}

/// Callback-independent lifecycle adapter used by generated ABI shims.
///
/// One controller owns at most one initialized `Runtime<P>`. Generated exports
/// place it in their process-global singleton and forward callbacks here.
#[doc(hidden)]
pub struct RuntimeController<P: Nts3Plugin> {
    state: RuntimeState<Runtime<P>>,
}

impl<P: Nts3Plugin> RuntimeController<P> {
    pub const fn new() -> Self {
        Self {
            state: RuntimeState::new(),
        }
    }

    /// Validates and copies an SDK descriptor, allocates SDRAM, and constructs
    /// plugin state in the required order.
    ///
    /// # Safety
    /// A non-null `descriptor` must point to a readable SDK descriptor for this
    /// call. Its runtime-context pointer must follow the SDK genericfx contract.
    pub unsafe fn initialize(
        &mut self,
        descriptor: *const UnitRuntimeDescriptor,
        sdram_bytes: u32,
    ) -> i8 {
        let validated = match unsafe { validate_descriptor(descriptor) } {
            Ok(validated) => validated,
            Err(code) => return code,
        };

        let mut plugin_error = None;
        let result =
            self.state.initialize(
                validated.hooks,
                sdram_bytes,
                || match Runtime::<P>::construct(validated.context) {
                    Ok(runtime) => Ok(runtime),
                    Err(error) => {
                        plugin_error = Some(error);
                        Err(RuntimeInitError::ConstructionFailed)
                    }
                },
            );

        match (result, plugin_error) {
            (Ok(()), None) => UNIT_ERR_NONE,
            (_, Some(error)) => error.sdk_code(),
            (Err(error), None) => runtime_error_code(error),
        }
    }

    pub fn allocation_stats(&self) -> Option<AllocationStats> {
        self.state.allocation_stats()
    }

    pub fn reset(&mut self) -> bool {
        let Some(runtime) = self.state.active_mut() else {
            return false;
        };
        runtime.reset();
        true
    }

    pub fn suspend(&mut self) -> bool {
        let Some(runtime) = self.state.ready_mut() else {
            return false;
        };
        runtime.suspend();
        self.state.suspend().is_ok()
    }

    pub fn resume(&mut self) -> bool {
        if self.state.resume().is_err() {
            return false;
        }
        let Some(runtime) = self.state.ready_mut() else {
            return false;
        };
        runtime.resume();
        true
    }

    pub fn teardown(&mut self) -> bool {
        self.state.teardown(|runtime| runtime.teardown()).is_ok()
    }

    /// Processes one SDK render call without retaining any audio pointer.
    ///
    /// # Safety
    /// For nonzero `frames`, SDK input/output pointers must each cover that many
    /// interleaved stereo frames and be disjoint or exactly equal.
    pub unsafe fn render(
        &mut self,
        input: *const f32,
        output: *mut f32,
        frames: u32,
    ) -> Result<(), RenderError> {
        let runtime = self.state.ready_mut().ok_or(RenderError::Inactive)?;
        // SAFETY: this method forwards its documented pointer requirements and
        // `Runtime::render` keeps all pointer-derived state call-local.
        unsafe { runtime.render(input, output, frames) }
    }

    /// Returns a parameter's raw target, or zero for inactive/unknown IDs.
    pub fn get_parameter(&mut self, index: u8) -> i32 {
        self.state
            .active_mut()
            .and_then(|runtime| runtime.parameters.get(index))
            .unwrap_or(0)
    }

    /// Clamps and dispatches a parameter update. Unknown IDs are ignored.
    pub fn set_parameter(&mut self, index: u8, value: i32) -> bool {
        let Some(runtime) = self.state.active_mut() else {
            return false;
        };
        runtime.parameters.set(index, value)
    }

    /// Returns a static custom display string, or null for inactive/unknown IDs.
    pub fn parameter_string_value(&mut self, index: u8, value: i32) -> *const c_char {
        self.state
            .active_mut()
            .and_then(|runtime| runtime.parameters.string_value(index, value))
            .map_or(ptr::null(), |value| value.as_ptr())
    }

    pub fn touch_event(&mut self, id: u8, raw_phase: u8, x: u32, y: u32) -> bool {
        let Some(phase) = TouchPhase::from_raw(raw_phase) else {
            return false;
        };
        let Some(runtime) = self.state.active_mut() else {
            return false;
        };
        runtime.touch_event(id, phase, x, y);
        true
    }

    pub fn set_tempo(&mut self, tempo_uq16_16: u32) -> bool {
        let Some(runtime) = self.state.active_mut() else {
            return false;
        };
        runtime.plugin.tempo_changed(uq16_16_to_f32(tempo_uq16_16));
        true
    }

    pub fn tempo_4ppqn_tick(&mut self, counter: u32) -> bool {
        let Some(runtime) = self.state.active_mut() else {
            return false;
        };
        runtime.plugin.tempo_4ppqn_tick(counter);
        true
    }
}

impl<P: Nts3Plugin> Default for RuntimeController<P> {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn uq16_16_to_f32(value: u32) -> f32 {
    (value >> 16) as f32 + (value & 0xffff) as f32 / 65_536.0
}

struct ValidatedDescriptor {
    hooks: UnitRuntimeHooks,
    context: RuntimeContext,
}

unsafe fn validate_descriptor(
    descriptor: *const UnitRuntimeDescriptor,
) -> Result<ValidatedDescriptor, i8> {
    if descriptor.is_null() {
        return Err(UNIT_ERR_UNDEF);
    }

    // SAFETY: the caller promises a readable SDK descriptor. Its C structure is
    // packed, so an explicit unaligned by-value copy avoids field references.
    let descriptor = unsafe { descriptor.read_unaligned() };
    if descriptor.target() != UNIT_TARGET_NTS3_KAOSS_GENERICFX {
        return Err(UNIT_ERR_TARGET);
    }
    if !unit_api_is_compatible(descriptor.api()) {
        return Err(UNIT_ERR_API_VERSION);
    }
    if descriptor.sample_rate() != SAMPLE_RATE_HZ {
        return Err(UNIT_ERR_SAMPLERATE);
    }
    if descriptor.frames_per_buffer() == 0
        || descriptor.input_channels() != STEREO_CHANNELS
        || descriptor.output_channels() != STEREO_CHANNELS
    {
        return Err(UNIT_ERR_GEOMETRY);
    }

    let hooks = descriptor.hooks();
    if hooks.sdram_alloc().is_none()
        || hooks.sdram_free().is_none()
        || hooks.sdram_avail().is_none()
    {
        return Err(UNIT_ERR_MEMORY);
    }
    if hooks.runtime_context().is_null() {
        return Err(UNIT_ERR_UNDEF);
    }

    // SAFETY: the validated non-null context pointer is supplied by an NTS-3
    // genericfx runtime for the descriptor's lifetime. Copying unaligned avoids
    // making assumptions beyond the ABI's readable bytes.
    let genericfx = unsafe {
        hooks
            .runtime_context()
            .cast::<UnitRuntimeGenericfxContext>()
            .read_unaligned()
    };

    Ok(ValidatedDescriptor {
        hooks,
        context: RuntimeContext {
            sample_rate_hz: descriptor.sample_rate(),
            maximum_frames: descriptor.frames_per_buffer() as usize,
            touch_area: [genericfx.touch_area_width(), genericfx.touch_area_height()],
            get_raw_input: genericfx.get_raw_input(),
        },
    })
}

const fn runtime_error_code(error: RuntimeInitError) -> i8 {
    match error {
        RuntimeInitError::MissingMemoryHooks
        | RuntimeInitError::InvalidMemoryBudget
        | RuntimeInitError::MemoryUnavailable
        | RuntimeInitError::MemoryAllocationFailed
        | RuntimeInitError::AddressOverflow => UNIT_ERR_MEMORY,
        RuntimeInitError::InvalidState | RuntimeInitError::ConstructionFailed => UNIT_ERR_UNDEF,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use nts3_sys::{
        GENERICFX_PARAM_ASSIGN_X, GENERICFX_PARAM_ASSIGN_Y, GenericfxCurve, GenericfxParamMapping,
        UNIT_API_2_0_0, UNIT_MAX_PARAM_COUNT, UNIT_PARAM_NAME_SIZE, UNIT_PARAM_TYPE_MSEC,
        UNIT_PARAM_TYPE_PERCENT, UNIT_TOUCH_PHASE_BEGAN, UNIT_TOUCH_PHASE_CANCELLED,
        UNIT_TOUCH_PHASE_ENDED, UNIT_TOUCH_PHASE_MOVED, UNIT_TOUCH_PHASE_STATIONARY,
        UNUSED_MAPPING, UNUSED_PARAM, UnitParam, UnitParamFormat,
    };

    use crate::__private::Sealed;
    use crate::allocator::TEST_SERIAL;
    use crate::{Nts3Parameters, Parameter, SmoothStatus, SmoothedParameter, StereoBuffer};
    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static FREES: AtomicUsize = AtomicUsize::new(0);
    static RAW_CALLS: AtomicUsize = AtomicUsize::new(0);
    static RAW_POINTER_SELECT: AtomicUsize = AtomicUsize::new(0);
    static TOUCH_BITS: AtomicU32 = AtomicU32::new(0);
    static TEMPO_BITS: AtomicU32 = AtomicU32::new(0);
    static TICKS: AtomicU32 = AtomicU32::new(0);
    static LIFECYCLE_BITS: AtomicU32 = AtomicU32::new(0);
    static CONSTRUCTION_ORDER: AtomicU32 = AtomicU32::new(0);
    static FAILING_INIT_KIND: AtomicUsize = AtomicUsize::new(0);

    #[repr(align(4096))]
    struct Arena([u8; 16 * 1024]);

    static mut ARENA: Arena = Arena([0; 16 * 1024]);
    static RAW_A: [f32; 4] = [10.0, 11.0, 12.0, 13.0];
    static RAW_B: [f32; 4] = [20.0, 21.0, 22.0, 23.0];

    unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        if size <= 16 * 1024 {
            // SAFETY: lifecycle tests are serialized and the arena is released
            // before reuse; `addr_of_mut!` creates no mutable reference.
            unsafe { core::ptr::addr_of_mut!(ARENA.0).cast::<u8>() }
        } else {
            ptr::null_mut()
        }
    }

    unsafe extern "C" fn free(pointer: *const u8) {
        // SAFETY: address comparison does not read the mutable static, and the
        // lifecycle test is its only serialized user.
        let expected = unsafe { core::ptr::addr_of!(ARENA.0).cast::<u8>() };
        assert_eq!(pointer, expected);
        FREES.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn available() -> usize {
        16 * 1024
    }

    unsafe extern "C" fn get_raw_input() -> *const f32 {
        RAW_CALLS.fetch_add(1, Ordering::SeqCst);
        if RAW_POINTER_SELECT.fetch_xor(1, Ordering::SeqCst) == 0 {
            RAW_A.as_ptr()
        } else {
            RAW_B.as_ptr()
        }
    }

    struct TestParameters;

    impl Default for TestParameters {
        fn default() -> Self {
            assert_eq!(CONSTRUCTION_ORDER.swap(1, Ordering::SeqCst), 0);
            Self
        }
    }

    impl Sealed for TestParameters {}
    impl Nts3Parameters for TestParameters {}

    struct TestPlugin;

    impl Default for TestPlugin {
        fn default() -> Self {
            assert_eq!(CONSTRUCTION_ORDER.swap(2, Ordering::SeqCst), 1);
            Self
        }
    }

    impl Nts3Plugin for TestPlugin {
        type Parameters = TestParameters;

        fn initialize(&mut self, context: &InitContext<'_>) -> Result<(), InitError> {
            assert_eq!(CONSTRUCTION_ORDER.swap(3, Ordering::SeqCst), 2);
            assert_eq!(context.sample_rate_hz(), 48_000);
            assert_eq!(context.maximum_frames(), 2);
            assert_eq!(context.touch_area(), [501, 200]);
            LIFECYCLE_BITS.fetch_or(1, Ordering::SeqCst);
            Ok(())
        }

        fn reset(&mut self) {
            LIFECYCLE_BITS.fetch_or(2, Ordering::SeqCst);
        }

        fn resume(&mut self) {
            LIFECYCLE_BITS.fetch_or(4, Ordering::SeqCst);
        }

        fn suspend(&mut self) {
            LIFECYCLE_BITS.fetch_or(8, Ordering::SeqCst);
        }

        fn teardown(&mut self) {
            LIFECYCLE_BITS.fetch_or(16, Ordering::SeqCst);
        }

        fn process(&mut self, _parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
            let mut raw = buffer.raw_input().unwrap().frames();
            for mut frame in buffer.frames_mut() {
                let input = frame.input();
                let raw = raw.next().unwrap();
                frame.write([input[0] + raw[0], input[1] + raw[1]]);
            }
        }

        fn touch_event(&mut self, event: TouchEvent) {
            let phase_bit = match event.phase() {
                TouchPhase::Began => 1,
                TouchPhase::Moved => 2,
                TouchPhase::Ended => 4,
                TouchPhase::Stationary => 8,
                TouchPhase::Cancelled => 16,
            };
            assert_eq!(event.clamped_position(), [500, 199]);
            TOUCH_BITS.fetch_or(phase_bit, Ordering::SeqCst);
        }

        fn tempo_changed(&mut self, bpm: f32) {
            TEMPO_BITS.store(bpm.to_bits(), Ordering::SeqCst);
        }

        fn tempo_4ppqn_tick(&mut self, counter: u32) {
            TICKS.store(counter, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct FailingParameters;
    impl Sealed for FailingParameters {}
    impl Nts3Parameters for FailingParameters {}

    const fn parameter_name(value: &[u8]) -> [u8; UNIT_PARAM_NAME_SIZE] {
        let mut result = [0; UNIT_PARAM_NAME_SIZE];
        let mut index = 0;
        while index < value.len() && index < UNIT_PARAM_NAME_SIZE - 1 {
            result[index] = value[index];
            index += 1;
        }
        result
    }

    const MANUAL_DESCRIPTORS: [UnitParam; UNIT_MAX_PARAM_COUNT] = [
        UnitParam::new(
            1,
            2_000,
            1,
            500,
            UNIT_PARAM_TYPE_MSEC,
            UnitParamFormat::FIXED_ZERO,
            parameter_name(b"TIME"),
        ),
        UnitParam::new(
            0,
            1_000,
            0,
            0,
            UNIT_PARAM_TYPE_PERCENT,
            UnitParamFormat::from_raw(0x11),
            parameter_name(b"FEEDBACK"),
        ),
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
    ];
    const MANUAL_MAPPINGS: [GenericfxParamMapping; UNIT_MAX_PARAM_COUNT] = [
        GenericfxParamMapping::new(
            GENERICFX_PARAM_ASSIGN_X,
            GenericfxCurve::from_raw(1),
            1,
            2_000,
            500,
        ),
        GenericfxParamMapping::new(
            GENERICFX_PARAM_ASSIGN_Y,
            GenericfxCurve::LINEAR_UNIPOLAR,
            0,
            1_000,
            0,
        ),
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
    ];

    struct ManualParameters {
        time: SmoothedParameter,
        feedback: Parameter,
    }

    impl Default for ManualParameters {
        fn default() -> Self {
            Self {
                time: SmoothedParameter::from_descriptor(&MANUAL_DESCRIPTORS[0], 100.0),
                feedback: Parameter::from_descriptor(&MANUAL_DESCRIPTORS[1]),
            }
        }
    }

    impl Sealed for ManualParameters {}

    impl Nts3Parameters for ManualParameters {
        const DESCRIPTORS: [UnitParam; UNIT_MAX_PARAM_COUNT] = MANUAL_DESCRIPTORS;
        const MAPPINGS: [GenericfxParamMapping; UNIT_MAX_PARAM_COUNT] = MANUAL_MAPPINGS;
        const COUNT: usize = 2;

        fn get(&self, index: u8) -> Option<i32> {
            match index {
                0 => Some(i32::from(self.time.raw())),
                1 => Some(i32::from(self.feedback.raw())),
                _ => None,
            }
        }

        fn set(&mut self, index: u8, value: i32) -> bool {
            match index {
                0 => {
                    self.time.set(value);
                    true
                }
                1 => {
                    self.feedback.set(value);
                    true
                }
                _ => false,
            }
        }

        fn initialize_smoothers(&mut self, sample_rate: f32) {
            self.time.set_sample_rate(sample_rate);
        }

        fn reset_smoothers(&mut self) {
            self.time.reset_smoothing();
        }

        fn begin_block(&mut self) {
            self.time.begin_block();
        }

        fn end_block(&mut self) {
            self.time.end_block();
        }

        fn string_value(&self, index: u8, value: i32) -> Option<&'static core::ffi::CStr> {
            (index == 1 && value >= 1_000).then_some(c"FULL")
        }
    }

    #[derive(Default)]
    struct ManualPlugin;

    impl Nts3Plugin for ManualPlugin {
        type Parameters = ManualParameters;

        fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
            for mut frame in buffer.frames_mut() {
                frame.write([
                    parameters.time.next_plain(),
                    parameters.feedback.normalized(),
                ]);
            }
        }
    }

    #[derive(Default)]
    struct FailingPlugin;

    impl Nts3Plugin for FailingPlugin {
        type Parameters = FailingParameters;

        fn initialize(&mut self, _context: &InitContext<'_>) -> Result<(), InitError> {
            Err(match FAILING_INIT_KIND.load(Ordering::SeqCst) {
                0 => InitError::Target,
                1 => InitError::ApiVersion,
                2 => InitError::SampleRate,
                3 => InitError::Geometry,
                4 => InitError::Memory,
                _ => InitError::Undefined,
            })
        }

        fn process(&mut self, _parameters: &mut Self::Parameters, _buffer: &mut StereoBuffer<'_>) {}
    }

    fn hooks(context: &UnitRuntimeGenericfxContext) -> UnitRuntimeHooks {
        UnitRuntimeHooks::new(
            (context as *const UnitRuntimeGenericfxContext).cast(),
            Some(allocate),
            Some(free),
            Some(available),
        )
    }

    fn descriptor(
        context: &UnitRuntimeGenericfxContext,
        target: u32,
        api: u32,
        rate: u32,
        frames: u16,
        inputs: u8,
        outputs: u8,
    ) -> UnitRuntimeDescriptor {
        UnitRuntimeDescriptor::new(target, api, rate, frames, inputs, outputs, hooks(context))
    }

    fn valid_descriptor(context: &UnitRuntimeGenericfxContext) -> UnitRuntimeDescriptor {
        descriptor(
            context,
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            UNIT_API_2_0_0,
            48_000,
            2,
            2,
            2,
        )
    }

    #[test]
    fn validation_failures_map_to_sdk_codes_before_construction() {
        let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
        let valid = valid_descriptor(&context);

        // SAFETY: null is explicitly rejected without dereference.
        assert_eq!(
            unsafe { validate_descriptor(ptr::null()) }.err(),
            Some(UNIT_ERR_UNDEF)
        );

        let cases = [
            (
                descriptor(&context, 0, UNIT_API_2_0_0, 48_000, 2, 2, 2),
                UNIT_ERR_TARGET,
            ),
            (
                descriptor(
                    &context,
                    UNIT_TARGET_NTS3_KAOSS_GENERICFX,
                    (2 << 16) | (1 << 8),
                    48_000,
                    2,
                    2,
                    2,
                ),
                UNIT_ERR_API_VERSION,
            ),
            (
                descriptor(
                    &context,
                    UNIT_TARGET_NTS3_KAOSS_GENERICFX,
                    UNIT_API_2_0_0,
                    44_100,
                    2,
                    2,
                    2,
                ),
                UNIT_ERR_SAMPLERATE,
            ),
            (
                descriptor(
                    &context,
                    UNIT_TARGET_NTS3_KAOSS_GENERICFX,
                    UNIT_API_2_0_0,
                    48_000,
                    0,
                    2,
                    2,
                ),
                UNIT_ERR_GEOMETRY,
            ),
            (
                descriptor(
                    &context,
                    UNIT_TARGET_NTS3_KAOSS_GENERICFX,
                    UNIT_API_2_0_0,
                    48_000,
                    2,
                    1,
                    2,
                ),
                UNIT_ERR_GEOMETRY,
            ),
        ];
        for (descriptor, expected) in cases {
            // SAFETY: each local descriptor and context remain readable.
            assert_eq!(
                unsafe { validate_descriptor(&descriptor) }.err(),
                Some(expected)
            );
        }

        let missing_hooks = UnitRuntimeDescriptor::new(
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            UNIT_API_2_0_0,
            48_000,
            2,
            2,
            2,
            UnitRuntimeHooks::new(
                (&context as *const UnitRuntimeGenericfxContext).cast(),
                None,
                Some(free),
                Some(available),
            ),
        );
        // SAFETY: the descriptor remains readable.
        assert_eq!(
            unsafe { validate_descriptor(&missing_hooks) }.err(),
            Some(UNIT_ERR_MEMORY)
        );

        let null_context = UnitRuntimeDescriptor::new(
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            UNIT_API_2_0_0,
            48_000,
            2,
            2,
            2,
            UnitRuntimeHooks::new(ptr::null(), Some(allocate), Some(free), Some(available)),
        );
        // SAFETY: the descriptor remains readable and context is rejected.
        assert_eq!(
            unsafe { validate_descriptor(&null_context) }.err(),
            Some(UNIT_ERR_UNDEF)
        );

        // SAFETY: the valid descriptor and context remain readable.
        assert!(unsafe { validate_descriptor(&valid) }.is_ok());
    }

    #[test]
    fn lifecycle_render_raw_refresh_touch_and_tempo_dispatch() {
        let _guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ALLOCATIONS.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);
        RAW_CALLS.store(0, Ordering::SeqCst);
        RAW_POINTER_SELECT.store(0, Ordering::SeqCst);
        TOUCH_BITS.store(0, Ordering::SeqCst);
        LIFECYCLE_BITS.store(0, Ordering::SeqCst);
        CONSTRUCTION_ORDER.store(0, Ordering::SeqCst);

        let context = UnitRuntimeGenericfxContext::new(501, 200, Some(get_raw_input));
        let descriptor = valid_descriptor(&context);
        let mut controller = RuntimeController::<TestPlugin>::new();

        let mut before = [1.0, 2.0];
        // SAFETY: pointer storage is valid, but inactive state rejects it first.
        assert_eq!(
            unsafe { controller.render(before.as_ptr(), before.as_mut_ptr(), 1) },
            Err(RenderError::Inactive)
        );
        assert!(!controller.reset());
        assert!(!controller.touch_event(0, UNIT_TOUCH_PHASE_BEGAN, 0, 0));

        // SAFETY: descriptor/context remain valid throughout initialization.
        assert_eq!(
            unsafe { controller.initialize(&descriptor, 4096) },
            UNIT_ERR_NONE
        );
        assert_eq!(CONSTRUCTION_ORDER.load(Ordering::SeqCst), 3);
        assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(controller.allocation_stats().unwrap().allocations, 0);

        let input = [1.0, 2.0, 3.0, 4.0];
        let mut first = [0.0; 4];
        let mut second = [0.0; 4];
        // SAFETY: arrays contain two separate stereo frames.
        unsafe {
            controller
                .render(input.as_ptr(), first.as_mut_ptr(), 2)
                .unwrap();
            controller
                .render(input.as_ptr(), second.as_mut_ptr(), 2)
                .unwrap();
        }
        assert_eq!(first, [11.0, 13.0, 15.0, 17.0]);
        assert_eq!(second, [21.0, 23.0, 25.0, 27.0]);
        assert_eq!(RAW_CALLS.load(Ordering::SeqCst), 2);

        // A zero-frame render still refreshes raw input but needs no storage.
        // SAFETY: zero frames permit null audio pointers and perform no access.
        unsafe {
            controller.render(ptr::null(), ptr::null_mut(), 0).unwrap();
        }
        assert_eq!(RAW_CALLS.load(Ordering::SeqCst), 3);

        // Frame bounds are checked before hook invocation or pointer access.
        // SAFETY: rejected before accessing the intentionally null pointers.
        assert_eq!(
            unsafe { controller.render(ptr::null(), ptr::null_mut(), 3) },
            Err(RenderError::FrameCountExceeded)
        );
        assert_eq!(RAW_CALLS.load(Ordering::SeqCst), 3);

        for phase in [
            UNIT_TOUCH_PHASE_BEGAN,
            UNIT_TOUCH_PHASE_MOVED,
            UNIT_TOUCH_PHASE_ENDED,
            UNIT_TOUCH_PHASE_STATIONARY,
            UNIT_TOUCH_PHASE_CANCELLED,
        ] {
            assert!(controller.touch_event(0, phase, 999, 999));
        }
        assert_eq!(TOUCH_BITS.load(Ordering::SeqCst), 0b1_1111);
        assert!(!controller.touch_event(0, 99, 0, 0));

        assert!(controller.set_tempo((123 << 16) | 0x8000));
        assert_eq!(f32::from_bits(TEMPO_BITS.load(Ordering::SeqCst)), 123.5);
        assert!(controller.tempo_4ppqn_tick(77));
        assert_eq!(TICKS.load(Ordering::SeqCst), 77);

        assert!(controller.reset());
        assert!(controller.suspend());
        assert!(controller.reset());
        assert!(controller.set_tempo(120 << 16));
        assert!(controller.resume());
        assert!(controller.teardown());
        assert_eq!(FREES.load(Ordering::SeqCst), 1);
        assert_eq!(LIFECYCLE_BITS.load(Ordering::SeqCst), 0b1_1111);
        assert!(!controller.teardown());

        // A complete second lifecycle is supported.
        CONSTRUCTION_ORDER.store(0, Ordering::SeqCst);
        // SAFETY: descriptor/context remain valid throughout initialization.
        assert_eq!(
            unsafe { controller.initialize(&descriptor, 4096) },
            UNIT_ERR_NONE
        );
        assert!(controller.teardown());
        assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 2);
        assert_eq!(FREES.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn memory_and_plugin_failures_map_and_leave_no_ready_state() {
        let _guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ALLOCATIONS.store(0, Ordering::SeqCst);
        FREES.store(0, Ordering::SeqCst);
        CONSTRUCTION_ORDER.store(0, Ordering::SeqCst);

        let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
        let descriptor = valid_descriptor(&context);
        let mut ordered = RuntimeController::<TestPlugin>::new();
        // The unavailable arena is rejected before parameter/plugin defaults.
        // SAFETY: descriptor/context remain valid throughout the call.
        assert_eq!(
            unsafe { ordered.initialize(&descriptor, 16 * 1024 + 1) },
            UNIT_ERR_MEMORY
        );
        assert_eq!(CONSTRUCTION_ORDER.load(Ordering::SeqCst), 0);
        assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
        assert!(!ordered.reset());

        let expected = [
            UNIT_ERR_TARGET,
            UNIT_ERR_API_VERSION,
            UNIT_ERR_SAMPLERATE,
            UNIT_ERR_GEOMETRY,
            UNIT_ERR_MEMORY,
            UNIT_ERR_UNDEF,
        ];
        let mut failing = RuntimeController::<FailingPlugin>::new();
        for (kind, code) in expected.into_iter().enumerate() {
            FAILING_INIT_KIND.store(kind, Ordering::SeqCst);
            // SAFETY: descriptor/context remain valid throughout the call.
            assert_eq!(unsafe { failing.initialize(&descriptor, 4096) }, code);
            assert!(!failing.reset());
        }
        assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), expected.len());
        assert_eq!(FREES.load(Ordering::SeqCst), expected.len());
    }

    #[test]
    fn manual_parameter_metadata_dispatch_and_runtime_hooks_stay_consistent() {
        let _guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        CONSTRUCTION_ORDER.store(0, Ordering::SeqCst);

        assert_eq!(ManualParameters::COUNT, 2);
        assert_eq!(core::mem::size_of::<ManualPlugin>(), 0);
        assert_eq!(core::mem::size_of::<ManualParameters>(), 44);
        assert_eq!(
            core::mem::size_of::<Runtime<ManualPlugin>>(),
            if cfg!(target_pointer_width = "64") {
                80
            } else {
                64
            }
        );
        let defaults = ManualParameters::default();
        assert_eq!(defaults.time.raw(), ManualParameters::DESCRIPTORS[0].init());
        assert_eq!(
            defaults.feedback.raw(),
            ManualParameters::DESCRIPTORS[1].init()
        );
        assert_eq!(ManualParameters::MAPPINGS[0].min(), 1);
        assert_eq!(ManualParameters::MAPPINGS[0].max(), 2_000);

        let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
        let descriptor = valid_descriptor(&context);
        let mut controller = RuntimeController::<ManualPlugin>::new();
        // SAFETY: descriptor/context remain readable throughout initialization.
        assert_eq!(
            unsafe { controller.initialize(&descriptor, 4096) },
            UNIT_ERR_NONE
        );
        assert_eq!(controller.get_parameter(0), 500);
        assert_eq!(controller.get_parameter(1), 0);
        assert_eq!(controller.get_parameter(200), 0);
        assert!(!controller.set_parameter(200, i32::MAX));
        assert!(controller.parameter_string_value(200, 0).is_null());

        assert!(controller.set_parameter(0, i32::MAX));
        assert!(controller.set_parameter(1, i32::MAX));
        assert_eq!(controller.get_parameter(0), 2_000);
        assert_eq!(controller.get_parameter(1), 1_000);
        assert!(!controller.parameter_string_value(1, 1_000).is_null());

        let input = [0.0; 2];
        let mut output = [0.0; 2];
        // SAFETY: one separate stereo input/output frame is live for the call.
        unsafe {
            controller
                .render(input.as_ptr(), output.as_mut_ptr(), 1)
                .unwrap();
        }
        assert!(output[0] > 500.0 && output[0] < 2_000.0);
        assert_eq!(output[1], 1.0);
        assert_eq!(
            controller
                .state
                .active_mut()
                .unwrap()
                .parameters
                .time
                .status(),
            SmoothStatus::Active
        );

        // Reset retains targets while snapping in-flight smoothing to them.
        assert!(controller.reset());
        // SAFETY: one separate stereo input/output frame is live for the call.
        unsafe {
            controller
                .render(input.as_ptr(), output.as_mut_ptr(), 1)
                .unwrap();
        }
        assert_eq!(output, [2_000.0, 1.0]);

        assert!(controller.suspend());
        assert!(controller.set_parameter(1, i32::MIN));
        assert_eq!(controller.get_parameter(1), 0);
        assert!(controller.resume());
        assert!(controller.teardown());
    }

    #[test]
    fn uq16_16_conversion_covers_integer_fraction_and_maximum() {
        assert_eq!(uq16_16_to_f32(0), 0.0);
        assert_eq!(uq16_16_to_f32(120 << 16), 120.0);
        assert_eq!(uq16_16_to_f32((120 << 16) | 0x8000), 120.5);
        assert_eq!(uq16_16_to_f32(u32::MAX), 65_536.0);
    }
}

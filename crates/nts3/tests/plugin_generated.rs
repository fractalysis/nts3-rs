use core::ptr;

use nts3::__private::{UNIT_API_VERSION, UNIT_TARGET_NTS3_KAOSS_GENERICFX, UnitRuntimeDescriptor};
use nts3::prelude::*;
use nts3_sys::{
    UNIT_ERR_NONE, UnitGetParamStringValueFn, UnitGetParamValueFn, UnitInitFn, UnitRenderFn,
    UnitResetFn, UnitResumeFn, UnitRuntimeGenericfxContext, UnitRuntimeHooks, UnitSetParamValueFn,
    UnitSetTempoFn, UnitSuspendFn, UnitTeardownFn, UnitTempo4ppqnTickFn, UnitTouchEventFn,
};

#[derive(Nts3Parameters)]
struct TestParameters {
    #[parameter(
        name = "GAIN",
        min = 0,
        max = 10,
        default = 5,
        parameter_type = "percent"
    )]
    gain: Parameter,
}

#[derive(Default)]
struct TestPlugin;

#[nts3::plugin(
    name = "Generated Adapter",
    developer_id = 0x5255_5354,
    unit_id = 0x4745_4E41,
    sdram_bytes = 4096
)]
impl Nts3Plugin for TestPlugin {
    type Parameters = TestParameters;

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        let gain = parameters.gain.normalized();
        for mut frame in buffer.frames_mut() {
            let [left, right] = frame.input();
            frame.write([left * gain, right * gain]);
        }
    }
}

const _: UnitInitFn = unit_init;
const _: UnitTeardownFn = unit_teardown;
const _: UnitResetFn = unit_reset;
const _: UnitResumeFn = unit_resume;
const _: UnitSuspendFn = unit_suspend;
const _: UnitRenderFn = unit_render;
const _: UnitGetParamValueFn = unit_get_param_value;
const _: UnitGetParamStringValueFn = unit_get_param_str_value;
const _: UnitSetParamValueFn = unit_set_param_value;
const _: UnitSetTempoFn = unit_set_tempo;
const _: UnitTempo4ppqnTickFn = unit_tempo_4ppqn_tick;
const _: UnitTouchEventFn = unit_touch_event;

#[repr(align(4096))]
struct Arena([u8; 4096]);

static mut ARENA: Arena = Arena([0; 4096]);

unsafe extern "C" fn allocate(bytes: usize) -> *mut u8 {
    if bytes <= 4096 {
        // SAFETY: this fixture's single callback lifecycle is serialized, and
        // taking an address creates no reference to the mutable static.
        unsafe { core::ptr::addr_of_mut!(ARENA.0).cast() }
    } else {
        ptr::null_mut()
    }
}

unsafe extern "C" fn free(pointer: *const u8) {
    // SAFETY: address comparison creates no reference and the test lifecycle
    // has already dropped every arena-backed runtime value.
    let expected = unsafe { core::ptr::addr_of!(ARENA.0).cast() };
    assert_eq!(pointer, expected);
}

unsafe extern "C" fn available() -> usize {
    4096
}

#[test]
fn generated_metadata_and_callbacks_share_one_runtime() {
    let common = unit_header.common();
    assert_eq!(common.header_size(), 376);
    assert_eq!(common.target(), UNIT_TARGET_NTS3_KAOSS_GENERICFX);
    assert_eq!(common.api(), UNIT_API_VERSION);
    assert_eq!(common.developer_id(), 0x5255_5354);
    assert_eq!(common.unit_id(), 0x4745_4E41);
    assert_eq!(common.version(), 0x0000_0100);
    assert_eq!(common.name(), *b"Generated Adapter\0\0\0");
    assert_eq!(common.num_params(), 1);
    assert_eq!(
        common.params()[0].name(),
        *b"GAIN\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
    );

    assert_eq!(nts3_resources.magic(), *b"N3RS");
    assert_eq!(nts3_resources.schema_version(), 1);
    assert_eq!(nts3_resources.record_size(), 12);
    assert_eq!(nts3_resources.sdram_bytes(), 4096);

    let context = UnitRuntimeGenericfxContext::new(1024, 1024, None);
    let descriptor = UnitRuntimeDescriptor::new(
        UNIT_TARGET_NTS3_KAOSS_GENERICFX,
        UNIT_API_VERSION,
        48_000,
        4,
        2,
        2,
        UnitRuntimeHooks::new(
            (&context as *const UnitRuntimeGenericfxContext).cast(),
            Some(allocate),
            Some(free),
            Some(available),
        ),
    );

    // SAFETY: descriptor/context and all hook storage remain valid for the
    // complete callback lifecycle.
    assert_eq!(unsafe { unit_init(&descriptor) }, UNIT_ERR_NONE);
    assert_eq!(unit_get_param_value(0), 5);
    assert_eq!(unit_get_param_value(99), 0);
    assert!(unit_get_param_str_value(0, 5).is_null());

    // Host macro expansion must not replace the process allocator with the
    // target's sealed 4 KiB arena allocator.
    let host_allocation = vec![0_u8; 8192];
    assert_eq!(host_allocation.len(), 8192);

    let input = [1.0, -1.0, 0.5, -0.5];
    let mut output = [0.0; 4];
    // SAFETY: both arrays contain two disjoint interleaved stereo frames.
    unsafe { unit_render(input.as_ptr(), output.as_mut_ptr(), 2) };
    assert_eq!(output, [0.5, -0.5, 0.25, -0.25]);

    unit_set_param_value(0, i32::MAX);
    assert_eq!(unit_get_param_value(0), 10);
    // SAFETY: both arrays remain valid for two frames.
    unsafe { unit_render(input.as_ptr(), output.as_mut_ptr(), 2) };
    assert_eq!(output, input);

    unit_reset();
    unit_suspend();
    unit_resume();
    unit_set_tempo(120 << 16);
    unit_tempo_4ppqn_tick(1);
    unit_touch_event(0, 0, 0, 0);
    unit_teardown();
    assert_eq!(unit_get_param_value(0), 0);
}

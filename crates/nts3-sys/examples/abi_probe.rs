use core::mem::{align_of, size_of};
use nts3_sys::*;

macro_rules! layout {
    ($ty:ty, $label:literal) => {
        println!(concat!("LAYOUT.", $label, ".size={}"), size_of::<$ty>());
        println!(concat!("LAYOUT.", $label, ".align={}"), align_of::<$ty>());
    };
}
macro_rules! offset {
    ($label:literal, $value:expr) => {
        println!(concat!("OFFSET.", $label, "={}"), $value);
    };
}
macro_rules! constant {
    ($label:literal, $value:expr) => {
        println!(concat!("CONST.", $label, "={}"), $value);
    };
}
macro_rules! signature {
    ($label:literal, $ty:ty) => {{
        let _: Option<$ty> = None;
        println!(concat!("SIGNATURE.", $label, "=1"));
    }};
}

fn dummy_header() -> GenericfxUnitHeader {
    let decimal_one = UnitParamFormat::new(1, UNIT_PARAM_FRAC_MODE_DECIMAL).unwrap();
    let exp_bipolar = GenericfxCurve::new(GENERICFX_CURVE_EXP, GENERICFX_CURVE_BIPOLAR).unwrap();
    GenericfxUnitHeader::new(
        UnitHeader::new(
            size_of::<GenericfxUnitHeader>() as u32,
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            UNIT_API_VERSION,
            0,
            0,
            0x0001_0000,
            *b"dummy\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            4,
            [
                UnitParam::new(
                    0,
                    1023,
                    0,
                    0,
                    UNIT_PARAM_TYPE_NONE,
                    UnitParamFormat::FIXED_ZERO,
                    *b"PARAM1\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                ),
                UnitParam::new(
                    0,
                    1023,
                    0,
                    0,
                    UNIT_PARAM_TYPE_NONE,
                    UnitParamFormat::FIXED_ZERO,
                    *b"PARAM2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                ),
                UnitParam::new(
                    -1000,
                    1000,
                    0,
                    0,
                    UNIT_PARAM_TYPE_DRYWET,
                    decimal_one,
                    *b"DEPTH\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                ),
                UnitParam::new(
                    0,
                    3,
                    0,
                    1,
                    UNIT_PARAM_TYPE_STRINGS,
                    UnitParamFormat::FIXED_ZERO,
                    *b"PARAM4\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                ),
                UNUSED_PARAM,
                UNUSED_PARAM,
                UNUSED_PARAM,
                UNUSED_PARAM,
            ],
        ),
        [
            GenericfxParamMapping::new(
                GENERICFX_PARAM_ASSIGN_X,
                GenericfxCurve::LINEAR_UNIPOLAR,
                0,
                1023,
                256,
            ),
            GenericfxParamMapping::new(
                GENERICFX_PARAM_ASSIGN_Y,
                GenericfxCurve::LINEAR_UNIPOLAR,
                512,
                1023,
                512,
            ),
            GenericfxParamMapping::new(GENERICFX_PARAM_ASSIGN_DEPTH, exp_bipolar, -1000, 1000, 0),
            GenericfxParamMapping::new(
                GENERICFX_PARAM_ASSIGN_NONE,
                GenericfxCurve::LINEAR_UNIPOLAR,
                0,
                3,
                1,
            ),
            UNUSED_MAPPING,
            UNUSED_MAPPING,
            UNUSED_MAPPING,
            UNUSED_MAPPING,
        ],
    )
}

fn main() {
    layout!(UnitRuntimeHooks, "runtime_hooks");
    offset!("runtime_hooks.context", layout::RUNTIME_HOOKS_CONTEXT);
    offset!("runtime_hooks.alloc", layout::RUNTIME_HOOKS_ALLOC);
    offset!("runtime_hooks.free", layout::RUNTIME_HOOKS_FREE);
    offset!("runtime_hooks.avail", layout::RUNTIME_HOOKS_AVAIL);

    layout!(UnitRuntimeDescriptor, "runtime_desc");
    offset!("runtime_desc.target", layout::RUNTIME_DESC_TARGET);
    offset!("runtime_desc.api", layout::RUNTIME_DESC_API);
    offset!("runtime_desc.sample_rate", layout::RUNTIME_DESC_SAMPLE_RATE);
    offset!("runtime_desc.frames", layout::RUNTIME_DESC_FRAMES);
    offset!("runtime_desc.inputs", layout::RUNTIME_DESC_INPUTS);
    offset!("runtime_desc.outputs", layout::RUNTIME_DESC_OUTPUTS);
    offset!("runtime_desc.hooks", layout::RUNTIME_DESC_HOOKS);

    layout!(UnitRuntimeGenericfxContext, "genericfx_context");
    offset!("genericfx_context.width", layout::GENERICFX_CONTEXT_WIDTH);
    offset!("genericfx_context.height", layout::GENERICFX_CONTEXT_HEIGHT);
    offset!(
        "genericfx_context.raw_input",
        layout::GENERICFX_CONTEXT_RAW_INPUT
    );

    layout!(UnitParam, "unit_param");
    offset!("unit_param.min", layout::PARAM_MIN);
    offset!("unit_param.max", layout::PARAM_MAX);
    offset!("unit_param.center", layout::PARAM_CENTER);
    offset!("unit_param.init", layout::PARAM_INIT);
    offset!("unit_param.type", layout::PARAM_TYPE);
    offset!("unit_param.format", layout::PARAM_FORMAT);
    offset!("unit_param.name", layout::PARAM_NAME);

    layout!(UnitHeader, "unit_header");
    offset!("unit_header.header_size", layout::HEADER_SIZE);
    offset!("unit_header.target", layout::HEADER_TARGET);
    offset!("unit_header.api", layout::HEADER_API);
    offset!("unit_header.dev_id", layout::HEADER_DEV_ID);
    offset!("unit_header.unit_id", layout::HEADER_UNIT_ID);
    offset!("unit_header.version", layout::HEADER_VERSION);
    offset!("unit_header.name", layout::HEADER_NAME);
    offset!("unit_header.reserved0", layout::HEADER_RESERVED0);
    offset!("unit_header.reserved1", layout::HEADER_RESERVED1);
    offset!("unit_header.num_params", layout::HEADER_NUM_PARAMS);
    offset!("unit_header.params", layout::HEADER_PARAMS);

    layout!(GenericfxParamMapping, "mapping");
    offset!("mapping.assign", layout::MAPPING_ASSIGN);
    offset!("mapping.curve", layout::MAPPING_CURVE);
    offset!("mapping.min", layout::MAPPING_MIN);
    offset!("mapping.max", layout::MAPPING_MAX);
    offset!("mapping.value", layout::MAPPING_VALUE);

    layout!(GenericfxUnitHeader, "generic_header");
    offset!("generic_header.common", layout::GENERIC_HEADER_COMMON);
    offset!("generic_header.mappings", layout::GENERIC_HEADER_MAPPINGS);

    constant!("UNIT_MODULE_GLOBAL", UNIT_MODULE_GLOBAL);
    constant!("UNIT_MODULE_MODFX", UNIT_MODULE_MODFX);
    constant!("UNIT_MODULE_DELFX", UNIT_MODULE_DELFX);
    constant!("UNIT_MODULE_REVFX", UNIT_MODULE_REVFX);
    constant!("UNIT_MODULE_OSC", UNIT_MODULE_OSC);
    constant!("UNIT_MODULE_SYNTH", UNIT_MODULE_SYNTH);
    constant!("UNIT_MODULE_MASTERFX", UNIT_MODULE_MASTERFX);
    constant!("UNIT_MODULE_GENERICFX", UNIT_MODULE_GENERICFX);
    constant!("NUM_UNIT_MODULES", NUM_UNIT_MODULES);
    constant!("TARGET_NTS3", UNIT_TARGET_NTS3_KAOSS);
    constant!("TARGET_NTS3_GLOBAL", UNIT_TARGET_NTS3_KAOSS_GLOBAL);
    constant!("TARGET_NTS3_GENERICFX", UNIT_TARGET_NTS3_KAOSS_GENERICFX);
    constant!("TARGET_PLATFORM", UNIT_TARGET_PLATFORM);
    constant!("TARGET_PLATFORM_MASK", UNIT_TARGET_PLATFORM_MASK);
    constant!("TARGET_MODULE_MASK", UNIT_TARGET_MODULE_MASK);
    constant!("API_1_0_0", UNIT_API_1_0_0);
    constant!("API_1_1_0", UNIT_API_1_1_0);
    constant!("API_2_0_0", UNIT_API_2_0_0);
    constant!("API_VERSION", UNIT_API_VERSION);
    constant!("API_MAJOR_MASK", UNIT_API_MAJOR_MASK);
    constant!("API_MINOR_MASK", UNIT_API_MINOR_MASK);
    constant!("API_PATCH_MASK", UNIT_API_PATCH_MASK);
    constant!("MAX_PARAMS", UNIT_MAX_PARAM_COUNT);
    constant!("GENERICFX_MAX_PARAMS", UNIT_GENERICFX_MAX_PARAM_COUNT);
    constant!("PARAM_NAME_LEN", UNIT_PARAM_NAME_LEN);
    constant!("PARAM_NAME_SIZE", UNIT_PARAM_NAME_SIZE);
    constant!("UNIT_NAME_LEN", UNIT_NAME_LEN);
    constant!("UNIT_NAME_SIZE", UNIT_NAME_SIZE);
    constant!("GENERICFX_FIXED_IDS", NUM_UNIT_GENERICFX_FIXED_PARAM_IDS);

    constant!("PARAM_NONE", UNIT_PARAM_TYPE_NONE);
    constant!("PARAM_PERCENT", UNIT_PARAM_TYPE_PERCENT);
    constant!("PARAM_DB", UNIT_PARAM_TYPE_DB);
    constant!("PARAM_CENTS", UNIT_PARAM_TYPE_CENTS);
    constant!("PARAM_SEMI", UNIT_PARAM_TYPE_SEMI);
    constant!("PARAM_OCT", UNIT_PARAM_TYPE_OCT);
    constant!("PARAM_HERTZ", UNIT_PARAM_TYPE_HERTZ);
    constant!("PARAM_KHERTZ", UNIT_PARAM_TYPE_KHERTZ);
    constant!("PARAM_BPM", UNIT_PARAM_TYPE_BPM);
    constant!("PARAM_MSEC", UNIT_PARAM_TYPE_MSEC);
    constant!("PARAM_SEC", UNIT_PARAM_TYPE_SEC);
    constant!("PARAM_ENUM", UNIT_PARAM_TYPE_ENUM);
    constant!("PARAM_STRINGS", UNIT_PARAM_TYPE_STRINGS);
    constant!("PARAM_RESERVED0", UNIT_PARAM_TYPE_RESERVED0);
    constant!("PARAM_DRYWET", UNIT_PARAM_TYPE_DRYWET);
    constant!("PARAM_PAN", UNIT_PARAM_TYPE_PAN);
    constant!("PARAM_SPREAD", UNIT_PARAM_TYPE_SPREAD);
    constant!("PARAM_ONOFF", UNIT_PARAM_TYPE_ONOFF);
    constant!("PARAM_MIDI_NOTE", UNIT_PARAM_TYPE_MIDI_NOTE);
    constant!("PARAM_TYPE_COUNT", UNIT_PARAM_TYPE_COUNT);
    constant!("FRAC_FIXED", UNIT_PARAM_FRAC_MODE_FIXED);
    constant!("FRAC_DECIMAL", UNIT_PARAM_FRAC_MODE_DECIMAL);

    constant!("ASSIGN_NONE", GENERICFX_PARAM_ASSIGN_NONE);
    constant!("ASSIGN_X", GENERICFX_PARAM_ASSIGN_X);
    constant!("ASSIGN_Y", GENERICFX_PARAM_ASSIGN_Y);
    constant!("ASSIGN_DEPTH", GENERICFX_PARAM_ASSIGN_DEPTH);
    constant!("ASSIGN_COUNT", NUM_GENERICFX_PARAM_ASSIGNS);
    constant!("CURVE_LINEAR", GENERICFX_CURVE_LINEAR);
    constant!("CURVE_EXP", GENERICFX_CURVE_EXP);
    constant!("CURVE_LOG", GENERICFX_CURVE_LOG);
    constant!("CURVE_TOGGLE", GENERICFX_CURVE_TOGGLE);
    constant!("CURVE_MINCLIP", GENERICFX_CURVE_MINCLIP);
    constant!("CURVE_MAXCLIP", GENERICFX_CURVE_MAXCLIP);
    constant!("CURVE_COUNT", NUM_GENERICFX_CURVES);
    constant!("CURVE_UNIPOLAR", GENERICFX_CURVE_UNIPOLAR);
    constant!("CURVE_BIPOLAR", GENERICFX_CURVE_BIPOLAR);
    constant!("CURVE_POLARITY_COUNT", NUM_GENERICFX_CURVE_POLARITIES);

    constant!("TOUCH_BEGAN", UNIT_TOUCH_PHASE_BEGAN);
    constant!("TOUCH_MOVED", UNIT_TOUCH_PHASE_MOVED);
    constant!("TOUCH_ENDED", UNIT_TOUCH_PHASE_ENDED);
    constant!("TOUCH_STATIONARY", UNIT_TOUCH_PHASE_STATIONARY);
    constant!("TOUCH_CANCELLED", UNIT_TOUCH_PHASE_CANCELLED);
    constant!("TOUCH_COUNT", NUM_UNIT_TOUCH_PHASES);

    constant!("ERR_NONE", UNIT_ERR_NONE);
    constant!("ERR_TARGET", UNIT_ERR_TARGET);
    constant!("ERR_API_VERSION", UNIT_ERR_API_VERSION);
    constant!("ERR_SAMPLERATE", UNIT_ERR_SAMPLERATE);
    constant!("ERR_GEOMETRY", UNIT_ERR_GEOMETRY);
    constant!("ERR_MEMORY", UNIT_ERR_MEMORY);
    constant!("ERR_UNDEF", UNIT_ERR_UNDEF);

    signature!("sdram_alloc", UnitRuntimeSdramAllocFn);
    signature!("sdram_free", UnitRuntimeSdramFreeFn);
    signature!("sdram_avail", UnitRuntimeSdramAvailFn);
    signature!("raw_input", UnitRuntimeGenericfxGetRawInputFn);
    signature!("unit_init", UnitInitFn);
    signature!("unit_teardown", UnitTeardownFn);
    signature!("unit_reset", UnitResetFn);
    signature!("unit_resume", UnitResumeFn);
    signature!("unit_suspend", UnitSuspendFn);
    signature!("unit_render", UnitRenderFn);
    signature!("get_param", UnitGetParamValueFn);
    signature!("get_param_string", UnitGetParamStringValueFn);
    signature!("set_param", UnitSetParamValueFn);
    signature!("set_tempo", UnitSetTempoFn);
    signature!("tempo_tick", UnitTempo4ppqnTickFn);
    signature!("touch_event", UnitTouchEventFn);

    let header = dummy_header();
    // SAFETY: `GenericfxUnitHeader` is packed and consists entirely of
    // initialized byte/integer fields with no padding. The slice has byte
    // alignment and cannot outlive this local value.
    let bytes = unsafe {
        core::slice::from_raw_parts(
            core::ptr::addr_of!(header).cast::<u8>(),
            size_of::<GenericfxUnitHeader>(),
        )
    };
    print!("HEADER.bytes=");
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

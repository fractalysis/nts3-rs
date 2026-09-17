#![no_std]

use core::ffi::{c_char, c_void};
#[cfg(not(test))]
use core::panic::PanicInfo;
use core::ptr;

const TARGET_NTS3_GENERICFX: u32 = (6 << 8) | 7;
const API_2_0_0: u32 = 2 << 16;
const SAMPLE_RATE: u32 = 48_000;
const STEREO_CHANNELS: u8 = 2;

const ERR_NONE: i8 = 0;
const ERR_TARGET: i8 = -1;
const ERR_API_VERSION: i8 = -2;
const ERR_SAMPLE_RATE: i8 = -4;
const ERR_GEOMETRY: i8 = -8;
const ERR_UNDEFINED: i8 = -32;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct UnitParam {
    min: i16,
    max: i16,
    center: i16,
    init: i16,
    parameter_type: u8,
    fraction: u8,
    name: [u8; 22],
}

const UNUSED_PARAM: UnitParam = UnitParam {
    min: 0,
    max: 0,
    center: 0,
    init: 0,
    parameter_type: 0,
    fraction: 0,
    name: [0; 22],
};

const TEST_PARAM: UnitParam = UnitParam {
    min: 0,
    max: 1,
    center: 0,
    init: 0,
    parameter_type: 0,
    fraction: 0,
    name: *b"TEST\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
};

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct GenericFxParamMapping {
    assign: u8,
    curve_and_polarity: u8,
    min: i16,
    max: i16,
    value: i16,
}

const UNUSED_MAPPING: GenericFxParamMapping = GenericFxParamMapping {
    assign: 0,
    curve_and_polarity: 0,
    min: 0,
    max: 0,
    value: 0,
};

const TEST_MAPPING: GenericFxParamMapping = GenericFxParamMapping {
    assign: 0,
    curve_and_polarity: 0,
    min: 0,
    max: 1,
    value: 0,
};

#[repr(C, packed)]
struct UnitHeader {
    header_size: u32,
    target: u32,
    api: u32,
    developer_id: u32,
    unit_id: u32,
    version: u32,
    name: [u8; 20],
    reserved0: u32,
    reserved1: u32,
    num_params: u32,
    params: [UnitParam; 8],
    default_mappings: [GenericFxParamMapping; 8],
}

const _: () = assert!(core::mem::size_of::<UnitParam>() == 32);
const _: () = assert!(core::mem::size_of::<GenericFxParamMapping>() == 8);
const _: () = assert!(core::mem::size_of::<UnitHeader>() == 376);
const _: () = assert!(core::mem::align_of::<UnitHeader>() == 1);

const UNIT_NAME: [u8; 20] = *b"Rust Pass Through\0\0\0";

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".unit_header")]
static unit_header: UnitHeader = UnitHeader {
    header_size: core::mem::size_of::<UnitHeader>() as u32,
    target: TARGET_NTS3_GENERICFX,
    api: API_2_0_0,
    developer_id: 0x5255_5354,
    unit_id: 0x5041_5353,
    version: 0x0001_0000,
    name: UNIT_NAME,
    reserved0: 0,
    reserved1: 0,
    num_params: 1,
    params: [
        TEST_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
        UNUSED_PARAM,
    ],
    default_mappings: [
        TEST_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
    ],
};

#[repr(C)]
#[derive(Clone, Copy)]
struct RuntimeHooks {
    runtime_context: *const c_void,
    sdram_alloc: Option<unsafe extern "C" fn(usize) -> *mut u8>,
    sdram_free: Option<unsafe extern "C" fn(*const u8)>,
    sdram_avail: Option<unsafe extern "C" fn() -> usize>,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RuntimeDescriptor {
    target: u32,
    api: u32,
    sample_rate: u32,
    frames_per_buffer: u16,
    input_channels: u8,
    output_channels: u8,
    hooks: RuntimeHooks,
}

#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<RuntimeHooks>() == 16);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<RuntimeDescriptor>() == 32);

/// # Safety
/// `descriptor` must point to a readable SDK runtime descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unit_init(descriptor: *const RuntimeDescriptor) -> i8 {
    if descriptor.is_null() {
        return ERR_UNDEFINED;
    }

    // SAFETY: the runtime promises a readable descriptor pointer for this call.
    // Its C type is packed, so an explicitly unaligned copy is required.
    let descriptor = unsafe { descriptor.read_unaligned() };

    if descriptor.target != TARGET_NTS3_GENERICFX {
        return ERR_TARGET;
    }
    let api_major = descriptor.api & 0x007f_0000;
    let api_minor = descriptor.api & 0x0000_7f00;
    if api_major != API_2_0_0 || api_minor != 0 {
        return ERR_API_VERSION;
    }
    if descriptor.sample_rate != SAMPLE_RATE {
        return ERR_SAMPLE_RATE;
    }
    if descriptor.input_channels != STEREO_CHANNELS || descriptor.output_channels != STEREO_CHANNELS
    {
        return ERR_GEOMETRY;
    }

    ERR_NONE
}

#[unsafe(no_mangle)]
pub extern "C" fn unit_teardown() {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_reset() {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_resume() {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_suspend() {}

/// # Safety
/// The pointers must describe SDK-provided stereo buffers containing `frames`
/// frames. They may be disjoint or exactly equal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unit_render(input: *const f32, output: *mut f32, frames: u32) {
    let Some(samples) = (frames as usize).checked_mul(2) else {
        return;
    };
    if samples == 0 || input.is_null() || output.is_null() {
        return;
    }

    for index in 0..samples {
        // SAFETY: the NTS-3 ABI guarantees readable/writable stereo buffers for
        // `frames` frames. Reading before writing supports exact in-place I/O.
        let sample = unsafe { ptr::read(input.add(index)) };
        // SAFETY: each output sample is in bounds and is written exactly once.
        unsafe { ptr::write(output.add(index), sample) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn unit_get_param_value(_id: u8) -> i32 {
    0
}

static EMPTY_STRING: [c_char; 1] = [0];

#[unsafe(no_mangle)]
pub extern "C" fn unit_get_param_str_value(_id: u8, _value: i32) -> *const c_char {
    EMPTY_STRING.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn unit_set_param_value(_id: u8, _value: i32) {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_set_tempo(_tempo: u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_tempo_4ppqn_tick(_counter: u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn unit_touch_event(_id: u8, _phase: u8, _x: u32, _y: u32) {}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> RuntimeDescriptor {
        RuntimeDescriptor {
            target: TARGET_NTS3_GENERICFX,
            api: API_2_0_0,
            sample_rate: SAMPLE_RATE,
            frames_per_buffer: 64,
            input_channels: STEREO_CHANNELS,
            output_channels: STEREO_CHANNELS,
            hooks: RuntimeHooks {
                runtime_context: ptr::null(),
                sdram_alloc: None,
                sdram_free: None,
                sdram_avail: None,
            },
        }
    }

    #[test]
    fn validates_runtime_descriptor() {
        let valid = descriptor();
        // SAFETY: `valid` remains readable for the duration of the call.
        assert_eq!(unsafe { unit_init(&valid) }, ERR_NONE);

        let mut wrong_api = descriptor();
        wrong_api.api = (2 << 16) | (1 << 8);
        // SAFETY: `wrong_api` remains readable for the duration of the call.
        assert_eq!(unsafe { unit_init(&wrong_api) }, ERR_API_VERSION);

        // SAFETY: null is explicitly accepted and rejected before dereference.
        assert_eq!(unsafe { unit_init(ptr::null()) }, ERR_UNDEFINED);
    }

    #[test]
    fn copies_separate_and_in_place_stereo_buffers() {
        let input = [0.25, -0.5, 0.75, -1.0];
        let mut output = [0.0; 4];
        // SAFETY: both arrays contain two readable/writable stereo frames.
        unsafe { unit_render(input.as_ptr(), output.as_mut_ptr(), 2) };
        assert_eq!(output, input);

        let mut in_place = input;
        // SAFETY: exact input/output overlap is explicitly supported by the ABI.
        unsafe { unit_render(in_place.as_ptr(), in_place.as_mut_ptr(), 2) };
        assert_eq!(in_place, input);
    }
}

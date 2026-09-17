#![no_std]

#[cfg(not(test))]
use core::panic::PanicInfo;
use core::ptr;
#[cfg(test)]
use nts3_sys::UnitRuntimeHooks;
use nts3_sys::{
    GenericfxCurve, GenericfxParamMapping, GenericfxUnitHeader, UNIT_API_2_0_0,
    UNIT_ERR_API_VERSION, UNIT_ERR_GEOMETRY, UNIT_ERR_NONE, UNIT_ERR_SAMPLERATE, UNIT_ERR_TARGET,
    UNIT_ERR_UNDEF, UNIT_PARAM_TYPE_NONE, UNIT_TARGET_NTS3_KAOSS_GENERICFX, UNUSED_MAPPING,
    UNUSED_PARAM, UnitHeader, UnitParam, UnitParamFormat, UnitRuntimeDescriptor,
};

const SAMPLE_RATE: u32 = 48_000;
const STEREO_CHANNELS: u8 = 2;

const TEST_PARAM: UnitParam = UnitParam::new(
    0,
    1,
    0,
    0,
    UNIT_PARAM_TYPE_NONE,
    UnitParamFormat::FIXED_ZERO,
    *b"TEST\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
);

const TEST_MAPPING: GenericfxParamMapping =
    GenericfxParamMapping::new(0, GenericfxCurve::LINEAR_UNIPOLAR, 0, 1, 0);

const UNIT_NAME: [u8; 20] = *b"Rust Pass Through\0\0\0";

#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".unit_header")]
static unit_header: GenericfxUnitHeader = GenericfxUnitHeader::new(
    UnitHeader::new(
        core::mem::size_of::<GenericfxUnitHeader>() as u32,
        UNIT_TARGET_NTS3_KAOSS_GENERICFX,
        UNIT_API_2_0_0,
        0x5255_5354,
        0x5041_5353,
        0x0001_0000,
        UNIT_NAME,
        1,
        [
            TEST_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
            UNUSED_PARAM,
        ],
    ),
    [
        TEST_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
        UNUSED_MAPPING,
    ],
);

/// # Safety
/// `descriptor` must point to a readable SDK runtime descriptor for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unit_init(descriptor: *const UnitRuntimeDescriptor) -> i8 {
    if descriptor.is_null() {
        return UNIT_ERR_UNDEF;
    }

    // SAFETY: the runtime promises a readable descriptor pointer for this call.
    // Its C type is packed, so an explicitly unaligned copy is required.
    let descriptor = unsafe { descriptor.read_unaligned() };

    if descriptor.target() != UNIT_TARGET_NTS3_KAOSS_GENERICFX {
        return UNIT_ERR_TARGET;
    }
    let api_major = descriptor.api() & 0x007f_0000;
    let api_minor = descriptor.api() & 0x0000_7f00;
    if api_major != UNIT_API_2_0_0 || api_minor != 0 {
        return UNIT_ERR_API_VERSION;
    }
    if descriptor.sample_rate() != SAMPLE_RATE {
        return UNIT_ERR_SAMPLERATE;
    }
    if descriptor.input_channels() != STEREO_CHANNELS
        || descriptor.output_channels() != STEREO_CHANNELS
    {
        return UNIT_ERR_GEOMETRY;
    }

    UNIT_ERR_NONE
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

static EMPTY_STRING: [core::ffi::c_char; 1] = [0];

#[unsafe(no_mangle)]
pub extern "C" fn unit_get_param_str_value(_id: u8, _value: i32) -> *const core::ffi::c_char {
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

    fn descriptor() -> UnitRuntimeDescriptor {
        UnitRuntimeDescriptor::new(
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            UNIT_API_2_0_0,
            SAMPLE_RATE,
            64,
            STEREO_CHANNELS,
            STEREO_CHANNELS,
            UnitRuntimeHooks::new(ptr::null(), None, None, None),
        )
    }

    #[test]
    fn validates_runtime_descriptor() {
        let valid = descriptor();
        // SAFETY: `valid` remains readable for the duration of the call.
        assert_eq!(unsafe { unit_init(&valid) }, UNIT_ERR_NONE);

        let wrong_api = UnitRuntimeDescriptor::new(
            UNIT_TARGET_NTS3_KAOSS_GENERICFX,
            (2 << 16) | (1 << 8),
            SAMPLE_RATE,
            64,
            STEREO_CHANNELS,
            STEREO_CHANNELS,
            UnitRuntimeHooks::new(ptr::null(), None, None, None),
        );
        // SAFETY: `wrong_api` remains readable for the duration of the call.
        assert_eq!(unsafe { unit_init(&wrong_api) }, UNIT_ERR_API_VERSION);

        // SAFETY: null is explicitly accepted and rejected before dereference.
        assert_eq!(unsafe { unit_init(ptr::null()) }, UNIT_ERR_UNDEF);
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

// BSD 3-Clause License
//
// Copyright (c) 2018-2023, KORG INC.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the conditions in
// ../LICENSE-KORG-BSD-3-Clause are met.
//
// This file is a Rust representation of the raw ABI declarations in Korg's
// NTS-3 SDK API 2.0 runtime.h and unit_genericfx.h. Names are made idiomatic,
// but values and layouts are kept byte-for-byte compatible.

#![no_std]

use core::ffi::{c_char, c_void};

pub const UNIT_MODULE_GLOBAL: u32 = 0;
pub const UNIT_MODULE_MODFX: u32 = 1;
pub const UNIT_MODULE_DELFX: u32 = 2;
pub const UNIT_MODULE_REVFX: u32 = 3;
pub const UNIT_MODULE_OSC: u32 = 4;
pub const UNIT_MODULE_SYNTH: u32 = 5;
pub const UNIT_MODULE_MASTERFX: u32 = 6;
pub const UNIT_MODULE_GENERICFX: u32 = 7;
pub const NUM_UNIT_MODULES: u32 = 8;

pub const UNIT_TARGET_NTS3_KAOSS: u32 = 6 << 8;
pub const UNIT_TARGET_NTS3_KAOSS_GLOBAL: u32 = UNIT_TARGET_NTS3_KAOSS | UNIT_MODULE_GLOBAL;
pub const UNIT_TARGET_NTS3_KAOSS_GENERICFX: u32 = UNIT_TARGET_NTS3_KAOSS | UNIT_MODULE_GENERICFX;
pub const UNIT_TARGET_PLATFORM: u32 = UNIT_TARGET_NTS3_KAOSS;
pub const UNIT_TARGET_PLATFORM_MASK: u32 = 0x7f << 8;
pub const UNIT_TARGET_MODULE_MASK: u32 = 0x7f;

#[inline]
pub const fn unit_target_platform_is_compatible(target: u32) -> bool {
    target & UNIT_TARGET_PLATFORM_MASK == UNIT_TARGET_NTS3_KAOSS
}

pub const UNIT_API_1_0_0: u32 = 1 << 16;
pub const UNIT_API_1_1_0: u32 = (1 << 16) | (1 << 8);
pub const UNIT_API_2_0_0: u32 = 2 << 16;
pub const UNIT_API_VERSION: u32 = UNIT_API_2_0_0;
pub const UNIT_API_MAJOR_MASK: u32 = 0x7f << 16;
pub const UNIT_API_MINOR_MASK: u32 = 0x7f << 8;
pub const UNIT_API_PATCH_MASK: u32 = 0x7f;

#[inline]
pub const fn unit_api_major(version: u32) -> u32 {
    (version >> 16) & 0x7f
}

#[inline]
pub const fn unit_api_minor(version: u32) -> u32 {
    (version >> 8) & 0x7f
}

#[inline]
pub const fn unit_api_patch(version: u32) -> u32 {
    version & 0x7f
}

#[inline]
pub const fn unit_api_is_compatible(api: u32) -> bool {
    api & UNIT_API_MAJOR_MASK == UNIT_API_VERSION & UNIT_API_MAJOR_MASK
        && api & UNIT_API_MINOR_MASK == UNIT_API_VERSION & UNIT_API_MINOR_MASK
}

pub const UNIT_MAX_PARAM_COUNT: usize = 8;
pub const UNIT_GENERICFX_MAX_PARAM_COUNT: usize = UNIT_MAX_PARAM_COUNT;
pub const UNIT_PARAM_NAME_LEN: usize = 21;
pub const UNIT_PARAM_NAME_SIZE: usize = UNIT_PARAM_NAME_LEN + 1;
pub const UNIT_NAME_LEN: usize = 19;
pub const UNIT_NAME_SIZE: usize = UNIT_NAME_LEN + 1;
pub const NUM_UNIT_GENERICFX_FIXED_PARAM_IDS: u32 = 0;

pub const UNIT_PARAM_TYPE_NONE: u8 = 0;
pub const UNIT_PARAM_TYPE_PERCENT: u8 = 1;
pub const UNIT_PARAM_TYPE_DB: u8 = 2;
pub const UNIT_PARAM_TYPE_CENTS: u8 = 3;
pub const UNIT_PARAM_TYPE_SEMI: u8 = 4;
pub const UNIT_PARAM_TYPE_OCT: u8 = 5;
pub const UNIT_PARAM_TYPE_HERTZ: u8 = 6;
pub const UNIT_PARAM_TYPE_KHERTZ: u8 = 7;
pub const UNIT_PARAM_TYPE_BPM: u8 = 8;
pub const UNIT_PARAM_TYPE_MSEC: u8 = 9;
pub const UNIT_PARAM_TYPE_SEC: u8 = 10;
pub const UNIT_PARAM_TYPE_ENUM: u8 = 11;
pub const UNIT_PARAM_TYPE_STRINGS: u8 = 12;
pub const UNIT_PARAM_TYPE_RESERVED0: u8 = 13;
pub const UNIT_PARAM_TYPE_DRYWET: u8 = 14;
pub const UNIT_PARAM_TYPE_PAN: u8 = 15;
pub const UNIT_PARAM_TYPE_SPREAD: u8 = 16;
pub const UNIT_PARAM_TYPE_ONOFF: u8 = 17;
pub const UNIT_PARAM_TYPE_MIDI_NOTE: u8 = 18;
pub const UNIT_PARAM_TYPE_COUNT: u8 = 19;

pub const UNIT_PARAM_FRAC_MODE_FIXED: u8 = 0;
pub const UNIT_PARAM_FRAC_MODE_DECIMAL: u8 = 1;

pub const GENERICFX_PARAM_ASSIGN_NONE: u8 = 0;
pub const GENERICFX_PARAM_ASSIGN_X: u8 = 1;
pub const GENERICFX_PARAM_ASSIGN_Y: u8 = 2;
pub const GENERICFX_PARAM_ASSIGN_DEPTH: u8 = 3;
pub const NUM_GENERICFX_PARAM_ASSIGNS: u8 = 4;

pub const GENERICFX_CURVE_LINEAR: u8 = 0;
pub const GENERICFX_CURVE_EXP: u8 = 1;
pub const GENERICFX_CURVE_LOG: u8 = 2;
pub const GENERICFX_CURVE_TOGGLE: u8 = 3;
pub const GENERICFX_CURVE_MINCLIP: u8 = 4;
pub const GENERICFX_CURVE_MAXCLIP: u8 = 5;
pub const NUM_GENERICFX_CURVES: u8 = 6;

pub const GENERICFX_CURVE_UNIPOLAR: u8 = 0;
pub const GENERICFX_CURVE_BIPOLAR: u8 = 1;
pub const NUM_GENERICFX_CURVE_POLARITIES: u8 = 2;

pub const UNIT_TOUCH_PHASE_BEGAN: u8 = 0;
pub const UNIT_TOUCH_PHASE_MOVED: u8 = 1;
pub const UNIT_TOUCH_PHASE_ENDED: u8 = 2;
pub const UNIT_TOUCH_PHASE_STATIONARY: u8 = 3;
pub const UNIT_TOUCH_PHASE_CANCELLED: u8 = 4;
pub const NUM_UNIT_TOUCH_PHASES: u8 = 5;

pub const UNIT_ERR_NONE: i8 = 0;
pub const UNIT_ERR_TARGET: i8 = -1;
pub const UNIT_ERR_API_VERSION: i8 = -2;
pub const UNIT_ERR_SAMPLERATE: i8 = -4;
pub const UNIT_ERR_GEOMETRY: i8 = -8;
pub const UNIT_ERR_MEMORY: i8 = -16;
pub const UNIT_ERR_UNDEF: i8 = -32;

pub type UnitRuntimeBaseContext = c_void;
pub type UnitRuntimeSdramAllocFn = unsafe extern "C" fn(usize) -> *mut u8;
pub type UnitRuntimeSdramFreeFn = unsafe extern "C" fn(*const u8);
pub type UnitRuntimeSdramAvailFn = unsafe extern "C" fn() -> usize;
pub type UnitRuntimeGenericfxGetRawInputFn = unsafe extern "C" fn() -> *const f32;

pub type UnitInitFn = unsafe extern "C" fn(*const UnitRuntimeDescriptor) -> i8;
pub type UnitTeardownFn = unsafe extern "C" fn();
pub type UnitResetFn = unsafe extern "C" fn();
pub type UnitResumeFn = unsafe extern "C" fn();
pub type UnitSuspendFn = unsafe extern "C" fn();
pub type UnitRenderFn = unsafe extern "C" fn(*const f32, *mut f32, u32);
pub type UnitGetParamValueFn = unsafe extern "C" fn(u8) -> i32;
pub type UnitGetParamStringValueFn = unsafe extern "C" fn(u8, i32) -> *const c_char;
pub type UnitSetParamValueFn = unsafe extern "C" fn(u8, i32);
pub type UnitSetTempoFn = unsafe extern "C" fn(u32);
pub type UnitTempo4ppqnTickFn = unsafe extern "C" fn(u32);
pub type UnitTouchEventFn = unsafe extern "C" fn(u8, u8, u32, u32);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnitRuntimeHooks {
    runtime_context: *const UnitRuntimeBaseContext,
    sdram_alloc: Option<UnitRuntimeSdramAllocFn>,
    sdram_free: Option<UnitRuntimeSdramFreeFn>,
    sdram_avail: Option<UnitRuntimeSdramAvailFn>,
}

impl UnitRuntimeHooks {
    pub const fn new(
        runtime_context: *const UnitRuntimeBaseContext,
        sdram_alloc: Option<UnitRuntimeSdramAllocFn>,
        sdram_free: Option<UnitRuntimeSdramFreeFn>,
        sdram_avail: Option<UnitRuntimeSdramAvailFn>,
    ) -> Self {
        Self {
            runtime_context,
            sdram_alloc,
            sdram_free,
            sdram_avail,
        }
    }

    pub const fn runtime_context(self) -> *const UnitRuntimeBaseContext {
        self.runtime_context
    }

    pub const fn sdram_alloc(self) -> Option<UnitRuntimeSdramAllocFn> {
        self.sdram_alloc
    }

    pub const fn sdram_free(self) -> Option<UnitRuntimeSdramFreeFn> {
        self.sdram_free
    }

    pub const fn sdram_avail(self) -> Option<UnitRuntimeSdramAvailFn> {
        self.sdram_avail
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct UnitRuntimeDescriptor {
    target: u32,
    api: u32,
    sample_rate: u32,
    frames_per_buffer: u16,
    input_channels: u8,
    output_channels: u8,
    hooks: UnitRuntimeHooks,
}

impl UnitRuntimeDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        target: u32,
        api: u32,
        sample_rate: u32,
        frames_per_buffer: u16,
        input_channels: u8,
        output_channels: u8,
        hooks: UnitRuntimeHooks,
    ) -> Self {
        Self {
            target,
            api,
            sample_rate,
            frames_per_buffer,
            input_channels,
            output_channels,
            hooks,
        }
    }

    pub const fn target(&self) -> u32 {
        self.target
    }

    pub const fn api(&self) -> u32 {
        self.api
    }

    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub const fn frames_per_buffer(&self) -> u16 {
        self.frames_per_buffer
    }

    pub const fn input_channels(&self) -> u8 {
        self.input_channels
    }

    pub const fn output_channels(&self) -> u8 {
        self.output_channels
    }

    pub const fn hooks(&self) -> UnitRuntimeHooks {
        self.hooks
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnitRuntimeGenericfxContext {
    touch_area_width: u32,
    touch_area_height: u32,
    get_raw_input: Option<UnitRuntimeGenericfxGetRawInputFn>,
}

impl UnitRuntimeGenericfxContext {
    pub const fn new(
        touch_area_width: u32,
        touch_area_height: u32,
        get_raw_input: Option<UnitRuntimeGenericfxGetRawInputFn>,
    ) -> Self {
        Self {
            touch_area_width,
            touch_area_height,
            get_raw_input,
        }
    }

    pub const fn touch_area_width(self) -> u32 {
        self.touch_area_width
    }

    pub const fn touch_area_height(self) -> u32 {
        self.touch_area_height
    }

    pub const fn get_raw_input(self) -> Option<UnitRuntimeGenericfxGetRawInputFn> {
        self.get_raw_input
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct UnitParamFormat(u8);

impl UnitParamFormat {
    pub const FIXED_ZERO: Self = Self(0);

    pub const fn new(fraction: u8, fraction_mode: u8) -> Option<Self> {
        if fraction <= 0x0f && fraction_mode <= 1 {
            Some(Self(fraction | (fraction_mode << 4)))
        } else {
            None
        }
    }

    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn fraction(self) -> u8 {
        self.0 & 0x0f
    }

    pub const fn fraction_mode(self) -> u8 {
        (self.0 >> 4) & 1
    }

    pub const fn reserved(self) -> u8 {
        self.0 >> 5
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct UnitParam {
    min: i16,
    max: i16,
    center: i16,
    init: i16,
    parameter_type: u8,
    format: UnitParamFormat,
    name: [u8; UNIT_PARAM_NAME_SIZE],
}

impl UnitParam {
    pub const fn new(
        min: i16,
        max: i16,
        center: i16,
        init: i16,
        parameter_type: u8,
        format: UnitParamFormat,
        name: [u8; UNIT_PARAM_NAME_SIZE],
    ) -> Self {
        Self {
            min,
            max,
            center,
            init,
            parameter_type,
            format,
            name,
        }
    }

    pub const fn min(&self) -> i16 {
        self.min
    }

    pub const fn max(&self) -> i16 {
        self.max
    }

    pub const fn center(&self) -> i16 {
        self.center
    }

    pub const fn init(&self) -> i16 {
        self.init
    }

    pub const fn parameter_type(&self) -> u8 {
        self.parameter_type
    }

    pub const fn format(&self) -> UnitParamFormat {
        self.format
    }

    pub const fn name(&self) -> [u8; UNIT_PARAM_NAME_SIZE] {
        self.name
    }
}

pub const UNUSED_PARAM: UnitParam = UnitParam::new(
    0,
    0,
    0,
    0,
    UNIT_PARAM_TYPE_NONE,
    UnitParamFormat::FIXED_ZERO,
    [0; UNIT_PARAM_NAME_SIZE],
);

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct GenericfxCurve(u8);

impl GenericfxCurve {
    pub const LINEAR_UNIPOLAR: Self = Self(0);

    pub const fn new(curve: u8, polarity: u8) -> Option<Self> {
        if curve <= 0x7f && polarity <= 1 {
            Some(Self(curve | (polarity << 7)))
        } else {
            None
        }
    }

    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn curve(self) -> u8 {
        self.0 & 0x7f
    }

    pub const fn polarity(self) -> u8 {
        self.0 >> 7
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GenericfxParamMapping {
    assign: u8,
    curve: GenericfxCurve,
    min: i16,
    max: i16,
    value: i16,
}

impl GenericfxParamMapping {
    pub const fn new(assign: u8, curve: GenericfxCurve, min: i16, max: i16, value: i16) -> Self {
        Self {
            assign,
            curve,
            min,
            max,
            value,
        }
    }

    pub const fn assign(&self) -> u8 {
        self.assign
    }

    pub const fn curve(&self) -> GenericfxCurve {
        self.curve
    }

    pub const fn min(&self) -> i16 {
        self.min
    }

    pub const fn max(&self) -> i16 {
        self.max
    }

    pub const fn value(&self) -> i16 {
        self.value
    }
}

pub const UNUSED_MAPPING: GenericfxParamMapping = GenericfxParamMapping::new(
    GENERICFX_PARAM_ASSIGN_NONE,
    GenericfxCurve::LINEAR_UNIPOLAR,
    0,
    0,
    0,
);

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct UnitHeader {
    header_size: u32,
    target: u32,
    api: u32,
    developer_id: u32,
    unit_id: u32,
    version: u32,
    name: [u8; UNIT_NAME_SIZE],
    reserved0: u32,
    reserved1: u32,
    num_params: u32,
    params: [UnitParam; UNIT_MAX_PARAM_COUNT],
}

impl UnitHeader {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        header_size: u32,
        target: u32,
        api: u32,
        developer_id: u32,
        unit_id: u32,
        version: u32,
        name: [u8; UNIT_NAME_SIZE],
        num_params: u32,
        params: [UnitParam; UNIT_MAX_PARAM_COUNT],
    ) -> Self {
        Self {
            header_size,
            target,
            api,
            developer_id,
            unit_id,
            version,
            name,
            reserved0: 0,
            reserved1: 0,
            num_params,
            params,
        }
    }

    pub const fn header_size(&self) -> u32 {
        self.header_size
    }

    pub const fn target(&self) -> u32 {
        self.target
    }

    pub const fn api(&self) -> u32 {
        self.api
    }

    pub const fn developer_id(&self) -> u32 {
        self.developer_id
    }

    pub const fn unit_id(&self) -> u32 {
        self.unit_id
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn name(&self) -> [u8; UNIT_NAME_SIZE] {
        self.name
    }

    pub const fn reserved0(&self) -> u32 {
        self.reserved0
    }

    pub const fn reserved1(&self) -> u32 {
        self.reserved1
    }

    pub const fn num_params(&self) -> u32 {
        self.num_params
    }

    pub const fn params(&self) -> [UnitParam; UNIT_MAX_PARAM_COUNT] {
        self.params
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GenericfxUnitHeader {
    common: UnitHeader,
    default_mappings: [GenericfxParamMapping; UNIT_GENERICFX_MAX_PARAM_COUNT],
}

impl GenericfxUnitHeader {
    pub const fn new(
        common: UnitHeader,
        default_mappings: [GenericfxParamMapping; UNIT_GENERICFX_MAX_PARAM_COUNT],
    ) -> Self {
        Self {
            common,
            default_mappings,
        }
    }

    pub const fn common(&self) -> UnitHeader {
        self.common
    }

    pub const fn default_mappings(
        &self,
    ) -> [GenericfxParamMapping; UNIT_GENERICFX_MAX_PARAM_COUNT] {
        self.default_mappings
    }
}

/// Field offsets used by the independent C/Rust ABI probe.
///
/// Keeping these in this crate allows the packed fields themselves to remain
/// private, preventing downstream code from accidentally borrowing an
/// unaligned field.
pub mod layout {
    use super::*;
    use core::mem::offset_of;

    pub const RUNTIME_HOOKS_CONTEXT: usize = offset_of!(UnitRuntimeHooks, runtime_context);
    pub const RUNTIME_HOOKS_ALLOC: usize = offset_of!(UnitRuntimeHooks, sdram_alloc);
    pub const RUNTIME_HOOKS_FREE: usize = offset_of!(UnitRuntimeHooks, sdram_free);
    pub const RUNTIME_HOOKS_AVAIL: usize = offset_of!(UnitRuntimeHooks, sdram_avail);

    pub const RUNTIME_DESC_TARGET: usize = offset_of!(UnitRuntimeDescriptor, target);
    pub const RUNTIME_DESC_API: usize = offset_of!(UnitRuntimeDescriptor, api);
    pub const RUNTIME_DESC_SAMPLE_RATE: usize = offset_of!(UnitRuntimeDescriptor, sample_rate);
    pub const RUNTIME_DESC_FRAMES: usize = offset_of!(UnitRuntimeDescriptor, frames_per_buffer);
    pub const RUNTIME_DESC_INPUTS: usize = offset_of!(UnitRuntimeDescriptor, input_channels);
    pub const RUNTIME_DESC_OUTPUTS: usize = offset_of!(UnitRuntimeDescriptor, output_channels);
    pub const RUNTIME_DESC_HOOKS: usize = offset_of!(UnitRuntimeDescriptor, hooks);

    pub const GENERICFX_CONTEXT_WIDTH: usize =
        offset_of!(UnitRuntimeGenericfxContext, touch_area_width);
    pub const GENERICFX_CONTEXT_HEIGHT: usize =
        offset_of!(UnitRuntimeGenericfxContext, touch_area_height);
    pub const GENERICFX_CONTEXT_RAW_INPUT: usize =
        offset_of!(UnitRuntimeGenericfxContext, get_raw_input);

    pub const PARAM_MIN: usize = offset_of!(UnitParam, min);
    pub const PARAM_MAX: usize = offset_of!(UnitParam, max);
    pub const PARAM_CENTER: usize = offset_of!(UnitParam, center);
    pub const PARAM_INIT: usize = offset_of!(UnitParam, init);
    pub const PARAM_TYPE: usize = offset_of!(UnitParam, parameter_type);
    pub const PARAM_FORMAT: usize = offset_of!(UnitParam, format);
    pub const PARAM_NAME: usize = offset_of!(UnitParam, name);

    pub const HEADER_SIZE: usize = offset_of!(UnitHeader, header_size);
    pub const HEADER_TARGET: usize = offset_of!(UnitHeader, target);
    pub const HEADER_API: usize = offset_of!(UnitHeader, api);
    pub const HEADER_DEV_ID: usize = offset_of!(UnitHeader, developer_id);
    pub const HEADER_UNIT_ID: usize = offset_of!(UnitHeader, unit_id);
    pub const HEADER_VERSION: usize = offset_of!(UnitHeader, version);
    pub const HEADER_NAME: usize = offset_of!(UnitHeader, name);
    pub const HEADER_RESERVED0: usize = offset_of!(UnitHeader, reserved0);
    pub const HEADER_RESERVED1: usize = offset_of!(UnitHeader, reserved1);
    pub const HEADER_NUM_PARAMS: usize = offset_of!(UnitHeader, num_params);
    pub const HEADER_PARAMS: usize = offset_of!(UnitHeader, params);

    pub const MAPPING_ASSIGN: usize = offset_of!(GenericfxParamMapping, assign);
    pub const MAPPING_CURVE: usize = offset_of!(GenericfxParamMapping, curve);
    pub const MAPPING_MIN: usize = offset_of!(GenericfxParamMapping, min);
    pub const MAPPING_MAX: usize = offset_of!(GenericfxParamMapping, max);
    pub const MAPPING_VALUE: usize = offset_of!(GenericfxParamMapping, value);

    pub const GENERIC_HEADER_COMMON: usize = offset_of!(GenericfxUnitHeader, common);
    pub const GENERIC_HEADER_MAPPINGS: usize = offset_of!(GenericfxUnitHeader, default_mappings);
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};

    assert!(size_of::<UnitParamFormat>() == 1);
    assert!(align_of::<UnitParamFormat>() == 1);
    assert!(size_of::<UnitParam>() == 32);
    assert!(align_of::<UnitParam>() == 1);
    assert!(offset_of!(UnitParam, min) == 0);
    assert!(offset_of!(UnitParam, max) == 2);
    assert!(offset_of!(UnitParam, center) == 4);
    assert!(offset_of!(UnitParam, init) == 6);
    assert!(offset_of!(UnitParam, parameter_type) == 8);
    assert!(offset_of!(UnitParam, format) == 9);
    assert!(offset_of!(UnitParam, name) == 10);

    assert!(size_of::<GenericfxCurve>() == 1);
    assert!(align_of::<GenericfxCurve>() == 1);
    assert!(size_of::<GenericfxParamMapping>() == 8);
    assert!(align_of::<GenericfxParamMapping>() == 1);
    assert!(offset_of!(GenericfxParamMapping, assign) == 0);
    assert!(offset_of!(GenericfxParamMapping, curve) == 1);
    assert!(offset_of!(GenericfxParamMapping, min) == 2);
    assert!(offset_of!(GenericfxParamMapping, max) == 4);
    assert!(offset_of!(GenericfxParamMapping, value) == 6);

    assert!(size_of::<UnitHeader>() == 312);
    assert!(align_of::<UnitHeader>() == 1);
    assert!(offset_of!(UnitHeader, header_size) == 0);
    assert!(offset_of!(UnitHeader, target) == 4);
    assert!(offset_of!(UnitHeader, api) == 8);
    assert!(offset_of!(UnitHeader, developer_id) == 12);
    assert!(offset_of!(UnitHeader, unit_id) == 16);
    assert!(offset_of!(UnitHeader, version) == 20);
    assert!(offset_of!(UnitHeader, name) == 24);
    assert!(offset_of!(UnitHeader, reserved0) == 44);
    assert!(offset_of!(UnitHeader, reserved1) == 48);
    assert!(offset_of!(UnitHeader, num_params) == 52);
    assert!(offset_of!(UnitHeader, params) == 56);

    assert!(size_of::<GenericfxUnitHeader>() == 376);
    assert!(align_of::<GenericfxUnitHeader>() == 1);
    assert!(offset_of!(GenericfxUnitHeader, common) == 0);
    assert!(offset_of!(GenericfxUnitHeader, default_mappings) == 312);

    assert!(size_of::<UnitRuntimeHooks>() == size_of::<usize>() * 4);
    assert!(align_of::<UnitRuntimeHooks>() == align_of::<usize>());
    assert!(offset_of!(UnitRuntimeHooks, runtime_context) == 0);
    assert!(offset_of!(UnitRuntimeHooks, sdram_alloc) == size_of::<usize>());
    assert!(offset_of!(UnitRuntimeHooks, sdram_free) == size_of::<usize>() * 2);
    assert!(offset_of!(UnitRuntimeHooks, sdram_avail) == size_of::<usize>() * 3);

    assert!(size_of::<UnitRuntimeDescriptor>() == 16 + size_of::<UnitRuntimeHooks>());
    assert!(align_of::<UnitRuntimeDescriptor>() == 1);
    assert!(offset_of!(UnitRuntimeDescriptor, target) == 0);
    assert!(offset_of!(UnitRuntimeDescriptor, api) == 4);
    assert!(offset_of!(UnitRuntimeDescriptor, sample_rate) == 8);
    assert!(offset_of!(UnitRuntimeDescriptor, frames_per_buffer) == 12);
    assert!(offset_of!(UnitRuntimeDescriptor, input_channels) == 14);
    assert!(offset_of!(UnitRuntimeDescriptor, output_channels) == 15);
    assert!(offset_of!(UnitRuntimeDescriptor, hooks) == 16);

    assert!(size_of::<UnitRuntimeGenericfxContext>() == 8 + size_of::<usize>());
    assert!(align_of::<UnitRuntimeGenericfxContext>() == align_of::<usize>());
    assert!(offset_of!(UnitRuntimeGenericfxContext, touch_area_width) == 0);
    assert!(offset_of!(UnitRuntimeGenericfxContext, touch_area_height) == 4);
    assert!(offset_of!(UnitRuntimeGenericfxContext, get_raw_input) == 8);
};

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn bitfield_bytes_round_trip() {
        for fraction in 0..=15 {
            for mode in 0..=1 {
                let format = UnitParamFormat::new(fraction, mode).unwrap();
                assert_eq!(format.fraction(), fraction);
                assert_eq!(format.fraction_mode(), mode);
                assert_eq!(format.reserved(), 0);
                assert_eq!(format.raw(), fraction | (mode << 4));
            }
        }
        assert!(UnitParamFormat::new(16, 0).is_none());
        assert!(UnitParamFormat::new(0, 2).is_none());

        for curve in 0..=127 {
            for polarity in 0..=1 {
                let encoded = GenericfxCurve::new(curve, polarity).unwrap();
                assert_eq!(encoded.curve(), curve);
                assert_eq!(encoded.polarity(), polarity);
                assert_eq!(encoded.raw(), curve | (polarity << 7));
            }
        }
        assert!(GenericfxCurve::new(128, 0).is_none());
        assert!(GenericfxCurve::new(0, 2).is_none());
    }

    #[test]
    fn native_pointer_layout_is_consistent() {
        assert_eq!(size_of::<UnitRuntimeHooks>(), size_of::<usize>() * 4);
        assert_eq!(align_of::<UnitRuntimeHooks>(), align_of::<usize>());
        assert_eq!(
            size_of::<UnitRuntimeDescriptor>(),
            16 + size_of::<UnitRuntimeHooks>()
        );
        assert_eq!(
            size_of::<UnitRuntimeGenericfxContext>(),
            if cfg!(target_pointer_width = "64") {
                16
            } else {
                12
            }
        );
    }

    #[test]
    fn dummy_header_serializes_without_padding() {
        const DECIMAL_ONE: UnitParamFormat = UnitParamFormat::from_raw(0x11);
        const EXP_BIPOLAR: GenericfxCurve = GenericfxCurve::from_raw(0x81);
        let header = GenericfxUnitHeader::new(
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
                        DECIMAL_ONE,
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
                GenericfxParamMapping::new(
                    GENERICFX_PARAM_ASSIGN_DEPTH,
                    EXP_BIPOLAR,
                    -1000,
                    1000,
                    0,
                ),
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
        );

        // SAFETY: both packed header types contain only initialized integer/byte
        // fields and have no padding. A byte slice may have alignment one and
        // remains bounded by the live local value for this assertion.
        let bytes = unsafe {
            core::slice::from_raw_parts(
                core::ptr::addr_of!(header).cast::<u8>(),
                size_of::<GenericfxUnitHeader>(),
            )
        };
        assert_eq!(bytes.len(), 376);
        assert_eq!(&bytes[0..4], &376u32.to_ne_bytes());
        assert_eq!(bytes[56 + 9], 0);
        assert_eq!(bytes[56 + 2 * 32 + 9], 0x11);
        assert_eq!(bytes[312 + 2 * 8 + 1], 0x81);
        assert_eq!(bytes[375], 0);
    }
}

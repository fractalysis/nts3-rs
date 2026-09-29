// Diopser: a phase rotation plugin
// Copyright (C) 2021-2024 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use nts3::prelude::*;

mod filter;

use filter::{BiquadCoefficients, StereoBiquad};

/// Diopser defaults to zero stages, which would make a fixed-parameter port a
/// pass-through. This port deliberately uses a fixed audible stage count for
/// the hardware version.
const FILTER_STAGES: usize = 100;
const DEFAULT_FREQUENCY_HZ: f32 = 200.0;
const DEFAULT_RESONANCE: f32 = 0.5;
const RESONANCE_SCALE: f32 = 0.01;
const MIN_FREQUENCY_HZ: f32 = 5.0;

/// The remaining original parameters are fixed at their defaults: bypass off,
/// spread 0 octaves, octave spread style, maximum automation precision, and
/// the hidden "very important" switch on. Only frequency and resonance need
/// host-visible NTS-3 descriptors.
#[derive(Nts3Parameters)]
pub struct DiopserParameters {
    #[parameter(
        name = "FREQUENCY",
        min = 5,
        max = 20000,
        default = 200,
        parameter_type = "hertz",
        smoothing_ms = 100.0,
        assign = "x",
        curve = "exp",
        mapping_min = 5,
        mapping_max = 20000,
        mapping_default = 200
    )]
    pub frequency_hz: SmoothedParameter,

    /// Stored in hundredths so the original 0.01..30.00 Q range fits the
    /// NTS-3's i16 parameter representation without losing useful precision.
    #[parameter(
        name = "RESONANCE",
        min = 1,
        max = 3000,
        default = 50,
        parameter_type = "none",
        decimal_places = 2,
        smoothing_ms = 100.0,
        assign = "y",
        curve = "exp",
        mapping_min = 1,
        mapping_max = 3000,
        mapping_default = 50
    )]
    pub resonance: SmoothedParameter,
}

/// Scalar stereo DSP for the Cortex-M7. The target has a scalar VFPv4-D16 FPU
/// and integer DSP instructions, but no NEON/Advanced SIMD unit.
pub struct DiopserPlug {
    // External SDRAM keeps the stage states out of both the constrained static
    // SRAM image and the initialization callback's small stack frame.
    filters: Vec<StereoBiquad>,
    coefficients: BiquadCoefficients,
    sample_rate: f32,
    coefficient_frequency_hz: f32,
    coefficient_resonance: f32,
}

impl Default for DiopserPlug {
    fn default() -> Self {
        Self {
            filters: alloc::vec![StereoBiquad::new(); FILTER_STAGES],
            coefficients: BiquadCoefficients::identity(),
            sample_rate: 48_000.0,
            // Force a coefficient refresh during initialization even if the
            // constants above are changed to zero in the future.
            coefficient_frequency_hz: -1.0,
            coefficient_resonance: -1.0,
        }
    }
}

/// These IDs are non-reserved placeholders. A distributed build must use the
/// developer's own registered ID.
#[nts3::plugin(
    name = "Diopser",
    developer_id = 0x4652_5348,
    unit_id = 0x4449_4F50,
    sdram_bytes = 20000
)]
impl Nts3Plugin for DiopserPlug {
    type Parameters = DiopserParameters;

    fn initialize(&mut self, context: &InitContext<'_>) -> Result<(), InitError> {
        self.sample_rate = context.sample_rate_hz() as f32;
        self.update_coefficients(DEFAULT_FREQUENCY_HZ, DEFAULT_RESONANCE);
        Ok(())
    }

    fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        for mut frame in buffer.frames_mut() {
            // SmoothedParameter advances at audio rate. The original default
            // automation precision is also one coefficient update per sample.
            let frequency_hz = parameters.frequency_hz.next_plain();
            let resonance = parameters.resonance.next_plain() * RESONANCE_SCALE;
            if frequency_hz != self.coefficient_frequency_hz
                || resonance != self.coefficient_resonance
            {
                self.update_coefficients(frequency_hz, resonance);
            }

            let mut wet = frame.input();
            for filter in &mut self.filters {
                wet = filter.process(wet, &self.coefficients);
            }
            frame.write(wet);
        }
    }
}

#[cfg(feature = "web")]
nts3_rs_wasm::export_wasm_effect!(DiopserPlug);

impl DiopserPlug {
    fn update_coefficients(&mut self, frequency_hz: f32, resonance: f32) {
        // Keep the Audio EQ Cookbook preconditions true if a future runtime
        // accepts a sample rate lower than the NTS-3's normal 48 kHz.
        let maximum_frequency_hz = self.sample_rate / 2.05;
        let bounded_frequency_hz = frequency_hz.clamp(MIN_FREQUENCY_HZ, maximum_frequency_hz);
        self.coefficients =
            BiquadCoefficients::allpass(self.sample_rate, bounded_frequency_hz, resonance);
        self.coefficient_frequency_hz = frequency_hz;
        self.coefficient_resonance = resonance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_defaults_match_the_original() {
        let parameters = DiopserParameters::default();
        assert_eq!(parameters.frequency_hz.raw(), 200);
        assert_eq!(parameters.resonance.raw(), 50);
        assert_eq!(DiopserParameters::COUNT, 2);
    }

    #[test]
    fn static_dsp_state_stays_small() {
        // The 1,600-byte stage buffer is in external SDRAM. Only the Vec
        // handle and scalar coefficient/cache state remain in the static runtime.
        assert!(core::mem::size_of::<DiopserPlug>() < 64);
        assert_eq!(core::mem::size_of::<StereoBiquad>(), 16);
    }
}

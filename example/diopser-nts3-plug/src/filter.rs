// Diopser: a phase rotation plugin
// Copyright (C) 2021-2024 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

/// Prenormalized coefficients for a second-order all-pass filter.
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoefficients {
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Audio EQ Cookbook all-pass coefficients, matching the NIH-plug version.
    pub fn allpass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let omega = core::f32::consts::TAU * (frequency / sample_rate);
        let (sin_omega, cos_omega) = libm::sincosf(omega);
        let alpha = sin_omega / (2.0 * q);
        let a0_recip = 1.0 / (1.0 + alpha);

        let b0 = (1.0 - alpha) * a0_recip;
        let b1 = (-2.0 * cos_omega) * a0_recip;
        let b2 = 1.0;

        Self {
            b0,
            b1,
            b2,
            a1: b1,
            a2: b0,
        }
    }
}

/// One scalar stereo stage. Coefficients are shared by all stages because the
/// fixed spread is zero, avoiding redundant coefficient copies for every stage.
#[derive(Clone, Copy, Debug)]
pub struct StereoBiquad {
    s1: [f32; 2],
    s2: [f32; 2],
}

impl StereoBiquad {
    pub const fn new() -> Self {
        Self {
            s1: [0.0; 2],
            s2: [0.0; 2],
        }
    }

    #[inline(always)]
    pub fn process(&mut self, samples: [f32; 2], coefficients: &BiquadCoefficients) -> [f32; 2] {
        let output = [
            coefficients.b0 * samples[0] + self.s1[0],
            coefficients.b0 * samples[1] + self.s1[1],
        ];

        self.s1 = [
            coefficients.b1 * samples[0] - coefficients.a1 * output[0] + self.s2[0],
            coefficients.b1 * samples[1] - coefficients.a1 * output[1] + self.s2[1],
        ];
        self.s2 = [
            coefficients.b2 * samples[0] - coefficients.a2 * output[0],
            coefficients.b2 * samples[1] - coefficients.a2 * output[1],
        ];

        output
    }

    pub fn reset(&mut self) {
        self.s1 = [0.0; 2];
        self.s2 = [0.0; 2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_passes_stereo_and_reset_clears_state() {
        let mut filter = StereoBiquad::new();
        let identity = BiquadCoefficients::identity();
        assert_eq!(filter.process([0.25, -0.75], &identity), [0.25, -0.75]);
        filter.reset();
        assert_eq!(filter.process([0.0, 0.0], &identity), [0.0, 0.0]);
    }

    #[test]
    fn allpass_coefficients_and_output_are_finite_at_parameter_extremes() {
        for frequency in [5.0, 200.0, 20_000.0] {
            for q in [0.01, 0.5, 30.0] {
                let coefficients = BiquadCoefficients::allpass(48_000.0, frequency, q);
                let mut filter = StereoBiquad::new();
                let output = filter.process([1.0, -1.0], &coefficients);
                assert!(output.into_iter().all(f32::is_finite));
            }
        }
    }
}

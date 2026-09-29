#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use nts3::prelude::*;

// The NTS-3 runtime accepts only 48 kHz. Precomputing the fixed filter and
// envelope coefficients avoids deep libm call chains on its tiny callback stack.
const SAMPLE_RATE: f32 = 48_000.0;
const ATTACK: [f32; 3] = [0.998_693_35, 0.997_213_7, 0.995_381_06];
const RELEASE: [f32; 3] = [0.999_261_5, 0.999_261_5, 0.998_422_96];
const UP_THRESHOLD_POWER: [f32; 3] = [8.317_638e-5, 6.606_934e-5, 8.317_638e-5];
const DOWN_THRESHOLD_POWER: [f32; 3] = [0.000_416_869_38, 0.000_954_992_6, 0.000_281_838_3];
const PRE_GAIN: f32 = 1.819_700_8; // +5.2 dB
const POST_GAIN: [f32; 3] = [3.273_407, 1.927_525, 3.273_407]; // +10.3, +5.7, +10.3 dB
const UP_THRESHOLD: [f32; 3] = [-40.8, -41.8, -40.8];
const DOWN_THRESHOLD: [f32; 3] = [-33.8, -30.2, -35.5];
const DOWN_RATIO: [f32; 2] = [66.7, 66.7]; // high band: infinite ratio
const UP_RATIO: f32 = 4.17;
// Approximation: Ableton does not publish the knee transfer curve. A 24 dB
// quadratic knee matched the supplied render better than narrow 6/12 dB knees.
const KNEE_DB: f32 = 24.0;

#[derive(Nts3Parameters)]
pub struct OttParameters {
    #[parameter(
        name = "UPWARD",
        min = 0,
        max = 1000,
        default = 1000,
        parameter_type = "percent",
        decimal_places = 1,
        assign = "x",
        curve = "linear",
        mapping_min = 0,
        mapping_max = 1000,
        mapping_default = 1000
    )]
    pub upward: Parameter,

    #[parameter(
        name = "DOWNWARD",
        min = 0,
        max = 1000,
        default = 1000,
        parameter_type = "percent",
        decimal_places = 1,
        assign = "y",
        curve = "linear",
        mapping_min = 0,
        mapping_max = 1000,
        mapping_default = 1000
    )]
    pub downward: Parameter,

    #[parameter(
        name = "DEPTH",
        min = -1000,
        max = 1000,
        center = 0,
        default = 1000,
        parameter_type = "drywet",
        decimal_places = 1,
        assign = "depth",
        curve = "exp",
        curve_polarity = "bipolar",
        mapping_min = -1000,
        mapping_max = 1000,
        mapping_default = 1000
    )]
    pub depth: Parameter,
}

/// Transposed direct form II; two channels share the same coefficients.
#[derive(Clone, Copy)]
struct Biquad {
    z1: [f32; 2],
    z2: [f32; 2],
}

impl Biquad {
    const fn new() -> Self {
        Self {
            z1: [0.0; 2],
            z2: [0.0; 2],
        }
    }

    #[inline]
    fn process(&mut self, input: [f32; 2], c: &Coefficients) -> [f32; 2] {
        let output = [c.b0 * input[0] + self.z1[0], c.b0 * input[1] + self.z1[1]];
        for ch in 0..2 {
            self.z1[ch] = c.b1 * input[ch] - c.a1 * output[ch] + self.z2[ch];
            self.z2[ch] = c.b2 * input[ch] - c.a2 * output[ch];
        }
        output
    }
}

#[derive(Clone, Copy)]
struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

// RBJ Butterworth biquads at 48 kHz; two identical sections per LR4 branch.
const LOW_COEFFICIENTS: (Coefficients, Coefficients) = (
    Coefficients {
        b0: 0.000_033_128_276,
        b1: 0.000_066_256_55,
        b2: 0.000_033_128_276,
        a1: -1.983_654_3,
        a2: 0.983_786_76,
    },
    Coefficients {
        b0: 0.991_860_3,
        b1: -1.983_720_5,
        b2: 0.991_860_3,
        a1: -1.983_654_3,
        a2: 0.983_786_76,
    },
);
const HIGH_COEFFICIENTS: (Coefficients, Coefficients) = (
    Coefficients {
        b0: 0.021_620_719,
        b1: 0.043_241_438,
        b2: 0.021_620_719,
        a1: -1.543_121_1,
        a2: 0.629_604,
    },
    Coefficients {
        b0: 0.793_181_3,
        b1: -1.586_362_6,
        b2: 0.793_181_3,
        a1: -1.543_121_1,
        a2: 0.629_604,
    },
);

/// Fourth-order Linkwitz-Riley crossover (two cascaded Butterworth sections
/// on each side). The low and high outputs sum to an all-pass response.
struct Crossover {
    low: [Biquad; 2],
    high: [Biquad; 2],
}

impl Crossover {
    const fn new() -> Self {
        Self {
            low: [Biquad::new(); 2],
            high: [Biquad::new(); 2],
        }
    }

    #[inline]
    fn split(
        &mut self,
        input: [f32; 2],
        low: &Coefficients,
        high: &Coefficients,
    ) -> ([f32; 2], [f32; 2]) {
        let l = self.low[0].process(input, low);
        let l = self.low[1].process(l, low);
        let h = self.high[0].process(input, high);
        let h = self.high[1].process(h, high);
        (l, h)
    }
}

struct RmsCompressor {
    up_power: f32,
    down_power: f32,
    attack: f32,
    release: f32,
}

impl RmsCompressor {
    const fn new() -> Self {
        Self {
            up_power: 0.0,
            down_power: 0.0,
            attack: 0.0,
            release: 0.0,
        }
    }

    fn configure(&mut self, band: usize) {
        // Attack = exp(-3000 / (48000 * attack_ms)); release uses -10000.
        // These settling factors were calibrated against the reference.
        self.attack = ATTACK[band];
        self.release = RELEASE[band];
    }

    fn reset(&mut self, band: usize) {
        // Start at the thresholds rather than amplifying an empty detector on
        // the first transient after reset.
        self.up_power = UP_THRESHOLD_POWER[band];
        self.down_power = DOWN_THRESHOLD_POWER[band];
    }

    #[inline]
    fn process(&mut self, input: [f32; 2], band: usize, up: f32, down: f32) -> [f32; 2] {
        // Stereo-linked RMS detection keeps the stereo image from shifting as
        // independent channel levels change. Pregain is applied before detection.
        let l = input[0] * PRE_GAIN;
        let r = input[1] * PRE_GAIN;
        let power = 0.5 * (l * l + r * r);
        let down_coefficient = if power > self.down_power {
            self.attack
        } else {
            self.release
        };
        self.down_power = down_coefficient * self.down_power + (1.0 - down_coefficient) * power;
        // The upward detector must follow falling levels as well as rising
        // ones; otherwise a kick holds back the quiet bands for the entire
        // long downward release time.
        self.up_power = self.attack * self.up_power + (1.0 - self.attack) * power;

        // 10 log10(power) = 20 log10(RMS). The floor keeps silence finite.
        let up_db = 10.0 * libm::log10f(self.up_power.max(1.0e-12));
        let down_db = 10.0 * libm::log10f(self.down_power.max(1.0e-12));
        let up_ratio = 1.0 + (UP_RATIO - 1.0) * up;
        let up_slope = 1.0 - 1.0 / up_ratio;
        let down_slope = if band == 2 {
            down // interpolate the *slope* to an infinite ratio
        } else {
            1.0 - 1.0 / (1.0 + (DOWN_RATIO[band] - 1.0) * down)
        };
        let gain_db = up_slope * soft_knee(UP_THRESHOLD[band] - up_db)
            - down_slope * soft_knee(down_db - DOWN_THRESHOLD[band]);
        let gain =
            libm::exp2f(gain_db * (core::f32::consts::LOG2_10 / 20.0)) * PRE_GAIN * POST_GAIN[band];
        [input[0] * gain, input[1] * gain]
    }
}

#[inline]
fn soft_knee(distance_db: f32) -> f32 {
    let half = KNEE_DB * 0.5;
    if distance_db <= -half {
        0.0
    } else if distance_db >= half {
        distance_db
    } else {
        let t = distance_db + half;
        t * t / (2.0 * KNEE_DB)
    }
}

pub struct OttPlug {
    // Keep filter history out of static SRAM and the unit_init stack frame.
    // Indices: low split, high split, low-band phase correction.
    splits: Vec<Crossover>,
    low_coefficients: (Coefficients, Coefficients),
    high_coefficients: (Coefficients, Coefficients),
    compressors: [RmsCompressor; 3],
}

impl Default for OttPlug {
    fn default() -> Self {
        let mut splits = Vec::with_capacity(3);
        for _ in 0..3 {
            splits.push(Crossover::new());
        }
        Self {
            splits,
            low_coefficients: LOW_COEFFICIENTS,
            high_coefficients: HIGH_COEFFICIENTS,
            compressors: [
                RmsCompressor::new(),
                RmsCompressor::new(),
                RmsCompressor::new(),
            ],
        }
    }
}

// Non-reserved example IDs; replace the developer ID before distribution.
#[nts3::plugin(
    name = "OTT",
    developer_id = 0x4652_5348,
    unit_id = 0x4F54_5433,
    sdram_bytes = 20000
)]
impl Nts3Plugin for OttPlug {
    type Parameters = OttParameters;

    fn initialize(&mut self, context: &InitContext<'_>) -> Result<(), InitError> {
        if context.sample_rate_hz() != SAMPLE_RATE as u32 {
            return Err(InitError::SampleRate);
        }
        for (index, compressor) in self.compressors.iter_mut().enumerate() {
            compressor.configure(index);
        }
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        // Clear in place: constructing whole 64-byte crossovers here made the
        // nested unit_init -> reset -> Crossover::new -> memcpy stack too deep.
        for crossover in &mut self.splits {
            for filter in crossover.low.iter_mut().chain(crossover.high.iter_mut()) {
                filter.z1 = [0.0; 2];
                filter.z2 = [0.0; 2];
            }
        }
        for (band, compressor) in self.compressors.iter_mut().enumerate() {
            compressor.reset(band);
        }
    }

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        let up = parameters.upward.normalized();
        let down = parameters.downward.normalized();
        let wet = parameters.depth.normalized();
        let (low_split, remaining) = self.splits.split_at_mut(1);
        let (high_split, low_phase) = remaining.split_at_mut(1);
        let low_split = &mut low_split[0];
        let high_split = &mut high_split[0];
        let low_phase = &mut low_phase[0];
        for mut frame in buffer.frames_mut() {
            let input = frame.input();
            let (low, upper) =
                low_split.split(input, &self.low_coefficients.0, &self.low_coefficients.1);
            let (mid, high) =
                high_split.split(upper, &self.high_coefficients.0, &self.high_coefficients.1);
            let (low_a, low_b) =
                low_phase.split(low, &self.high_coefficients.0, &self.high_coefficients.1);
            let low = [low_a[0] + low_b[0], low_a[1] + low_b[1]];
            let low = self.compressors[0].process(low, 0, up, down);
            let mid = self.compressors[1].process(mid, 1, up, down);
            let high = self.compressors[2].process(high, 2, up, down);
            let dry = 1.0 - wet;
            frame.write([
                dry * input[0] + wet * (low[0] + mid[0] + high[0]),
                dry * input[1] + wet * (low[1] + mid[1] + high[1]),
            ]);
        }
    }
}

#[cfg(feature = "web")]
nts3_rs_wasm::export_wasm_effect!(OttPlug);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsp_history_stays_out_of_the_static_runtime() {
        assert_eq!(core::mem::size_of::<Crossover>(), 64);
        assert!(core::mem::size_of::<OttPlug>() <= 160);
        let plugin = OttPlug::default();
        assert_eq!(plugin.splits.len(), 3);
        assert_eq!(plugin.splits.capacity(), 3);
    }

    #[test]
    fn knee_joins_the_linear_regions() {
        assert_eq!(soft_knee(-12.0), 0.0);
        assert_eq!(soft_knee(12.0), 12.0);
        assert_eq!(soft_knee(0.0), 3.0);
    }

    #[test]
    fn crossover_recombines_without_a_notch() {
        let low_c = LOW_COEFFICIENTS;
        let high_c = HIGH_COEFFICIENTS;
        for frequency in [40.0, 88.3, 500.0, 2_500.0, 8_000.0] {
            let (mut first, mut second, mut phase) =
                (Crossover::new(), Crossover::new(), Crossover::new());
            let mut input_power = 0.0;
            let mut output_power = 0.0;
            for index in 0..48_000 {
                let sample =
                    libm::sinf(core::f32::consts::TAU * frequency * index as f32 / 48_000.0);
                let (low, upper) = first.split([sample; 2], &low_c.0, &low_c.1);
                let (mid, high) = second.split(upper, &high_c.0, &high_c.1);
                let (low_a, low_b) = phase.split(low, &high_c.0, &high_c.1);
                if index >= 24_000 {
                    input_power += sample * sample;
                    let combined = low_a[0] + low_b[0] + mid[0] + high[0];
                    output_power += combined * combined;
                }
            }
            let magnitude = libm::sqrtf(output_power / input_power);
            assert!(
                (magnitude - 1.0).abs() < 0.02,
                "{frequency} Hz: {magnitude}"
            );
        }
    }

    #[test]
    fn compression_boosts_quiet_and_reduces_loud() {
        let mut compressor = RmsCompressor::new();
        compressor.configure(2);
        let fixed_gain = PRE_GAIN * POST_GAIN[2];
        let quiet = [0.0001, 0.0001];
        compressor.up_power = (quiet[0] * PRE_GAIN).powi(2);
        compressor.down_power = compressor.up_power;
        assert!(compressor.process(quiet, 2, 1.0, 1.0)[0] > quiet[0] * fixed_gain);
        let loud = [0.5, 0.5];
        compressor.up_power = (loud[0] * PRE_GAIN).powi(2);
        compressor.down_power = compressor.up_power;
        assert!(compressor.process(loud, 2, 1.0, 1.0)[0] < loud[0] * fixed_gain);
        assert_eq!(compressor.process([0.0; 2], 2, 1.0, 1.0), [0.0; 2]);
    }

    #[test]
    fn ratios_at_zero_leave_the_compressor_unity() {
        let mut compressor = RmsCompressor::new();
        compressor.configure(2);
        // At X=Y=0 only the fixed pre/post gains remain.
        let output = compressor.process([0.25, -0.25], 2, 0.0, 0.0);
        let gain = PRE_GAIN * POST_GAIN[2];
        assert!((output[0] - 0.25 * gain).abs() < 1.0e-5);
        assert!((output[1] + 0.25 * gain).abs() < 1.0e-5);
    }
}

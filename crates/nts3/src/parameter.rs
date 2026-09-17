// This module adapts the f32 smoothing algorithm from wrl/baseplug's
// `src/smooth.rs` at commit 9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74.
// Baseplug is licensed MIT OR Apache-2.0; this adaptation uses the MIT grant.
// See THIRD_PARTY_NOTICES.md and plan/references/baseplug-LICENSE-MIT.
//
// Modifications: specialize to f32 and no_std, use libm::expf, replace the
// fixed-size output block with per-sample output, retain only each render
// block's first output for status updates, and define nonpositive smoothing
// times/sample rates as immediate.

use core::ffi::CStr;

use nts3_sys::{
    GenericfxParamMapping, UNIT_MAX_PARAM_COUNT, UNUSED_MAPPING, UNUSED_PARAM, UnitParam,
};

const SETTLE: f32 = 0.00001;

/// Result of a checked parameter update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterError {
    OutOfRange,
}

/// Compact integer target state for one NTS-3 parameter.
///
/// The two endpoints retain their declared orientation. Normal ranges map the
/// first endpoint to 0 and the second to 1; inverted ranges do the same in the
/// opposite numeric direction. Degenerate ranges normalize to 0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameter {
    raw: i16,
    min: i16,
    max: i16,
}

impl Parameter {
    /// Constructs a parameter and clamps `initial` to the numeric envelope.
    pub const fn new(min: i16, max: i16, initial: i16) -> Self {
        Self {
            raw: clamp_i16_to_endpoints(initial as i32, min, max),
            min,
            max,
        }
    }

    /// Constructs a parameter from the same descriptor used in the unit header.
    pub const fn from_descriptor(descriptor: &UnitParam) -> Self {
        Self::new(descriptor.min(), descriptor.max(), descriptor.init())
    }

    pub const fn raw(&self) -> i16 {
        self.raw
    }

    pub const fn plain(&self) -> f32 {
        self.raw as f32
    }

    pub fn normalized(&self) -> f32 {
        normalize(self.plain(), self.min, self.max)
    }

    pub const fn min(&self) -> i16 {
        self.min
    }

    pub const fn max(&self) -> i16 {
        self.max
    }

    /// Stores `value` after clamping it before the narrowing conversion.
    pub fn set(&mut self, value: i32) -> i16 {
        self.raw = clamp_i16_to_endpoints(value, self.min, self.max);
        self.raw
    }

    /// Stores an in-range value, leaving the old value unchanged otherwise.
    pub fn set_checked(&mut self, value: i32) -> Result<(), ParameterError> {
        let low = i32::from(self.min.min(self.max));
        let high = i32::from(self.min.max(self.max));
        if value < low || value > high {
            return Err(ParameterError::OutOfRange);
        }
        self.raw = value as i16;
        Ok(())
    }
}

const fn clamp_i16_to_endpoints(value: i32, first: i16, second: i16) -> i16 {
    let low = if first < second { first } else { second } as i32;
    let high = if first > second { first } else { second } as i32;
    if value < low {
        low as i16
    } else if value > high {
        high as i16
    } else {
        value as i16
    }
}

fn normalize(value: f32, first: i16, second: i16) -> f32 {
    if first == second {
        return 0.0;
    }

    let low = f32::from(first.min(second));
    let high = f32::from(first.max(second));
    let bounded = value.clamp(low, high);
    (bounded - f32::from(first)) / (f32::from(second) - f32::from(first))
}

/// Baseplug-compatible one-pole smoother lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmoothStatus {
    Inactive,
    Active,
    Deactivating,
}

impl SmoothStatus {
    pub const fn is_active(self) -> bool {
        !matches!(self, Self::Inactive)
    }
}

/// Allocation-free f32 adaptation of Baseplug's one-pole smoother.
///
/// Milliseconds are an exponential time constant, not a linear completion
/// duration. After one time constant, about 36.8% of the initial error remains.
#[derive(Clone, Copy, Debug)]
pub struct Smooth {
    input: f32,
    status: SmoothStatus,
    a: f32,
    b: f32,
    last_output: f32,
    block_first_output: f32,
    block_has_output: bool,
}

impl Smooth {
    pub const fn new(input: f32) -> Self {
        Self {
            input,
            status: SmoothStatus::Inactive,
            a: 1.0,
            b: 0.0,
            last_output: input,
            block_first_output: input,
            block_has_output: false,
        }
    }

    /// Resets target and current output while preserving configured speed.
    pub fn reset(&mut self, value: f32) {
        let a = self.a;
        let b = self.b;
        *self = Self {
            a,
            b,
            ..Self::new(value)
        };
    }

    /// Sets a destination and activates smoothing, even when unchanged.
    pub fn set(&mut self, value: f32) {
        self.input = value;
        self.status = SmoothStatus::Active;
    }

    pub const fn dest(&self) -> f32 {
        self.input
    }

    pub const fn current_value(&self) -> f32 {
        self.last_output
    }

    pub const fn status(&self) -> SmoothStatus {
        self.status
    }

    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Configures Baseplug's one-pole coefficients.
    ///
    /// `milliseconds <= 0`, a nonpositive sample rate, and non-finite inputs are
    /// defined as immediate to avoid invalid target coefficients.
    pub fn set_speed_ms(&mut self, sample_rate: f32, milliseconds: f32) {
        if !(milliseconds > 0.0
            && sample_rate > 0.0
            && milliseconds.is_finite()
            && sample_rate.is_finite())
        {
            self.a = 1.0;
            self.b = 0.0;
            return;
        }

        self.b = libm::expf(-1.0 / (milliseconds * (sample_rate / 1000.0)));
        self.a = 1.0 - self.b;
    }

    /// Returns the next sample from the one-pole recurrence.
    #[inline]
    pub fn next_value(&mut self) -> f32 {
        if self.status == SmoothStatus::Active {
            self.last_output = self.input * self.a + self.last_output * self.b;
        }
        if !self.block_has_output {
            self.block_first_output = self.last_output;
            self.block_has_output = true;
        }
        self.last_output
    }

    pub(crate) fn begin_block(&mut self) {
        self.block_has_output = false;
    }

    pub(crate) fn end_block(&mut self) -> SmoothStatus {
        if self.block_has_output {
            self.update_status()
        } else {
            self.status
        }
    }

    fn update_status_with_epsilon(&mut self, epsilon: f32) -> SmoothStatus {
        match self.status {
            SmoothStatus::Active => {
                if (self.input - self.block_first_output).abs() < epsilon {
                    self.reset(self.input);
                    self.status = SmoothStatus::Deactivating;
                }
            }
            SmoothStatus::Deactivating => self.status = SmoothStatus::Inactive,
            SmoothStatus::Inactive => {}
        }
        self.status
    }

    fn update_status(&mut self) -> SmoothStatus {
        self.update_status_with_epsilon(SETTLE)
    }
}

/// Integer target plus compact per-sample smoothing state.
#[derive(Clone, Copy, Debug)]
pub struct SmoothedParameter {
    target: Parameter,
    smooth: Smooth,
    smoothing_ms: f32,
}

impl SmoothedParameter {
    pub const fn new(min: i16, max: i16, initial: i16, smoothing_ms: f32) -> Self {
        let target = Parameter::new(min, max, initial);
        Self {
            smooth: Smooth::new(target.plain()),
            target,
            smoothing_ms,
        }
    }

    /// Constructs a smoothed parameter from its header descriptor.
    pub const fn from_descriptor(descriptor: &UnitParam, smoothing_ms: f32) -> Self {
        Self::new(
            descriptor.min(),
            descriptor.max(),
            descriptor.init(),
            smoothing_ms,
        )
    }

    pub const fn raw(&self) -> i16 {
        self.target.raw()
    }

    /// Returns the unsmoothed target value.
    pub const fn plain(&self) -> f32 {
        self.target.plain()
    }

    /// Returns the unsmoothed normalized target value.
    pub fn normalized(&self) -> f32 {
        self.target.normalized()
    }

    pub const fn status(&self) -> SmoothStatus {
        self.smooth.status()
    }

    pub const fn current_plain(&self) -> f32 {
        self.smooth.current_value()
    }

    pub fn current_normalized(&self) -> f32 {
        normalize(self.current_plain(), self.target.min(), self.target.max())
    }

    pub fn set(&mut self, value: i32) -> i16 {
        let stored = self.target.set(value);
        self.smooth.set(f32::from(stored));
        stored
    }

    pub fn set_checked(&mut self, value: i32) -> Result<(), ParameterError> {
        self.target.set_checked(value)?;
        self.smooth.set(self.target.plain());
        Ok(())
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.smooth.set_speed_ms(sample_rate, self.smoothing_ms);
    }

    /// Snaps the smoother to its current target without changing the target.
    pub fn reset_smoothing(&mut self) {
        self.smooth.reset(self.target.plain());
    }

    #[inline]
    pub fn next_plain(&mut self) -> f32 {
        self.smooth.next_value()
    }

    #[inline]
    pub fn next_normalized(&mut self) -> f32 {
        normalize(self.next_plain(), self.target.min(), self.target.max())
    }

    #[doc(hidden)]
    pub fn begin_block(&mut self) {
        self.smooth.begin_block();
    }

    #[doc(hidden)]
    pub fn end_block(&mut self) {
        self.smooth.end_block();
    }
}

/// Sealed parameter dispatch implemented by generated code and internal tests.
///
/// Constants, defaults, and dispatch implementations should all be sourced from
/// one metadata definition so header values cannot drift from runtime values.
#[doc(hidden)]
pub trait Nts3Parameters: crate::__private::Sealed + Default {
    const DESCRIPTORS: [UnitParam; UNIT_MAX_PARAM_COUNT] = [UNUSED_PARAM; UNIT_MAX_PARAM_COUNT];
    const MAPPINGS: [GenericfxParamMapping; UNIT_MAX_PARAM_COUNT] =
        [UNUSED_MAPPING; UNIT_MAX_PARAM_COUNT];
    const COUNT: usize = 0;

    fn get(&self, _index: u8) -> Option<i32> {
        None
    }

    fn set(&mut self, _index: u8, _value: i32) -> bool {
        false
    }

    fn initialize_smoothers(&mut self, _sample_rate: f32) {}

    fn reset_smoothers(&mut self) {}

    fn begin_block(&mut self) {}

    fn end_block(&mut self) {}

    fn string_value(&self, _index: u8, _value: i32) -> Option<&'static CStr> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    // Host-only reference retaining the pinned Baseplug fixed-block algorithm.
    // This is intentionally close to plan/references/baseplug-smooth.rs; generic
    // and formatting/Index APIs irrelevant to sequence behavior are omitted.
    const REFERENCE_MAX_BLOCK: usize = 128;

    struct ReferenceSmooth {
        output: [f32; REFERENCE_MAX_BLOCK],
        input: f32,
        status: SmoothStatus,
        a: f32,
        b: f32,
        last_output: f32,
    }

    impl ReferenceSmooth {
        fn new(input: f32) -> Self {
            Self {
                output: [input; REFERENCE_MAX_BLOCK],
                input,
                status: SmoothStatus::Inactive,
                a: 1.0,
                b: 0.0,
                last_output: input,
            }
        }

        fn reset(&mut self, value: f32) {
            let a = self.a;
            let b = self.b;
            *self = Self {
                a,
                b,
                ..Self::new(value)
            };
        }

        fn set(&mut self, value: f32) {
            self.input = value;
            self.status = SmoothStatus::Active;
        }

        fn set_speed_ms(&mut self, sample_rate: f32, milliseconds: f32) {
            if milliseconds <= 0.0 {
                self.b = 0.0;
                self.a = 1.0;
            } else {
                self.b = (-1.0 / (milliseconds * (sample_rate / 1000.0))).exp();
                self.a = 1.0 - self.b;
            }
        }

        fn process(&mut self, frames: usize) {
            if self.status != SmoothStatus::Active {
                return;
            }
            let input = self.input * self.a;
            self.output[0] = input + self.last_output * self.b;
            for index in 1..frames {
                self.output[index] = input + self.output[index - 1] * self.b;
            }
            self.last_output = self.output[frames - 1];
        }

        fn update_status(&mut self) {
            match self.status {
                SmoothStatus::Active => {
                    if (self.input - self.output[0]).abs() < SETTLE {
                        self.reset(self.input);
                        self.status = SmoothStatus::Deactivating;
                    }
                }
                SmoothStatus::Deactivating => self.status = SmoothStatus::Inactive,
                SmoothStatus::Inactive => {}
            }
        }
    }

    fn compare_partition(
        sample_rate: f32,
        milliseconds: f32,
        blocks: &[usize],
        retarget_at: Option<(usize, f32)>,
        reset_at: Option<(usize, f32)>,
    ) {
        let mut reference = ReferenceSmooth::new(-3.0);
        let mut adapted = Smooth::new(-3.0);
        reference.set_speed_ms(sample_rate, milliseconds);
        adapted.set_speed_ms(sample_rate, milliseconds);
        reference.set(8.0);
        adapted.set(8.0);

        for (block_index, &frames) in blocks.iter().enumerate() {
            if let Some((at, value)) = retarget_at {
                if block_index == at {
                    reference.set(value);
                    adapted.set(value);
                }
            }
            if let Some((at, value)) = reset_at {
                if block_index == at {
                    reference.reset(value);
                    adapted.reset(value);
                }
            }

            reference.process(frames);
            adapted.begin_block();
            let actual: Vec<_> = (0..frames).map(|_| adapted.next_value()).collect();
            for (expected, actual) in reference.output[..frames].iter().zip(actual) {
                assert!(
                    (expected - actual).abs() <= 2.0e-5,
                    "{expected} != {actual}"
                );
            }
            reference.update_status();
            adapted.end_block();
            assert_eq!(reference.status, adapted.status());
            assert!((reference.last_output - adapted.current_value()).abs() <= 2.0e-5);
        }
    }

    #[test]
    fn parameter_clamping_checked_updates_and_normalization_cover_range_shapes() {
        let mut negative = Parameter::new(-200, -100, -150);
        assert_eq!(negative.normalized(), 0.5);
        assert_eq!(negative.set(i32::MIN), -200);
        assert_eq!(negative.normalized(), 0.0);
        assert_eq!(negative.set(i32::MAX), -100);
        assert_eq!(negative.normalized(), 1.0);

        let mut bipolar = Parameter::new(-100, 100, 0);
        assert_eq!(bipolar.normalized(), 0.5);
        assert_eq!(bipolar.set_checked(101), Err(ParameterError::OutOfRange));
        assert_eq!(bipolar.raw(), 0);
        assert_eq!(bipolar.set_checked(-100), Ok(()));

        let mut inverted = Parameter::new(10, -10, 10);
        assert_eq!(inverted.normalized(), 0.0);
        inverted.set(0);
        assert_eq!(inverted.normalized(), 0.5);
        inverted.set(-10);
        assert_eq!(inverted.normalized(), 1.0);

        let mut degenerate = Parameter::new(7, 7, -20);
        assert_eq!(degenerate.raw(), 7);
        assert_eq!(degenerate.normalized(), 0.0);
        assert_eq!(degenerate.set(99), 7);
    }

    #[test]
    fn coefficients_and_known_outputs_match_baseplug_formula() {
        let mut smooth = Smooth::new(0.0);
        smooth.set_speed_ms(48_000.0, 100.0);
        let expected_b = (-1.0f32 / 4_800.0).exp();
        assert!((smooth.b - expected_b).abs() <= 1.0e-7);
        assert!((smooth.a - (1.0 - expected_b)).abs() <= 1.0e-7);

        smooth.set(1.0);
        smooth.begin_block();
        let values = [
            smooth.next_value(),
            smooth.next_value(),
            smooth.next_value(),
            smooth.next_value(),
        ];
        smooth.end_block();
        let expected = [0.00020831823, 0.00041659305, 0.0006248245, 0.000_833_012_6];
        for (actual, expected) in values.into_iter().zip(expected) {
            assert!((actual - expected).abs() <= 2.0e-7);
        }
    }

    #[test]
    fn adapted_sequences_match_reference_across_partitions_rates_and_events() {
        compare_partition(
            48_000.0,
            100.0,
            &[1, 7, 64, 3, 128, 5, 32, 9],
            Some((3, -1.25)),
            None,
        );
        compare_partition(
            44_100.0,
            0.25,
            &[16, 1, 5, 64, 2, 127, 8],
            Some((4, 2.0)),
            Some((2, -9.0)),
        );
        compare_partition(
            96_000.0,
            250.0,
            &[128, 128, 17, 3, 99],
            None,
            Some((1, 4.0)),
        );

        // Run through Active -> Deactivating -> Inactive with irregular blocks.
        let settling_blocks: Vec<_> = (0..400).map(|index| [1, 7, 64, 128][index % 4]).collect();
        compare_partition(48_000.0, 20.0, &settling_blocks, None, None);
    }

    #[test]
    fn nonpositive_time_is_immediate_and_status_timing_matches_baseplug() {
        for milliseconds in [0.0, -10.0] {
            let mut smooth = Smooth::new(2.0);
            smooth.set_speed_ms(48_000.0, milliseconds);
            smooth.set(9.0);
            smooth.begin_block();
            assert_eq!(smooth.next_value(), 9.0);
            assert_eq!(smooth.end_block(), SmoothStatus::Deactivating);
            assert_eq!(smooth.current_value(), 9.0);
            smooth.begin_block();
            assert_eq!(smooth.next_value(), 9.0);
            assert_eq!(smooth.end_block(), SmoothStatus::Inactive);
        }
    }

    #[test]
    fn settlement_is_strictly_below_threshold_and_snaps_exactly() {
        let mut exact = Smooth::new(SETTLE);
        exact.input = 0.0;
        exact.status = SmoothStatus::Active;
        exact.a = 0.0;
        exact.b = 1.0;
        exact.begin_block();
        exact.next_value();
        assert_eq!(exact.end_block(), SmoothStatus::Active);

        exact.last_output = SETTLE * 0.5;
        exact.begin_block();
        exact.next_value();
        assert_eq!(exact.end_block(), SmoothStatus::Deactivating);
        assert_eq!(exact.current_value().to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn retarget_rate_change_and_reset_preserve_continuity_as_expected() {
        let mut smooth = Smooth::new(0.0);
        smooth.set_speed_ms(48_000.0, 20.0);
        smooth.set(1.0);
        smooth.begin_block();
        let first = smooth.next_value();
        let before_change = smooth.next_value();
        smooth.set_speed_ms(96_000.0, 20.0);
        let after_change = smooth.next_value();
        assert!(first > 0.0 && before_change > first && after_change > before_change);

        smooth.set(-1.0);
        let retargeted = smooth.next_value();
        assert!(retargeted < after_change);
        smooth.reset(0.25);
        assert_eq!(smooth.dest(), 0.25);
        assert_eq!(smooth.current_value(), 0.25);
        assert_eq!(smooth.status(), SmoothStatus::Inactive);
    }

    #[test]
    fn zero_sample_block_does_not_advance_status() {
        let mut smooth = Smooth::new(0.0);
        smooth.set_speed_ms(48_000.0, 0.0);
        smooth.set(1.0);
        smooth.begin_block();
        assert_eq!(smooth.end_block(), SmoothStatus::Active);
    }

    #[test]
    fn compact_sizes_are_measured() {
        assert_eq!(core::mem::size_of::<Parameter>(), 6);
        assert_eq!(core::mem::size_of::<Smooth>(), 24);
        assert_eq!(core::mem::size_of::<SmoothedParameter>(), 36);
    }
}

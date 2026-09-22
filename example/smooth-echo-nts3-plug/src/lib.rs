#![no_std]

use fundsp::{Frame, audionode::AudioNode, delay, filter, typenum::U1};
use nts3::prelude::*;

// Experimental gain compensation as feedback approaches unity.
const VOLUME_CONTROL_COEF: f32 = 0.92;
const HIGHPASS_HZ: f32 = 10.0;
const MIN_TIME_SECONDS: f32 = 0.001;
const MAX_TIME_SECONDS: f32 = 2.0;

/// Host-owned parameters. The derive generates the eight-entry NTS-3 descriptor
/// table, default mappings, initialization, bounds checks, and index dispatch.
/// Unused descriptor slots are padded automatically.
#[derive(Nts3Parameters)]
pub struct EchoParameters {
    /// Stored by the NTS-3 as integer milliseconds and presented to DSP as f32.
    #[parameter(
        name = "TIME",
        min = 1,
        max = 2000,
        default = 500,
        parameter_type = "milliseconds",
        smoothing_ms = 100.0,
        assign = "x",
        curve = "exp",
        mapping_min = 1,
        mapping_max = 2000,
        mapping_default = 500
    )]
    pub time_ms: SmoothedParameter,

    /// One decimal digit makes 0..1000 display as 0.0..100.0 percent.
    #[parameter(
        name = "FEEDBACK",
        min = 0,
        max = 1000,
        default = 0,
        parameter_type = "percent",
        decimal_places = 1,
        assign = "y",
        curve = "linear",
        mapping_min = 0,
        mapping_max = 1000,
        mapping_default = 0
    )]
    pub feedback: Parameter,

    /// The hardware FX DEPTH slider controls the final dry/wet balance.
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

/// Only DSP and device-event state lives in the plugin. The framework owns the
/// parameter store and passes it into callbacks, avoiding Arc, locks, trait
/// objects, and a repetitive params() accessor in every plugin.
pub struct EchoPlug {
    delay_l: delay::Tap<U1>,
    delay_r: delay::Tap<U1>,
    highpass_l: filter::Highpole<f32, U1>,
    highpass_r: filter::Highpole<f32, U1>,
    last_sample_l: f32,
    last_sample_r: f32,

    // Touch activity is deliberately separate from coordinates. In particular,
    // (0, 0, touched) is not the same state as (0, 0, released).
    pad_touched: bool,
    last_touch_xy: [f32; 2],
}

impl Default for EchoPlug {
    fn default() -> Self {
        Self {
            delay_l: delay::Tap::new(MIN_TIME_SECONDS, MAX_TIME_SECONDS),
            delay_r: delay::Tap::new(MIN_TIME_SECONDS, MAX_TIME_SECONDS),
            highpass_l: filter::Highpole::new(HIGHPASS_HZ),
            highpass_r: filter::Highpole::new(HIGHPASS_HZ),
            last_sample_l: 0.0,
            last_sample_r: 0.0,
            pad_touched: false,
            last_touch_xy: [0.0, 0.0],
        }
    }
}

/// `plugin` supplies the fixed NTS-3 target/API metadata, emits the unit header,
/// exports every required C ABI callback, and installs the panic/allocator
/// runtime. Package version is read from CARGO_PKG_VERSION automatically.
///
/// The IDs below are valid non-reserved example IDs, but a distributing author
/// must choose/register their own developer ID before release.
#[nts3::plugin(
    name = "Smooth Echo",
    developer_id = 0x4652_5348,
    unit_id = 0x5345_4348,
    sdram_bytes = 1_100_000
)]
impl Nts3Plugin for EchoPlug {
    type Parameters = EchoParameters;

    fn initialize(&mut self, context: &InitContext<'_>) -> Result<(), InitError> {
        let sample_rate = context.sample_rate_hz() as f64;

        self.delay_l.set_sample_rate(sample_rate);
        self.delay_r.set_sample_rate(sample_rate);
        self.highpass_l.set_sample_rate(sample_rate);
        self.highpass_r.set_sample_rate(sample_rate);

        Ok(())
    }

    fn reset(&mut self) {
        self.delay_l.reset();
        self.delay_r.reset();
        self.highpass_l.reset();
        self.highpass_r.reset();
        self.last_sample_l = 0.0;
        self.last_sample_r = 0.0;
    }

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        for mut frame in buffer.frames_mut() {
            let [input_l, input_r] = frame.input();
            let time = parameters.time_ms.next_plain() * 0.001;
            let feedback = parameters.feedback.normalized();
            let wet_mix = parameters.depth.normalized();

            if feedback > 0.99 {
                // Near-unity feedback freezes the existing delay contents.
                let delayed_l = self.delay_l.tick(&Frame::from([self.last_sample_l, time]))[0];
                let delayed_r = self.delay_r.tick(&Frame::from([self.last_sample_r, time]))[0];
                self.last_sample_l = delayed_l;
                self.last_sample_r = delayed_r;
            } else {
                let gain = 1.0 - VOLUME_CONTROL_COEF * feedback;
                let delayed_l = self.delay_l.tick(&Frame::from([
                    input_l * gain + self.last_sample_l * feedback,
                    time,
                ]))[0];
                let delayed_r = self.delay_r.tick(&Frame::from([
                    input_r * gain + self.last_sample_r * feedback,
                    time,
                ]))[0];

                self.last_sample_l = input_l + delayed_l * feedback;
                self.last_sample_r = input_r + delayed_r * feedback;
            }

            let wet_l = self.highpass_l.tick(&Frame::from([self.last_sample_l]))[0];
            let wet_r = self.highpass_r.tick(&Frame::from([self.last_sample_r]))[0];
            let dry_mix = 1.0 - wet_mix;

            frame.write([
                input_l * dry_mix + wet_l * wet_mix,
                input_r * dry_mix + wet_r * wet_mix,
            ]);
        }
    }

    fn touch_event(&mut self, event: TouchEvent) {
        self.pad_touched = event.is_active();
        self.last_touch_xy = event.normalized_position();
    }
}

impl EchoPlug {
    /// Demonstrates that touch activity can be consumed as an ordinary internal
    /// effect control without spending one of the eight exposed parameter slots.
    pub fn pad_state(&self) -> (bool, [f32; 2]) {
        (self.pad_touched, self.last_touch_xy)
    }
}

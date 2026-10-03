#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use nts3::prelude::*;

const SAMPLE_RATE_HZ: usize = 48_000;

/// Maximum capture duration when chaos is fully raised.
pub const BUFFER_SECONDS: usize = 2;
/// Longest grain. Keep the `GRAIN LENGTH` parameter's 1000 ms maximum in sync.
pub const MAX_GRAIN_SECONDS: f32 = 1.0;
/// Grain density: 1 is adjacent, 2 is 50% overlap, and 0.5 leaves a grain-sized gap.
pub const GRAIN_AMOUNT: f32 = 2.0;
/// Dry/frozen transition time used at both touch edges.
pub const CROSSFADE_MILLISECONDS: f32 = 5.0;

const BUFFER_FRAMES: usize = SAMPLE_RATE_HZ * BUFFER_SECONDS;
const MIN_GRAIN_MILLISECONDS: f32 = 50.0;
// Two voices cover steady 2x density; the third keeps parameter movement from
// dropping a spawn while grains created at the previous length finish.
const MAX_ACTIVE_GRAINS: usize = 3;
const RNG_SEED: u32 = 0x6d2b_79f5;
// At 2x density the two half-overlapped polynomial windows sum to at most 1.
// Unity therefore prevents random grains from intermittently overshooting just
// because unrelated peaks align. The previous mean-normalized gain was 1.09375
// and could produce harsh clipping, especially at maximum chaos.
const GRAIN_OUTPUT_GAIN: f32 = 1.0;

const _: () = assert!(GRAIN_AMOUNT > 0.0 && GRAIN_AMOUNT <= 2.0);
const _: () = assert!(MAX_GRAIN_SECONDS > 0.0 && MAX_GRAIN_SECONDS < BUFFER_SECONDS as f32);

#[derive(Nts3Parameters)]
pub struct GranularFreezeParameters {
    #[parameter(
        name = "GRAIN LENGTH",
        min = 50,
        max = 1000,
        default = 100,
        parameter_type = "milliseconds",
        assign = "x",
        curve = "exp",
        mapping_min = 50,
        mapping_max = 1000,
        mapping_default = 100
    )]
    pub grain_milliseconds: Parameter,

    #[parameter(
        name = "CHAOS",
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
    pub chaos: Parameter,
}

#[derive(Clone, Copy)]
struct Grain {
    active: bool,
    read_index: usize,
    remaining: usize,
    phase: f32,
    phase_step: f32,
}

impl Grain {
    const fn inactive() -> Self {
        Self {
            active: false,
            read_index: 0,
            remaining: 0,
            phase: 0.0,
            phase_step: 0.0,
        }
    }
}

pub struct GranularFreezePlug {
    // The 768 KiB stereo history is allocated once in external SDRAM during
    // construction. Rendering, touch events, and resets never allocate.
    history: Vec<[f32; 2]>,
    write_index: usize,
    valid_frames: usize,
    grains: [Grain; MAX_ACTIVE_GRAINS],
    samples_until_grain: usize,
    grains_ready: bool,
    random_state: u32,
    pad_touched: bool,
    freeze_mix: f32,
    crossfade_step: f32,
}

impl Default for GranularFreezePlug {
    fn default() -> Self {
        Self {
            history: alloc::vec![[0.0; 2]; BUFFER_FRAMES],
            write_index: 0,
            valid_frames: 0,
            grains: [Grain::inactive(); MAX_ACTIVE_GRAINS],
            samples_until_grain: 0,
            grains_ready: false,
            random_state: RNG_SEED,
            pad_touched: false,
            freeze_mix: 0.0,
            crossfade_step: 1.0 / (SAMPLE_RATE_HZ as f32 * CROSSFADE_MILLISECONDS * 0.001),
        }
    }
}

// Non-reserved example IDs; replace the developer ID before distribution.
#[nts3::plugin(
    name = "Granular Freeze",
    developer_id = 0x4652_5348,
    unit_id = 0x4746_5245,
    sdram_bytes = 820_000
)]
impl Nts3Plugin for GranularFreezePlug {
    type Parameters = GranularFreezeParameters;

    fn initialize(&mut self, context: &InitContext<'_>) -> Result<(), InitError> {
        if context.sample_rate_hz() != SAMPLE_RATE_HZ as u32 {
            return Err(InitError::SampleRate);
        }
        self.crossfade_step =
            1.0 / (context.sample_rate_hz() as f32 * CROSSFADE_MILLISECONDS * 0.001);
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        // Old samples need not be cleared: valid_frames prevents them from being
        // addressed until live input has overwritten them.
        self.write_index = 0;
        self.valid_frames = 0;
        self.grains_ready = false;
        self.random_state = RNG_SEED;
        self.pad_touched = false;
        self.freeze_mix = 0.0;
        self.reset_grains();
    }

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        let requested_length = grain_frames(parameters.grain_milliseconds.plain());
        let chaos = parameters.chaos.normalized();
        let capture_length = capture_frames(requested_length, chaos);
        if self.grains_ready {
            // React quickly when X moves toward a shorter grain without
            // restarting voices on every small touch-coordinate change.
            self.samples_until_grain = self.samples_until_grain.min(grain_hop(requested_length));
        }

        for mut frame in buffer.frames_mut() {
            let input = frame.input();

            if !self.pad_touched && self.freeze_mix == 0.0 {
                // Firmware 1.1 gates the ordinary render input while the effect
                // is off, and its get_raw_input hook is not usable. Pass through
                // exactly here; a new touch starts a fresh two-second capture.
                frame.write(input);
                continue;
            }

            if self.pad_touched {
                if self.valid_frames < capture_length {
                    // At zero chaos, capture exactly one grain and freeze its
                    // start immediately. Raising chaos lengthens the capture,
                    // reaching the full two seconds only at maximum chaos.
                    self.record(input);
                }
                if self.valid_frames < requested_length {
                    // Do not synthesize progressively changing partial grains.
                    // If X asks for a longer grain, return to live monitoring
                    // only until the missing samples have been captured.
                    if self.grains_ready {
                        self.grains_ready = false;
                        self.freeze_mix = 0.0;
                        self.reset_grains();
                    }
                    frame.write(input);
                    continue;
                }
                if !self.grains_ready {
                    self.grains_ready = true;
                    self.reset_grains();
                }
                self.freeze_mix = (self.freeze_mix + self.crossfade_step).min(1.0);
            } else if self.freeze_mix <= self.crossfade_step {
                self.freeze_mix = 0.0;
            } else {
                self.freeze_mix -= self.crossfade_step;
            }

            let wet = self.next_frozen_frame(requested_length, chaos);
            if self.freeze_mix == 1.0 {
                // The steady touched state needs no dry-path arithmetic.
                frame.write(wet);
            } else {
                let dry_mix = 1.0 - self.freeze_mix;
                frame.write([
                    input[0] * dry_mix + wet[0] * self.freeze_mix,
                    input[1] * dry_mix + wet[1] * self.freeze_mix,
                ]);
            }

            if !self.pad_touched && self.freeze_mix == 0.0 {
                self.reset_grains();
            }
        }
    }

    fn touch_event(&mut self, event: TouchEvent) {
        let active = event.is_active();
        if active && !self.pad_touched {
            // NTS-3 firmware 1.1 cannot provide pre-touch audio. Start a fresh
            // live capture on Began (not on Moved/Stationary). DSP freezes the
            // write head as soon as the grain/chaos-dependent span is present.
            self.write_index = 0;
            self.valid_frames = 0;
            self.grains_ready = false;
            self.random_state = RNG_SEED;
            self.freeze_mix = 0.0;
            self.reset_grains();
        }
        self.pad_touched = active;
    }
}

#[cfg(feature = "web")]
nts3_rs_wasm::export_wasm_effect!(GranularFreezePlug);

impl GranularFreezePlug {
    #[inline]
    fn record(&mut self, frame: [f32; 2]) {
        self.history[self.write_index] = frame;
        self.write_index += 1;
        if self.write_index == self.history.len() {
            self.write_index = 0;
        }
        self.valid_frames = (self.valid_frames + 1).min(self.history.len());
    }

    #[inline]
    fn next_frozen_frame(&mut self, grain_length: usize, chaos: f32) -> [f32; 2] {
        if grain_length == 0 {
            return [0.0; 2];
        }

        if self.samples_until_grain == 0 {
            self.spawn_grain(grain_length, chaos);
            self.samples_until_grain = grain_hop(grain_length);
        }
        self.samples_until_grain -= 1;

        let mut output = [0.0; 2];
        let history_length = self.history.len();
        for grain in &mut self.grains {
            if !grain.active {
                continue;
            }

            let gain = grain_window(grain.phase) * GRAIN_OUTPUT_GAIN;
            let sample = self.history[grain.read_index];
            output[0] += sample[0] * gain;
            output[1] += sample[1] * gain;

            grain.read_index += 1;
            if grain.read_index == history_length {
                grain.read_index = 0;
            }
            grain.remaining -= 1;
            grain.phase += grain.phase_step;
            if grain.remaining == 0 {
                grain.active = false;
            }
        }
        output
    }

    fn spawn_grain(&mut self, length: usize, chaos: f32) {
        let Some(slot) = self.grains.iter().position(|grain| !grain.active) else {
            return;
        };

        // Firmware 1.1 capture always begins at history index zero. At zero
        // chaos, every grain starts from that first captured segment. Chaos
        // widens a uniform range forward toward the newest complete segment.
        let available_later = self.valid_frames - length;
        let chaos_range = ((BUFFER_FRAMES - length) as f32 * chaos.clamp(0.0, 1.0)) as usize;
        let maximum_later = available_later.min(chaos_range);
        let later = if maximum_later == 0 {
            0
        } else {
            self.next_random() as usize % (maximum_later + 1)
        };
        let read_index = later;
        let (phase, phase_step) = if length == 1 {
            (0.5, 0.0)
        } else {
            (0.0, 1.0 / (length - 1) as f32)
        };

        self.grains[slot] = Grain {
            active: true,
            read_index,
            remaining: length,
            phase,
            phase_step,
        };
    }

    #[inline]
    fn next_random(&mut self) -> u32 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.random_state = value;
        value
    }

    fn reset_grains(&mut self) {
        for grain in &mut self.grains {
            *grain = Grain::inactive();
        }
        self.samples_until_grain = 0;
    }
}

#[inline]
fn grain_frames(milliseconds: f32) -> usize {
    let maximum_ms = MAX_GRAIN_SECONDS * 1000.0;
    let milliseconds = milliseconds.clamp(MIN_GRAIN_MILLISECONDS, maximum_ms);
    (milliseconds * (SAMPLE_RATE_HZ as f32 / 1000.0)) as usize
}

#[inline]
fn capture_frames(grain_length: usize, chaos: f32) -> usize {
    let available = BUFFER_FRAMES - grain_length;
    grain_length + (available as f32 * chaos.clamp(0.0, 1.0)) as usize
}

#[inline]
fn grain_hop(length: usize) -> usize {
    ((length as f32 / GRAIN_AMOUNT) + 0.5) as usize
}

/// Isolated grain shape, deliberately cheap to replace. This compact polynomial
/// approximates exp(-3*x^2) near its peak and reaches exact zero at both ends.
#[inline]
fn grain_window(phase: f32) -> f32 {
    let x = phase * 2.0 - 1.0;
    let base = (1.0 - x * x).max(0.0);
    base * base * base
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(
        plugin: &mut GranularFreezePlug,
        parameters: &mut GranularFreezeParameters,
        data: &mut [f32],
    ) {
        let mut buffer = StereoBuffer::from_interleaved_in_place(data).unwrap();
        plugin.process(parameters, &mut buffer);
    }

    #[test]
    fn polynomial_window_is_symmetric_gaussian_shaped_and_peak_safe() {
        assert_eq!(grain_window(0.0), 0.0);
        assert_eq!(grain_window(0.5), 1.0);
        assert_eq!(grain_window(1.0), 0.0);
        assert!((grain_window(0.25) - grain_window(0.75)).abs() < 1.0e-7);
        assert!(grain_window(0.25) < grain_window(0.375));

        for step in 0..=1_000 {
            let phase = step as f32 / 1_000.0;
            let other_phase = if phase < 0.5 {
                phase + 0.5
            } else {
                phase - 0.5
            };
            let combined = (grain_window(phase) + grain_window(other_phase)) * GRAIN_OUTPUT_GAIN;
            assert!(combined <= 1.0 + 1.0e-6, "phase {phase}: {combined}");
        }
    }

    #[test]
    fn untouched_audio_is_bit_exact_without_recording() {
        let mut plugin = GranularFreezePlug::default();
        plugin
            .initialize(&InitContext::for_host(48_000, 256, [1024, 1024]))
            .unwrap();
        let mut parameters = GranularFreezeParameters::default();
        let original = [0.25, -0.5, 0.75, -1.0, f32::from_bits(1), -0.0];
        let mut audio = original;
        process(&mut plugin, &mut parameters, &mut audio);
        assert_eq!(audio.map(f32::to_bits), original.map(f32::to_bits));
        assert_eq!(plugin.valid_frames, 0);
    }

    #[test]
    fn length_and_amount_determine_grain_schedule() {
        assert_eq!(
            grain_frames(1.0),
            (MIN_GRAIN_MILLISECONDS * (SAMPLE_RATE_HZ as f32 / 1000.0)) as usize
        );
        assert_eq!(grain_frames(100.0), 4_800);
        assert_eq!(grain_frames(10_000.0), 48_000);
        assert_eq!(grain_hop(4_800), 2_400);
        assert_eq!(capture_frames(2_400, 0.0), 2_400);
        assert_eq!(capture_frames(2_400, 1.0), BUFFER_FRAMES);
    }

    #[test]
    fn zero_chaos_uses_the_beginning_of_the_touch_capture() {
        let mut plugin = GranularFreezePlug::default();
        for index in 0..10 {
            plugin.record([index as f32; 2]);
        }
        plugin.spawn_grain(4, 0.0);
        let grain = plugin.grains.iter().find(|grain| grain.active).unwrap();
        assert_eq!(grain.read_index, 0);
        assert_eq!(grain.remaining, 4);
    }

    #[test]
    fn touch_freezes_history_and_release_returns_to_recording() {
        let mut plugin = GranularFreezePlug::default();
        plugin
            .initialize(&InitContext::for_host(48_000, 512, [1024, 1024]))
            .unwrap();
        let mut parameters = GranularFreezeParameters::default();
        plugin.touch_event(TouchEvent::new(
            0,
            TouchPhase::Began,
            [512, 0],
            [1024, 1024],
        ));
        let mut touched = [0.75_f32; 512];
        for _ in 0..20 {
            process(&mut plugin, &mut parameters, &mut touched);
            assert!(touched.iter().all(|sample| sample.is_finite()));
        }
        let frozen_write_index = plugin.write_index;
        assert_eq!(plugin.valid_frames, 4_800);
        assert_eq!(plugin.freeze_mix, 1.0);

        plugin.touch_event(TouchEvent::new(
            0,
            TouchPhase::Ended,
            [512, 0],
            [1024, 1024],
        ));
        let mut released = [0.125_f32; 600];
        process(&mut plugin, &mut parameters, &mut released);
        assert_eq!(plugin.freeze_mix, 0.0);
        assert_eq!(plugin.write_index, frozen_write_index);
        assert_eq!(&released[released.len() - 32..], &[0.125; 32]);
    }

    #[test]
    fn zero_chaos_freezes_as_soon_as_one_grain_is_captured() {
        let mut plugin = GranularFreezePlug::default();
        plugin
            .initialize(&InitContext::for_host(48_000, 256, [1024, 1024]))
            .unwrap();
        let mut parameters = GranularFreezeParameters::default();
        parameters.grain_milliseconds.set(50);
        parameters.chaos.set(0);
        plugin.touch_event(TouchEvent::new(
            0,
            TouchPhase::Began,
            [512, 0],
            [1024, 1024],
        ));
        let mut block = [0.25_f32; 512];
        for _ in 0..10 {
            process(&mut plugin, &mut parameters, &mut block);
        }
        assert_eq!(plugin.valid_frames, 2_400);
        assert_eq!(plugin.write_index, 2_400);

        for _ in 0..10 {
            process(&mut plugin, &mut parameters, &mut block);
        }
        assert_eq!(plugin.valid_frames, 2_400);
        assert_eq!(plugin.write_index, 2_400);
    }

    #[test]
    fn maximum_chaos_captures_the_full_two_seconds() {
        let mut plugin = GranularFreezePlug::default();
        plugin
            .initialize(&InitContext::for_host(48_000, 256, [1024, 1024]))
            .unwrap();
        let mut parameters = GranularFreezeParameters::default();
        parameters.chaos.set(1_000);
        plugin.touch_event(TouchEvent::new(
            0,
            TouchPhase::Began,
            [512, 1023],
            [1024, 1024],
        ));
        let mut block = [0.25_f32; 512];
        for _ in 0..(BUFFER_FRAMES / 256) {
            process(&mut plugin, &mut parameters, &mut block);
        }
        assert_eq!(plugin.valid_frames, BUFFER_FRAMES);
        assert_eq!(plugin.write_index, 0);

        process(&mut plugin, &mut parameters, &mut block);
        assert_eq!(plugin.write_index, 0);
        assert_eq!(plugin.valid_frames, BUFFER_FRAMES);
    }

    #[test]
    fn static_state_stays_small_and_history_lives_in_sdram() {
        // Host pointers make the Vec and grain indices larger than they are on
        // the 32-bit target; either representation remains tiny beside history.
        assert!(core::mem::size_of::<GranularFreezePlug>() < 256);
        let plugin = GranularFreezePlug::default();
        assert_eq!(
            plugin.history.len() * core::mem::size_of::<[f32; 2]>(),
            768_000
        );
    }
}

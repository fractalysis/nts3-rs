#![no_std]

use nts3::prelude::*;

/// Observable DSP fixture for touch lifecycle testing.
///
/// Left-channel gain identifies all five phases. Right-channel gain is the
/// normalized X position while active and zero while inactive, so an active
/// touch at `(0, 0)` remains distinguishable through the left channel from a
/// release at `(0, 0)`.
#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct TouchProbe {
    phase: Option<TouchPhase>,
    active: bool,
    normalized_position: [f32; 2],
}

// Temporary private parameter stub until the Task 05/06 parameter engine and
// derive replace this internal fixture implementation.
#[derive(Default)]
struct ProbeParameters;

impl nts3::__private::Sealed for ProbeParameters {}
impl Nts3Parameters for ProbeParameters {}

impl Nts3Plugin for TouchProbe {
    type Parameters = ProbeParameters;

    fn process(&mut self, _parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        let left_gain = match self.phase {
            Some(TouchPhase::Began) => 0.25,
            Some(TouchPhase::Moved) => 0.5,
            Some(TouchPhase::Stationary) => 1.0,
            Some(TouchPhase::Ended) => 0.0,
            Some(TouchPhase::Cancelled) => -0.25,
            None => 0.0,
        };
        let right_gain = if self.active {
            self.normalized_position[0]
        } else {
            0.0
        };

        for mut frame in buffer.frames_mut() {
            let [left, right] = frame.input();
            frame.write([left * left_gain, right * right_gain]);
        }
    }

    fn touch_event(&mut self, event: TouchEvent) {
        self.phase = Some(event.phase());
        self.active = event.is_active();
        self.normalized_position = event.normalized_position();
    }
}

impl TouchProbe {
    #[allow(dead_code)]
    pub(crate) const fn state(&self) -> (Option<TouchPhase>, bool, [f32; 2]) {
        (self.phase, self.active, self.normalized_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_touch_and_release_are_observably_distinct() {
        let mut probe = TouchProbe::default();
        probe.touch_event(TouchEvent::new(0, TouchPhase::Began, [0, 0], [1024, 1024]));
        assert_eq!(probe.state(), (Some(TouchPhase::Began), true, [0.0, 0.0]));

        probe.touch_event(TouchEvent::new(0, TouchPhase::Ended, [0, 0], [1024, 1024]));
        assert_eq!(probe.state(), (Some(TouchPhase::Ended), false, [0.0, 0.0]));
    }

    #[test]
    fn all_phase_encodings_are_distinct() {
        let phases = [
            TouchPhase::Began,
            TouchPhase::Moved,
            TouchPhase::Ended,
            TouchPhase::Stationary,
            TouchPhase::Cancelled,
        ];
        let mut gains = [0.0; 5];
        for (index, phase) in phases.into_iter().enumerate() {
            let mut probe = TouchProbe::default();
            probe.touch_event(TouchEvent::new(0, phase, [512, 0], [1024, 1024]));
            gains[index] = match probe.phase {
                Some(TouchPhase::Began) => 0.25,
                Some(TouchPhase::Moved) => 0.5,
                Some(TouchPhase::Ended) => 0.0,
                Some(TouchPhase::Stationary) => 1.0,
                Some(TouchPhase::Cancelled) => -0.25,
                None => unreachable!(),
            };
        }
        assert_eq!(gains, [0.25, 0.5, 0.0, 1.0, -0.25]);
    }
}

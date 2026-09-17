use nts3_sys::{
    UNIT_TOUCH_PHASE_BEGAN, UNIT_TOUCH_PHASE_CANCELLED, UNIT_TOUCH_PHASE_ENDED,
    UNIT_TOUCH_PHASE_MOVED, UNIT_TOUCH_PHASE_STATIONARY,
};

/// Every touch lifecycle phase provided by the NTS-3 SDK.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchPhase {
    Began,
    Moved,
    Ended,
    Stationary,
    Cancelled,
}

impl TouchPhase {
    pub const fn from_raw(phase: u8) -> Option<Self> {
        match phase {
            UNIT_TOUCH_PHASE_BEGAN => Some(Self::Began),
            UNIT_TOUCH_PHASE_MOVED => Some(Self::Moved),
            UNIT_TOUCH_PHASE_ENDED => Some(Self::Ended),
            UNIT_TOUCH_PHASE_STATIONARY => Some(Self::Stationary),
            UNIT_TOUCH_PHASE_CANCELLED => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Began | Self::Moved | Self::Stationary)
    }
}

/// A touch event with phase and coordinates kept as independent state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchEvent {
    id: u8,
    phase: TouchPhase,
    raw: [u32; 2],
    area: [u32; 2],
}

impl TouchEvent {
    pub const fn new(id: u8, phase: TouchPhase, raw: [u32; 2], area: [u32; 2]) -> Self {
        Self {
            id,
            phase,
            raw,
            area,
        }
    }

    pub const fn id(self) -> u8 {
        self.id
    }

    pub const fn phase(self) -> TouchPhase {
        self.phase
    }

    /// Activity is derived only from phase, never from coordinates.
    pub const fn is_active(self) -> bool {
        self.phase.is_active()
    }

    pub const fn raw_position(self) -> [u32; 2] {
        self.raw
    }

    pub const fn touch_area(self) -> [u32; 2] {
        self.area
    }

    /// Coordinates clamped to `0..dimension`, with zero dimensions mapped to 0.
    pub const fn clamped_position(self) -> [u32; 2] {
        [
            clamp_axis(self.raw[0], self.area[0]),
            clamp_axis(self.raw[1], self.area[1]),
        ]
    }

    /// Coordinates normalized to 0.0..=1.0 using each area's last valid index.
    /// Zero- and one-sized malformed dimensions normalize to 0.0.
    pub fn normalized_position(self) -> [f32; 2] {
        [
            normalize_axis(self.raw[0], self.area[0]),
            normalize_axis(self.raw[1], self.area[1]),
        ]
    }
}

const fn clamp_axis(position: u32, dimension: u32) -> u32 {
    if dimension == 0 {
        0
    } else {
        let maximum = dimension - 1;
        if position > maximum {
            maximum
        } else {
            position
        }
    }
}

fn normalize_axis(position: u32, dimension: u32) -> f32 {
    if dimension <= 1 {
        0.0
    } else {
        clamp_axis(position, dimension) as f32 / (dimension - 1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phase_preserves_position_and_has_explicit_activity() {
        let cases = [
            (TouchPhase::Began, true),
            (TouchPhase::Moved, true),
            (TouchPhase::Ended, false),
            (TouchPhase::Stationary, true),
            (TouchPhase::Cancelled, false),
        ];
        for (phase, active) in cases {
            let event = TouchEvent::new(0, phase, [0, 0], [1024, 1024]);
            assert_eq!(event.is_active(), active);
            assert_eq!(event.raw_position(), [0, 0]);
            assert_eq!(event.clamped_position(), [0, 0]);
            assert_eq!(event.normalized_position(), [0.0, 0.0]);
        }
    }

    #[test]
    fn nonstandard_zero_and_out_of_range_dimensions_are_safe() {
        let event = TouchEvent::new(7, TouchPhase::Moved, [500, u32::MAX], [501, 200]);
        assert_eq!(event.id(), 7);
        assert_eq!(event.touch_area(), [501, 200]);
        assert_eq!(event.clamped_position(), [500, 199]);
        assert_eq!(event.normalized_position(), [1.0, 1.0]);

        let malformed = TouchEvent::new(0, TouchPhase::Began, [99, 88], [0, 1]);
        assert_eq!(malformed.clamped_position(), [0, 0]);
        assert_eq!(malformed.normalized_position(), [0.0, 0.0]);
    }

    #[test]
    fn raw_phase_conversion_rejects_unknown_values() {
        assert_eq!(TouchPhase::from_raw(0), Some(TouchPhase::Began));
        assert_eq!(TouchPhase::from_raw(4), Some(TouchPhase::Cancelled));
        assert_eq!(TouchPhase::from_raw(5), None);
        assert_eq!(TouchPhase::from_raw(u8::MAX), None);
    }
}

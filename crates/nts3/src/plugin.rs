use crate::{StereoBuffer, TouchEvent};

/// Immutable initialization information copied from the SDK descriptor.
pub struct InitContext<'runtime> {
    pub(crate) sample_rate_hz: u32,
    pub(crate) maximum_frames: usize,
    pub(crate) touch_area: [u32; 2],
    pub(crate) _lifetime: core::marker::PhantomData<&'runtime ()>,
}

impl InitContext<'_> {
    /// Constructs initialization data for non-hardware hosts and adapters.
    #[doc(hidden)]
    pub const fn for_host(
        sample_rate_hz: u32,
        maximum_frames: usize,
        touch_area: [u32; 2],
    ) -> Self {
        Self {
            sample_rate_hz,
            maximum_frames,
            touch_area,
            _lifetime: core::marker::PhantomData,
        }
    }

    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub const fn maximum_frames(&self) -> usize {
        self.maximum_frames
    }

    pub const fn touch_area(&self) -> [u32; 2] {
        self.touch_area
    }
}

/// Plugin-requested initialization failure mapped directly to an SDK code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitError {
    Target,
    ApiVersion,
    SampleRate,
    Geometry,
    Memory,
    Undefined,
}

impl InitError {
    pub(crate) const fn sdk_code(self) -> i8 {
        match self {
            Self::Target => nts3_sys::UNIT_ERR_TARGET,
            Self::ApiVersion => nts3_sys::UNIT_ERR_API_VERSION,
            Self::SampleRate => nts3_sys::UNIT_ERR_SAMPLERATE,
            Self::Geometry => nts3_sys::UNIT_ERR_GEOMETRY,
            Self::Memory => nts3_sys::UNIT_ERR_MEMORY,
            Self::Undefined => nts3_sys::UNIT_ERR_UNDEF,
        }
    }
}

/// Safe, statically dispatched NTS-3 plugin contract.
pub trait Nts3Plugin: Default + 'static {
    type Parameters: crate::Nts3Parameters;

    fn initialize(&mut self, _context: &InitContext<'_>) -> Result<(), InitError> {
        Ok(())
    }

    fn reset(&mut self) {}

    fn resume(&mut self) {}

    fn suspend(&mut self) {}

    fn teardown(&mut self) {}

    fn process(&mut self, parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>);

    fn touch_event(&mut self, _event: TouchEvent) {}

    fn tempo_changed(&mut self, _bpm: f32) {}

    fn tempo_4ppqn_tick(&mut self, _counter: u32) {}
}

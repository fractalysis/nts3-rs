#![no_std]

//! Minimal, binding-free WebAssembly ABI for NTS-3 effects.
//!
//! Each compiled effect exports a fixed 128-frame stereo buffer plus lifecycle,
//! parameter, and render functions. The browser creates a separate WebAssembly
//! instance for every chain slot, so plugin state is never shared between slots.

use core::cell::UnsafeCell;

use nts3::{InitContext, Nts3Parameters, Nts3Plugin, StereoBuffer};

const MAX_FRAMES: usize = 128;
const CHANNELS: usize = 2;

struct Effect<P: Nts3Plugin> {
    plugin: P,
    parameters: P::Parameters,
}

impl<P: Nts3Plugin> Effect<P> {
    fn new(sample_rate_hz: u32) -> Option<Self> {
        let mut parameters = P::Parameters::default();
        parameters.initialize_smoothers(sample_rate_hz as f32);
        parameters.reset_smoothers();
        let mut plugin = P::default();
        let context = InitContext::for_host(sample_rate_hz, MAX_FRAMES, [1024, 1024]);
        plugin.initialize(&context).ok()?;
        Some(Self { plugin, parameters })
    }

    fn set_parameter(&mut self, index: u8, value: i32) {
        self.parameters.set(index, value);
    }

    fn reset(&mut self) {
        self.parameters.reset_smoothers();
        self.plugin.reset();
    }

    fn render(&mut self, samples: &mut [f32]) {
        let Ok(mut buffer) = StereoBuffer::from_interleaved_in_place(samples) else {
            return;
        };
        self.parameters.begin_block();
        self.plugin.process(&mut self.parameters, &mut buffer);
        self.parameters.end_block();
    }
}

/// Storage behind the common raw-Wasm exports generated for an effect crate.
#[doc(hidden)]
pub struct WasmEffectRuntime<P: Nts3Plugin> {
    effect: UnsafeCell<Option<Effect<P>>>,
    samples: UnsafeCell<[f32; MAX_FRAMES * CHANNELS]>,
}

// One AudioWorklet invokes one WebAssembly instance serially. Every instance
// owns distinct Wasm globals, including this runtime and its plugin state.
unsafe impl<P: Nts3Plugin> Sync for WasmEffectRuntime<P> {}

impl<P: Nts3Plugin> WasmEffectRuntime<P> {
    pub const fn new() -> Self {
        Self {
            effect: UnsafeCell::new(None),
            samples: UnsafeCell::new([0.0; MAX_FRAMES * CHANNELS]),
        }
    }

    pub fn initialize(&self, sample_rate_hz: u32) -> bool {
        // SAFETY: the AudioWorklet callback contract is serialized.
        unsafe {
            *self.effect.get() = Effect::new(sample_rate_hz);
            (*self.effect.get()).is_some()
        }
    }

    pub fn sample_pointer(&self) -> *mut f32 {
        self.samples.get().cast::<f32>()
    }

    pub fn set_parameter(&self, index: u8, value: i32) {
        // SAFETY: the AudioWorklet callback contract is serialized.
        if let Some(effect) = unsafe { &mut *self.effect.get() } {
            effect.set_parameter(index, value);
        }
    }

    pub fn reset(&self) {
        // SAFETY: the AudioWorklet callback contract is serialized.
        if let Some(effect) = unsafe { &mut *self.effect.get() } {
            effect.reset();
        }
    }

    pub fn render(&self, frames: u32) {
        let frames = frames as usize;
        if frames > MAX_FRAMES {
            return;
        }
        // SAFETY: the fixed buffer contains MAX_FRAMES stereo frames and no
        // pointer or slice survives this serialized call.
        unsafe {
            let Some(effect) = (&mut *self.effect.get()).as_mut() else {
                return;
            };
            let storage = &mut *self.samples.get();
            effect.render(&mut storage[..frames * CHANNELS]);
        }
    }
}

impl<P: Nts3Plugin> Default for WasmEffectRuntime<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// Exports the common browser ABI for one concrete NTS-3 plugin type.
#[macro_export]
macro_rules! export_wasm_effect {
    ($plugin:ty) => {
        static __NTS3_WASM_RUNTIME: $crate::WasmEffectRuntime<$plugin> =
            $crate::WasmEffectRuntime::new();

        #[unsafe(no_mangle)]
        pub extern "C" fn web_init(sample_rate_hz: u32) -> i32 {
            i32::from(__NTS3_WASM_RUNTIME.initialize(sample_rate_hz))
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn web_audio_buffer() -> *mut f32 {
            __NTS3_WASM_RUNTIME.sample_pointer()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn web_set_parameter(index: u32, value: i32) {
            if let Ok(index) = u8::try_from(index) {
                __NTS3_WASM_RUNTIME.set_parameter(index, value);
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn web_reset() {
            __NTS3_WASM_RUNTIME.reset();
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn web_process(frames: u32) {
            __NTS3_WASM_RUNTIME.render(frames);
        }
    };
}

# nts3-rs-wasm

A small raw-WebAssembly adapter for running an `nts3::Nts3Plugin` in a browser
`AudioWorklet`. It intentionally has no `wasm-bindgen` dependency. Every effect
exports the same ABI:

- `web_init(sample_rate_hz) -> 1 | 0`
- `web_audio_buffer() -> *mut f32` (128 interleaved stereo frames)
- `web_set_parameter(index, raw_value)`
- `web_reset()`
- `web_process(frame_count)`

Each AudioWorklet chain slot creates a separate Wasm instance, so repeated uses
of the same effect have independent DSP and parameter state.

## Adding an effect

1. Add an optional dependency on `nts3-rs-wasm` and a `web` feature to the
   effect crate.
2. Set its library crate types to `['rlib', 'cdylib']`.
3. Add `#[cfg(feature = "web")] nts3_rs_wasm::export_wasm_effect!(YourPlugin);`.
4. Add the package/library/output name to `../../web/build.mjs`.
5. Install the pinned toolchain target once with
   `rustup target add wasm32-unknown-unknown --toolchain 1.98.1`.
6. Run `node external/nts3-rs/web/build.mjs` from the website repository.

The generated `.wasm` files are copied into the website's `assets/` directory.
They are committed assets; building the website itself does not require Rust.

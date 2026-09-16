# Research notes and resolved questions

## SDK sources examined

- `external/logue-sdk/platform/nts-3_kaoss/README.md`
- `external/logue-sdk/platform/nts-3_kaoss/common/runtime.h`
- `external/logue-sdk/platform/nts-3_kaoss/common/unit.h`
- `external/logue-sdk/platform/nts-3_kaoss/common/unit_genericfx.h`
- `external/logue-sdk/platform/nts-3_kaoss/common/_unit_base.c`
- `external/logue-sdk/platform/nts-3_kaoss/dummy-genericfx/{header.c,unit.cc,effect.h,Makefile,config.mk}`
- `external/logue-sdk/platform/nts-3_kaoss/ld/{unit.ld,rules.ld}`

The supplied Docker image successfully built the C++ `dummy-genericfx` baseline.
It contains the ARM toolchain but not Rust, motivating a derived pinned image.

## Touch representation

Touch activity is not inferred from X/Y or represented as an ordinary mapped
parameter. The SDK calls:

```c
void unit_touch_event(uint8_t id, uint8_t phase, uint32_t x, uint32_t y);
```

NTS-3 is single-touch (`id == 0`). Began/moved/stationary are active;
ended/cancelled terminate activity. Runtime context supplies touch area width
and height. Therefore activity and position must remain independent fields in
the Rust model.

## Rust/FunDSP feasibility

The original plugin uses FunDSP 0.15 and NIH-plug. A direct target check fails
because those versions/dependencies require `std`.

Newer FunDSP added `no_std` support using `default-features = false` while still
requiring `alloc` for allocating nodes. A minimal crate containing the exact
Smooth Echo `Tap<U1>` and `Highpole<f32, U1>` types was successfully checked for
`thumbv7em-none-eabihf` with FunDSP 0.19.1, 0.20.0 and 0.21.0. Tests of crates.io
0.22.0 and 0.23.0 failed in `resample.rs` due to missing `alloc::vec`/`Vec`
imports; current upstream also had additional missing alloc imports during the
check. Pin 0.21.0 until upstream/patched versions are proven.

FunDSP's `Tap` owns a `Vec<f32>`. At two seconds and 48 kHz it computes roughly
96,003 samples plus interpolation/SIMD padding, then rounds to the next power of
two: 131,072 samples = 524,288 bytes per channel. Two channels use 1,048,576
bytes. This is why the example explicitly declares a 1,100,000-byte arena and
why initialization high-water is an acceptance gate.

Rust fields themselves are ordinary scalar/layout storage. The proposed runtime
uses static dispatch and no object headers/GC. The likely static state cost is
small; the risks are dynamically allocated delay payload and monomorphized code
size, both measured by the plan.

## Existing Rust logue precedent and risk

A public Rust NTS-1 mkII experiment demonstrates `no_std`, PIC Rust and Rust
unit headers, but documents Thumb PLT problems with both LLD and GNU ld until a
GNU `--long-plt` path was used. NTS-3 must not assume this transfers unchanged.
The pass-through hardware gate and relocation/PLT inspection are mandatory.
Calling SDK hooks indirectly from the runtime descriptor should minimize
unresolved PLT calls.

## Baseplug code reuse

The parameter smoother will be adapted from wrl/baseplug at pinned commit
[`9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74`](https://github.com/wrl/baseplug/tree/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74), specifically
[`src/smooth.rs`](https://github.com/wrl/baseplug/blob/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74/src/smooth.rs#L10-L162).
Baseplug is `MIT OR Apache-2.0`; this project will use the MIT grant, retain its
license notice, cite the pinned source, and mark modifications.

The useful core is a one-pole exponential smoother, not a linear ramp:
`b = exp(-1/(ms * sample_rate/1000))`, `a = 1-b`, and
`y[n] = target*a + y[n-1]*b`, with a `1e-5` settle threshold and
Active/Deactivating/Inactive states. The NTS-3 adaptation should be f32-only,
`no_std`, use `libm::expf`, and expose per-sample `next_*` methods instead of
Baseplug's fixed `MAX_BLOCKSIZE` output array. Host golden tests must compare the
adaptation with the original algorithm.

Baseplug's `parameter.rs` normalized gradient translations and `declick.rs` are
also approved sources if they reduce target code/API complexity. They must be
reviewed rather than copied blindly: both currently use desktop/std-oriented
surrounding APIs, while NTS-3 already provides hardware mapping curves. Do not
import Baseplug's plugin host/model/serialization architecture.

## Why not NIH-plug

NIH-plug's `Plugin`, `Params` and `Buffer` design depends on `std`, `Arc`,
`Vec`, desktop event/background infrastructure and host-specific wrappers.
Its parameter setters are also framework-internal. Porting it would add large
code and semantic surface under a 32 KiB target budget. The NTS-3 facade instead
retains familiar ergonomics—typed plugin trait, derived parameters, smoothing
and export attribute—without carrying desktop abstractions.

## Open items that implementation must resolve experimentally

- Exact stripped Rust/FunDSP code size under Korg's loader limits.
- Exact Rust/GNU link flags and whether `--long-plt` is needed when all Korg APIs
  are reached via function pointers.
- Whether callbacks can be concurrent/preempt one another. The official C++
  template uses plain mutable fields, so MVP follows that model pending contrary
  evidence.
- Real hardware CPU and stack headroom.
- Availability of an NTS-3 to the agent; no automated substitute proves loader
  or touch behavior.

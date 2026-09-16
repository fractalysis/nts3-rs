# Architecture and public contract

## Design principles and patterns

Use these established patterns deliberately:

- **Facade:** `nts3::prelude` is the only import most authors need. It reexports
  safe types and proc macros while hiding raw ABI details.
- **Adapter:** generated `extern "C"` functions adapt Korg's callback ABI to a
  typed `Nts3Plugin` implementation.
- **Template Method:** `Runtime<P>` owns fixed initialization/teardown/parameter
  behavior and invokes optional plugin hooks. Plugins cannot accidentally omit
  required SDK steps.
- **Composition over inheritance:** `Runtime<P>` owns `P::Parameters`; plugin
  types contain only DSP/device state. This removes NIH-style `Arc` and the
  repetitive `params()` method.
- **Static polymorphism:** generic `Runtime<P>` is monomorphized. Do not use
  trait objects or heap allocation for framework dispatch.
- **Newtypes and typed enums:** unsafe integers/pointers remain in `nts3-sys`;
  the public API uses `TouchPhase`, `TouchEvent`, `StereoBuffer`, and validated
  parameter metadata.
- **State machine / typestate:** allocator/runtime states are `Uninitialized`,
  `Initializing`, `Ready`, `Suspended`, and `TearingDown`. Reject invalid
  transitions and seal allocation before audio starts.
- **RAII at the boundary:** initialize plugin state only after SDRAM succeeds;
  drop it before returning the arena to Korg. The process-global singleton is
  unavoidable because the SDK ABI is global, but it must be isolated inside the
  generated adapter.
- **Fail fast at compile/build time:** macros validate metadata and `cargo-nts3`
  validates artifacts. Avoid runtime checks in the audio loop.

Do not introduce an abstract host model copied from desktop plugin frameworks.
The NTS-3 has one fixed stereo geometry and a small callback API.

## Normative author API

The checked-in Smooth Echo source is the API acceptance fixture. The essential
trait should remain close to:

```rust
pub trait Nts3Plugin: Default + 'static {
    type Parameters: Nts3Parameters;

    fn initialize(&mut self, context: &InitContext<'_>)
        -> Result<(), InitError> { Ok(()) }
    fn reset(&mut self) {}
    fn resume(&mut self) {}
    fn suspend(&mut self) {}
    fn teardown(&mut self) {}

    fn process(
        &mut self,
        parameters: &mut Self::Parameters,
        buffer: &mut StereoBuffer<'_>,
    );

    fn touch_event(&mut self, event: TouchEvent) {}
    fn tempo_changed(&mut self, bpm: f32) {}
    fn tempo_4ppqn_tick(&mut self, counter: u32) {}
}
```

Only `type Parameters` and `process` are required. Stereo layout, parameter
routing, target/API checks, exports, defaults, panic handling, and allocation
must not be repeated by users.

A single attribute on the concrete impl supplies only unit-specific metadata
and emits the rest:

```rust
#[nts3::plugin(
    name = "Smooth Echo",
    developer_id = 0x4652_5348,
    unit_id = 0x5345_4348,
    sdram_bytes = 1_100_000
)]
impl Nts3Plugin for EchoPlug { /* hooks */ }
```

The macro reads `CARGO_PKG_VERSION_MAJOR/MINOR/PATCH`; no repeated version or
`nts3_export!` line is required. The platform target, API 2.0, stereo geometry,
header section, panic handler, and callback names are framework constants.

## Parameter model

`#[derive(Nts3Parameters)]` must generate:

- `Default` initialized from field metadata.
- A constant `[unit_param_t; 8]`, padded with SDK `none` descriptors.
- A constant `[genericfx_param_mapping_t; 8]`, also padded.
- Safe `set(index, i32)`, `get(index)`, sample-rate initialization, smoothing
  reset, and optional string formatting dispatch.
- Compile-time parameter count and metadata for the unit header.

MVP field types:

- `Parameter`: integer target value with `plain() -> f32`, `raw() -> i16`, and
  `normalized() -> f32`.
- `SmoothedParameter`: target plus a one-pole smoother adapted from
  [wrl/baseplug `smooth.rs`](https://github.com/wrl/baseplug/blob/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74/src/smooth.rs#L10-L162), with
  `next_plain()` and `next_normalized()`.

Preserve Baseplug's core behavior and names where they fit: `SmoothStatus`
(`Inactive`, `Active`, `Deactivating`), target/reset semantics, absolute settle
epsilon `0.00001`, and coefficients
`b = exp(-1 / (ms * sample_rate / 1000))`, `a = 1 - b`, followed per sample by
`y = target * a + previous * b`. Here `ms` is the filter time constant, not a
linear ramp duration; after one `ms` interval approximately 36.8% of the initial
error remains. Snap to the exact target only after the settle threshold, and
handle `ms <= 0` explicitly as immediate.

Adapt rather than retaining Baseplug's fixed `MAX_BLOCKSIZE` output array: NTS-3
plugins consume one value per sample through `next_*`, so store only target,
coefficients, last output, the first output of the current render block, and
status. Generated parameter code provides private `begin_block/end_block`
hooks called by `Runtime<P>` around `process`; `end_block` performs Baseplug's
`update_status` check against that block's first output. This preserves output
and Active → Deactivating → Inactive behavior for the same block partitioning
without reserving an output array per smoothed parameter. Use a `no_std`
exponential implementation such as `libm::expf`. Add golden tests comparing
output sequences, resets, retargeting and status transitions against the
unmodified Baseplug algorithm. Include the Baseplug MIT notice and pinned
source/commit attribution in copied source and project third-party notices.

The runtime has exclusive access to plugin and parameters during callbacks, as
in Korg's C++ template. Plain fields are preferable to atomics unless hardware
or SDK evidence demonstrates concurrent callbacks. This is both simpler and
smaller. Add synchronization only after a test proving it is needed.

Macro validation must cover:

- At most eight parameters.
- `i16` representability; `min <= center/default <= max` as applicable.
- Mapping min/max/default inside descriptor bounds.
- Unit name <=19 and parameter name <=21 valid 7-bit SDK characters.
- Supported display type, decimal/fixed fraction encoding, assignment and
  curve names.
- No reserved developer ID, including any case spelling of KORG.
- Version components fitting the Korg format.

Declaration order is the default parameter index for simplicity. Support an
optional explicit `index` for authors who need reorder-safe APIs, and diagnose
duplicates/gaps. Document that changing implicit order is preset-breaking.

## Touch model

Represent every SDK phase, including cancellation:

```rust
pub enum TouchPhase { Began, Moved, Ended, Stationary, Cancelled }

pub struct TouchEvent {
    id: u8,
    phase: TouchPhase,
    raw: [u32; 2],
    area: [u32; 2],
}
```

Provide `is_active()`, `raw_position()`, and `normalized_position()`. Active is
true for began/moved/stationary and false for ended/cancelled. Clamp malformed
coordinates and avoid division by zero. Never infer activity from coordinates.
The NTS host already maps X/Y controls to assigned parameters; do not duplicate
that mapping in the touch callback.

An internal touch-derived control does not consume a parameter slot. A future
opt-in `#[touch]` parameter source may be added, but it is not required for MVP.

## Safe audio buffer

Korg allows input/output to overlap completely or not at all. `StereoBuffer`
must hide raw pointers and expose a no-allocation frame iterator:

```rust
for mut frame in buffer.frames_mut() {
    let [left, right] = frame.input(); // copied values before output mutation
    frame.write([new_left, new_right]);
}
```

Internally use a small, audited unsafe iterator that yields each output frame
once. Test separate, exact in-place, zero-frame, one-frame and maximum-frame
cases. Reject unsupported channel geometry during `unit_init`; debug/test builds
should detect forbidden partial overlap.

Expose the NTS-3 raw-input hook through an optional buffer/context accessor,
refreshing its pointer every render call as required by the SDK.

## Runtime and allocation

Target runtime storage:

```text
MaybeUninit<Runtime<P>>
Runtime<P> = { plugin: P, parameters: P::Parameters, copied context }
```

Initialization sequence:

1. Validate non-null descriptor, target, API, 48 kHz/stereo geometry and hooks.
2. Validate declared SDRAM budget against `sdram_avail` and the 3 MiB platform
   ceiling.
3. Allocate one arena from `sdram_alloc`; initialize an aligned bump allocator.
4. Construct generated parameter defaults, then `P::default()` (FunDSP allocates
   here), initialize smoothers, and call `P::initialize`.
5. Record allocator high-water and seal it. Any render-time allocation is a
   test failure and a target OOM fault.
6. Publish the runtime as ready only after all steps succeed.

Teardown calls the plugin hook, drops runtime values, resets allocator state,
and returns the original arena pointer using `sdram_free`. A bump allocator is
chosen because target allocations are initialization-only and the entire arena
has one lifetime. Deallocation can be a no-op while active. Account for all
transient initialization allocations in the high-water mark.

Host builds use a tracking allocator/probe to instantiate exactly the same
plugin and parameter types and report peak/current allocation. Reserve margin;
do not set the declared budget equal to one observed run.

For Smooth Echo, two two-second FunDSP delay buffers each round to 131,072
`f32`s, totaling 1,048,576 bytes. Scalar plugin/framework state should remain in
the hundreds of bytes; assert `size_of::<Runtime<EchoPlug>>()` in reports/tests.

## Raw ABI crate

`nts3-sys` should hand-define only the small API surface, using `repr(C)` or
`repr(C, packed)` exactly where the SDK does. Avoid bindgen as a required user
step. Add a Docker test that compiles a C layout probe against SDK headers and
compares every size/alignment/offset and constant to Rust.

Include:

- Runtime descriptor/hooks and genericfx context function pointers.
- Unit parameter/header/default mapping structures.
- Target/API/error/parameter/mapping/touch constants.

Do not expose packed-field references. Copy fields with unaligned-safe reads.

## Generated ABI adapter

The `#[nts3::plugin]` proc macro emits concrete, unmangled callbacks calling
`nts3::runtime::<EchoPlug>`:

- `unit_init`, teardown/reset/resume/suspend/render.
- parameter get/set/string.
- tempo and 4PPQN tick.
- touch event.
- `unit_header` in `.unit_header`.

Use Rust-2024-safe attributes (`#[unsafe(no_mangle)]`,
`#[unsafe(link_section = ...)]`). Ensure static-library dead stripping cannot
remove exports. The framework supplies target-only global allocator and panic
handler exactly once.

## Crate boundaries

- `nts3-sys`: no logic, no allocation, no macros.
- `nts3-macros`: host-only `syn`/`quote`; compile-time validation and codegen.
- `nts3`: public facade and target runtime. Keep dependencies minimal.
- `cargo-nts3`: host CLI, Docker orchestration, ELF inspection and reports.

This separation prevents proc-macro dependencies and ELF parser code from
entering the hardware artifact.

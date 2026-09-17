# Implementation plan: ergonomic Rust framework for Korg NTS-3 units

## Goal

Implement a reusable, minimal `no_std` Rust framework that turns an ordinary
Rust type implementing `Nts3Plugin` into a valid Korg NTS-3 `genericfx`
`.nts3unit`. The normative authoring example is
[`example/smooth-echo-nts3-plug`](../example/smooth-echo-nts3-plug). The agent
should make that example compile **without adding hand-written ABI callbacks,
headers, panic handlers, allocators, linker flags, or export invocations to the
plugin**.

This is a framework, procedural-macro layer, and build/inspection tool. It is
not a new compiler and it is not an NIH-plug backend. The original
[`example/smooth-echo-nih-plug`](../example/smooth-echo-nih-plug) remains the
behavioral reference only.

## Confirmed direction

- Use Rust's `thumbv7em-none-eabihf` target and the Korg GNU linker environment.
- Do not depend on or port NIH-plug.
- Use FunDSP in `no_std + alloc` mode. Pin `fundsp = 0.21.0` initially; the
  Smooth Echo node subset has been checked successfully for the target. Versions
  0.22 and 0.23 currently fail a no-default-features cross-check because of
  missing `alloc` imports.
- Base parameter smoothing on wrl/baseplug's MIT-licensed one-pole smoother at
  commit `9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74`, adapted to an allocation-free
  `no_std` f32/sample API and retained with attribution. An exact source/license
  snapshot and provenance are stored under [`plan/references`](references/README.md).
- Preserve touch phase separately from XY position.
- Optimize the public API for the common case. The framework owns parameters,
  lifecycle dispatch, ABI exports, unit header, allocator, panic behavior, and
  descriptor padding.
- Use static dispatch and concrete storage. No trait objects, `Arc`, locks,
  threads, JSON state, GUI, MIDI, or desktop host abstractions on target.

## Platform constraints

From [`external/logue-sdk/platform/nts-3_kaoss/README.md`](../external/logue-sdk/platform/nts-3_kaoss/README.md):

- Cortex-M7 / STM32H725, hard-float Thumb target.
- 48 kHz stereo `f32`, interleaved buffers.
- ELF32 little-endian ARM EABI5 shared object, System V OS ABI.
- SDK API 2.0.0 and `genericfx` target.
- Approximately 32 KiB maximum unit size and 32 KiB RAM load size.
- Up to 3 MiB external SDRAM per runtime.
- Eight integer-backed exposed parameter slots.
- Input/output buffers overlap completely or not at all.
- Required touch callback has `began`, `moved`, `ended`, `stationary`, and
  `cancelled` phases; coordinates are normally 0..1023 but dimensions must be
  read from runtime context.

Treat limits as build failures, not documentation suggestions. The final size
and hardware behavior must be measured; Rust scalar variables have no hidden
per-variable runtime metadata, but monomorphized code and library routines can
still exhaust the 32 KiB code/load budget.

## Deliverables

```text
Cargo.toml                         # workspace
rust-toolchain.toml                # pinned Rust toolchain
crates/
  nts3-sys/                        # audited raw ABI types/constants
  nts3-macros/                     # Params derive and plugin export attribute
  nts3/                            # safe facade, runtime, allocator, buffers
  nts3-cli/                        # internal container-side Rust tooling engine
nts3                               # public Bash build/inspect/doctor/new launcher
container/
  Dockerfile                       # pinned Korg image + pinned Rust
  README.md
  run.sh                           # lower-level Bash container helper
example/
  smooth-echo-nih-plug/            # unchanged reference
  smooth-echo-nts3-plug/           # must compile unchanged
fixtures/
  pass-through/                    # smallest possible unit/ABI fixture
  touch-probe/                     # observable touch lifecycle fixture
tests/
  abi/                             # C-vs-Rust size/offset checks
  golden/                          # ELF and DSP regression fixtures
docs/
  quickstart.md
  parameters.md
  memory.md
  troubleshooting.md
```

## Definition of done

1. `./container/run.sh cargo test --workspace` passes in the pinned container.
2. `./nts3 check -p smooth-echo-nts3-plug` succeeds with no `std` linkage.
3. `./nts3 build -p smooth-echo-nts3-plug --release` creates a stripped
   `.nts3unit`, linker map, and JSON/human memory reports reproducibly through
   the Docker environment.
4. `./nts3 inspect` validates ABI, header, exports, relocations, and all known
   memory limits; malformed fixture ELFs are rejected by tests.
5. The generated header exactly matches SDK C layout and all eight parameter
   descriptors/default mappings are correct.
6. The native initialization probe reports actual allocator high-water use
   below the declared 1,100,000-byte Smooth Echo SDRAM arena with a documented
   safety margin.
7. Rendering performs no allocation after initialization.
8. Separate and in-place buffers produce equivalent output.
9. `(x=0, y=0, touched)` and `(x=0, y=0, released)` are distinct in tests.
10. A real NTS-3 loads and runs the pass-through, touch-probe, and Smooth Echo
    units. If the implementing agent has no device, artifacts and the exact
    checklist in `validation.md` must be handed to the user and hardware status
    must remain explicitly marked unverified.

## Implementation order

The order is intentional. Do not build the ergonomic macro layer before proving
that a tiny Rust-generated ELF loads on hardware.

1. **Toolchain and ELF spike** — prove the complete binary path with manually
   written exports.
2. **Raw ABI crate** — freeze and test C-compatible definitions.
3. **Safe runtime core** — allocator, state machine, buffer and context adapters.
4. **Parameters and smoothing** — generated descriptors and dispatch.
5. **Macros** — erase boilerplate and emit concrete ABI shims.
6. **Build/inspection CLI** — reproducible linking, packaging and reporting.
7. **Smooth Echo integration** — compile, profile, optimize and compare DSP.
8. **Hardware and release hardening** — lifecycle, four-runtime, stress and
   documentation tests.

Detailed API and internal design are in [architecture.md](architecture.md).
Concrete requirements are in [implementation.md](implementation.md), validation
gates in [validation.md](validation.md), and the ordered, zero-context task
prompts are indexed in [agents/README.md](agents/README.md). Each task is owned
by a different agent and leaves a filesystem handoff for its successor; there
is no lead agent or retained conversational context.

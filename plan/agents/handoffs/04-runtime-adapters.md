# Task 04 handoff — safe runtime adapters

## Status

**DONE.** Descriptor/lifecycle, audio-buffer, raw-input, touch, tempo, target
checks, no-allocation checks, ABI regression, and pass-through regression all
pass. Miri was attempted but is unavailable for the pinned stable toolchain.

## Summary and design decisions

- Added the public `nts3::prelude` runtime API: `Nts3Plugin`, `InitContext`,
  `InitError`, `StereoBuffer`, `StereoInput`, `TouchPhase`, and `TouchEvent`.
  Only `type Parameters` and `process` are required plugin items; all planned
  lifecycle, touch, and tempo methods have defaults.
- Added a temporary sealed `Nts3Parameters: Default` placeholder. Test/fixture
  parameter types are private; Task 05 should expand this contract and replace
  the stubs rather than changing the plugin trait shape.
- Added `Runtime<P>` ownership of parameters, plugin, and a copied context.
  Construction order is parameter defaults, plugin default, plugin initialize;
  tests assert this order. `RuntimeController<P>` wraps the Task 03 state
  machine and provides callback-independent init/render/lifecycle dispatch for
  Task 07's generated ABI shims.
- Descriptor validation copies the packed descriptor unaligned and rejects null
  descriptor/context, wrong exact genericfx target, incompatible API, non-48-kHz
  rate, zero frame maximum, non-stereo geometry, and missing SDRAM hooks with the
  corresponding SDK code before state construction. Allocator and every plugin
  `InitError` are mapped and failure cleanup permits reinitialization.
- `StereoBuffer` stores raw pointers internally and never creates overlapping
  Rust slices/references. Its iterator copies both input samples before exposing
  a unique output frame, supporting separate and exact in-place buffers.
  Debug/test construction rejects partial overlap. Tests cover 0, 1, odd,
  `u16::MAX`, and randomized frame counts with untouched guard regions.
- Optional raw input is exposed as a copy-only `StereoInput`. The SDK hook is
  called on every valid render (including zero-frame renders), and its pointer
  is scoped to that render's `StereoBuffer` lifetime.
- Touch phase remains independent from coordinates. Raw, clamped, and normalized
  positions support non-1024, zero, one, and out-of-range dimensions. Began,
  moved, and stationary are active; ended and cancelled are inactive.
- UQ16.16 tempo conversion and all lifecycle/touch/tempo hooks are allocation
  free. A counting-global-allocator integration test covers render, reset,
  suspend, resume, touch, tempo, tick, and teardown after initialization.
- Added `fixtures/touch-probe`, an allocation-free DSP fixture whose left gain
  distinctly encodes all five phases and whose right gain follows normalized X
  only while active. Thus began `(0,0)` and ended `(0,0)` are observable as
  different states. Task 07 will add generated exports/header.
- Shared allocator-state tests now use one crate-wide test mutex, avoiding
  races between Task 03 and Task 04 lifecycle tests.

## Files added or changed

Added:

- `crates/nts3/src/{buffer,plugin,runtime,touch}.rs`
- `crates/nts3/tests/runtime_no_alloc.rs`
- `fixtures/touch-probe/{Cargo.toml,README.md,src/lib.rs}`
- this handoff

Changed:

- `Cargo.toml`, `Cargo.lock`
- `crates/nts3/{README.md,src/lib.rs}`
- `crates/nts3/src/{allocator,runtime_state}.rs`
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` was edited. Pre-existing untracked SDK,
reference, and NIH example files were not modified.

## Validation commands and results

All native Cargo commands were run through PowerShell. All Docker commands were
run through Bash.

1. Prerequisite smoke checks before implementation — **PASS**:

   ```powershell
   cargo test -p nts3 --all-targets
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo run -p nts3 --example allocating-probe
   ```

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

2. Final host tests, formatting, host/target lint and target checks — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p nts3 --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p touch-probe --target thumbv7em-none-eabihf -- -D warnings
   cargo tree -p nts3 --target thumbv7em-none-eabihf
   ```

   Runtime results: 23 unit tests, one allocation fixture, and one explicit
   post-init no-allocation integration test passed. Touch probe: 2 tests passed.
   Target dependency tree remains only `nts3 -> nts3-sys`.

3. Final ABI and ELF regressions — **PASS**:

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   git diff --check
   ```

   C/Rust ABI reports matched all 149 records and all 376 header bytes. The
   pass-through built twice reproducibly and remained byte-identical.

4. Miri — **UNAVAILABLE, not failed**:

   ```powershell
   cargo miri test -p nts3 --lib buffer::tests
   ```

   Rust reported that `cargo-miri` is not available for pinned stable
   `1.85.1-x86_64-pc-windows-msvc`. No nightly toolchain was installed or used.

## Artifacts, hashes, and observations

- `target/nts3/pass-through/pass_through.nts3unit`
  - SHA-256:
    `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`
  - 3,648 bytes, unchanged from the hardware-tested Task 01 artifact.
- ABI layout remains unchanged: 149 C/Rust records and a 376-byte genericfx
  header compare exactly.
- Runtime rendering supports the descriptor's full `u16` maximum (65,535
  frames / 131,070 stereo samples) in host tests without touching guards.
- Allocation count remains unchanged across every post-init callback tested.
- Target code remains `no_std`; no host/proc-macro/ELF dependencies were added.
- Stack and real-time CPU remain unknown. No generated touch-probe unit exists
  yet, so its hardware behavior is explicitly unverified.

## Unresolved risks / next-task notes

- `Nts3Parameters` is deliberately only the temporary sealed `Default`
  placeholder. Task 05 must add descriptors, set/get, smoother initialization,
  and begin/end block hooks, then integrate those hooks around `process`.
- `RuntimeController` is public only as a documented implementation detail for
  generated exports. Task 07 must place it behind the one SDK-global singleton
  and uphold the existing serialized/no-lock callback assumption.
- `get_raw_input` is optional; a null hook/result yields `None`. SDRAM alloc,
  free, and available hooks are mandatory because every declared runtime arena
  is allocated as one block.
- Partial-overlap detection is debug/test-only, as planned; release safety relies
  on the SDK's documented exact-overlap-or-disjoint contract at the unsafe ABI
  boundary.
- Miri coverage remains pending toolchain availability.

## Next task prerequisite smoke commands

From PowerShell:

```powershell
cargo test --workspace
cargo check -p nts3 --target thumbv7em-none-eabihf
cargo check -p touch-probe --target thumbv7em-none-eabihf
```

From Bash:

```bash
./container/run.sh ./tests/abi/compare.sh
./container/run.sh ./fixtures/pass-through/verify.sh
```

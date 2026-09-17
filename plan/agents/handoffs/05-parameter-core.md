# Task 05 handoff — parameter and smoothing core

## Status

**DONE.** Parameter primitives, Baseplug-compatible smoothing, sealed metadata
contract, runtime dispatch, host golden tests, target checks, lint, ABI probe,
and pass-through regression all pass.

## Summary and design decisions

- Added compact integer-backed `Parameter` state with raw/plain/normalized
  access, clamp-before-narrowing updates, and checked no-change-on-error updates.
  Normalization covers negative, bipolar, inverted, and degenerate endpoint
  ranges.
- Added `SmoothedParameter` over an f32 specialization of wrl/baseplug's pinned
  one-pole `Smooth` algorithm. It retains `Inactive`/`Active`/`Deactivating`,
  reset/destination behavior, strict `< 1e-5` settlement, and the original
  coefficient/recurrence formulas.
- The adaptation uses `libm::expf`, stores only current/first-block output rather
  than `[T; MAX_BLOCKSIZE]`, and advances through `next_plain` or
  `next_normalized`. Nonpositive/nonfinite time or sample rate is immediate.
  Milliseconds are documented as an exponential time constant, not a ramp
  duration.
- A host-only fixed-block reference closely follows the pinned Baseplug source.
  Golden tests compare coefficients, known vectors, output/status sequences,
  irregular partitions through settlement, multiple rates, reset, and
  retargeting. Exact-threshold and zero-frame behavior have focused tests.
- Expanded the hidden `Nts3Parameters` contract with descriptor/mapping/count
  constants; safe get/set; smoother initialize/reset and begin/end block hooks;
  and optional static C-string lookup. Defaults remain zero-parameter so Task 04
  fixtures continue to compile until Task 06 derives implementations.
- Integrated parameter setup/ownership and callbacks into `Runtime<P>`.
  Rendering brackets DSP with parameter begin/end hooks. SDK reset retains raw
  targets but snaps in-flight smoothers to those targets. Unknown IDs return
  zero/no-op/null and never index metadata arrays.
- Added a manual two-parameter TIME/FEEDBACK implementation in runtime tests.
  Its defaults are constructed directly from the same descriptor constants used
  for metadata, preventing header/runtime drift; it covers Smooth Echo's plain
  and smoothed access patterns before proc macros exist.
- Reviewed Baseplug `parameter.rs` gradient translation and `declick.rs` at the
  pinned commit. Neither was copied: NTS-3 already applies declared mapping
  curves in hardware, while runtime normalized access is intentionally linear;
  `Declick<T>` stages arbitrary object swaps and adds state unrelated to the
  required integer parameter smoothing.
- Kept all runtime/parameter fields plain mutable values. The official SDK C++
  template uses ordinary cached parameter fields and gives no concrete evidence
  requiring atomics or locks.
- Added source and project attribution/modification notices plus the complete
  selected Baseplug MIT license.

## Files added or changed

Added:

- `crates/nts3/src/parameter.rs`
- `THIRD_PARTY_NOTICES.md`
- this handoff

Changed:

- `Cargo.toml` lock data (`libm` 0.2.16 resolved)
- `crates/nts3/Cargo.toml`
- `crates/nts3/README.md`
- `crates/nts3/src/lib.rs`
- `crates/nts3/src/runtime.rs`
- `crates/nts3/tests/runtime_no_alloc.rs`
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` was edited. Existing untracked SDK,
reference, and NIH example trees were not modified.

## Validation commands and results

All native Cargo commands were run through PowerShell. All Docker commands were
run through Bash.

1. Prerequisite smoke checks before implementation — **PASS**:

   ```powershell
   cargo test --workspace
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   ```

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

2. Final host tests and target checks — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   ```

   `nts3`: 32 unit tests, one allocation fixture, and one post-init
   no-allocation integration test passed. Other workspace tests also passed.

3. Host and target lint/dependency audit — **PASS**:

   ```powershell
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p nts3 --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p touch-probe --target thumbv7em-none-eabihf -- -D warnings
   cargo tree -p nts3 --target thumbv7em-none-eabihf
   ```

   Target dependencies are exactly `nts3-sys` and no-std `libm`; no `std`,
   threads, locks, JSON, proc-macro, or host tooling entered the target graph.

4. Final ABI and ELF regressions — **PASS**:

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   git diff --check
   ```

   C/Rust ABI reports still match all 149 records and 376 header bytes. The
   pass-through built twice reproducibly and remained byte-identical.

## Artifacts, hashes, and memory observations

- `target/nts3/pass-through/pass_through.nts3unit`
  - SHA-256:
    `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`
  - 3,648 bytes and unchanged from the hardware-tested Task 01 artifact.
- Host `size_of` values asserted by tests:
  - `Parameter`: 6 bytes
  - adapted `Smooth`: 24 bytes
  - `SmoothedParameter`: 36 bytes
  - manual TIME/FEEDBACK parameter set: 44 bytes
  - zero-sized manual test plugin: 0 bytes
  - `Runtime<ManualPlugin>` on the 64-bit host: 80 bytes
- The equivalent scalar parameter sizes are pointer-independent. The runtime
  test records an expected 64-byte 32-bit layout branch for the manual fixture;
  the concrete Smooth Echo runtime is intentionally measured by Task 09 after
  its derive/export integration exists.
- Runtime parameter, render, lifecycle, tempo, touch, and string fallback paths
  perform no post-init allocation in the counting-allocator test.

## Unresolved risks / next-task notes

- Task 06 should generate `Nts3Parameters` implementations matching the manual
  test: one metadata source for `Default`, eight descriptors/mappings, dispatch,
  and per-smoothed-field hooks. Hidden public `SmoothedParameter::begin_block`
  and `end_block` exist specifically because proc-macro expansions occur in
  downstream crates.
- `string_value` returns `Option<&'static CStr>`; generated custom-string tables
  can implement it without allocation. Runtime converts unknown/unsupported
  values to null.
- No new target unit containing `libm`/the parameter runtime is exportable until
  Task 07, so hardware behavior for this new code remains unverified. The
  unchanged pass-through retains its prior hardware result.
- Callback concurrency remains undocumented by Korg. No synchronization was
  added without evidence, consistent with the SDK template and plan.
- Stack and real-time CPU use remain unknown.

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

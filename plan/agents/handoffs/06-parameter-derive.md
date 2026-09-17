# Task 06 handoff — `Nts3Parameters` derive

## Status

**DONE.** The host-only derive, facade/prelude reexport, generated metadata and
dispatch, target fixture, byte tests, compile diagnostics, documentation, lint,
ABI regression, and pass-through regression pass.

## Summary and design decisions

- Added host-only `nts3-macros` using `syn`/`quote`. `nts3` reexports the derive
  from both its root and prelude while retaining the sealed trait with the same
  name in Rust's separate macro/type namespaces.
- `#[derive(Nts3Parameters)]` accepts named structs containing `Parameter` and
  `SmoothedParameter` fields. It generates `Default`, the sealed trait impl,
  exactly eight descriptors and mappings, count, static get/set matches, and
  smoother initialization/reset/block hooks. No generated path allocates, uses
  trait objects, or performs dynamic dispatch.
- One parsed metadata model drives descriptors, mappings, defaults, indices,
  and smoothing. Missing slots use `UNUSED_PARAM`/`UNUSED_MAPPING` from
  `nts3-sys` through hidden facade reexports.
- Declaration order is the default index. Optional explicit indices may reorder
  fields but must be unique and contiguous from zero. Preset compatibility and
  reorder implications are documented in `docs/parameters.md`.
- Validation covers the eight-parameter limit, required/duplicate/unknown
  options, named struct/field types, indices, `i16` conversion, descriptor
  order/default/center, mapping bounds (including valid inverted mappings), SDK
  names/characters, display names, decimal/fixed fraction modes, assignments,
  curves/polarity, and smoothing/type combinations.
- SDK custom-string parameters are deliberately rejected until an explicit
  static string-table API exists. This follows Phase 4's permitted deferral and
  avoids advertising a strings descriptor whose callback always returns null.
- Added 24 stable compile-fail snapshots plus one pass case. Diagnostics point
  to the invalid value and explain the required range/choice.
- Added exact generated-byte tests for Smooth Echo TIME/FEEDBACK descriptors and
  mappings, including all six canonical padding slots. Runtime tests cover
  defaults, clamping, smoothing, explicit reorder, unknown IDs, and zero
  allocations through generated valid/invalid dispatch.
- Replaced Touch Probe's manual empty sealed parameter implementation with the
  derive. Retained Task 05's manual low-level runtime parameter implementation
  as an independent reference test.
- Added a `no_std` `parameter-derive` fixture matching Smooth Echo's parameter
  declaration so the exact metadata shape is checked for
  `thumbv7em-none-eabihf` before Task 07 provides the plugin export attribute.

## Files added or changed

Added:

- `crates/nts3-macros/{Cargo.toml,README.md,src/lib.rs}`
- `crates/nts3/tests/{derive_generated.rs,derive_ui.rs}`
- `crates/nts3/tests/ui/` pass/fail cases and `.stderr` snapshots
- `fixtures/parameter-derive/{Cargo.toml,src/lib.rs}`
- `docs/parameters.md`
- this handoff

Changed:

- `Cargo.toml`, `Cargo.lock`
- `crates/nts3/{Cargo.toml,README.md,src/lib.rs}`
- `crates/nts3/tests/runtime_no_alloc.rs`
- `fixtures/touch-probe/src/lib.rs`
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` or either Smooth Echo source file was
edited.

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

2. Final host tests and derive diagnostics — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo test -p nts3 --test derive_generated --test derive_ui
   cargo test -p nts3 --test runtime_no_alloc
   ```

   The workspace includes 32 `nts3` unit tests, allocation/runtime integration
   tests, three generated-metadata tests, 24 compile-fail snapshots, one
   trybuild pass case, and all prior crate/fixture tests.

3. Target checks and lint — **PASS**:

   ```powershell
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p parameter-derive --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p nts3 --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p parameter-derive --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p touch-probe --target thumbv7em-none-eabihf -- -D warnings
   ```

4. Target parser-dependency audit — **PASS**:

   ```powershell
   cargo build -p parameter-derive --target thumbv7em-none-eabihf --release
   cargo tree -p parameter-derive --target thumbv7em-none-eabihf `
     -e normal,no-proc-macro
   ```

   The runtime target tree is only `nts3 -> {libm, nts3-sys}`. A binary scan of
   the target fixture rlib found no `syn`, `quote`, or `proc_macro2` marker.
   Proc-macro dependencies run on the build host only.

5. Final Docker ABI/ELF regressions — **PASS**:

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   git diff --check
   ```

   ABI reports still match all 149 records and 376 header bytes. Pass-through
   remains reproducible and byte-identical.

A first attempt to run final Cargo and Docker checks concurrently was discarded
because the Docker verifier intentionally runs `cargo clean` on the shared
`target/` directory. The exact Cargo suite was rerun after Docker completed and
passed; future agents should keep these two validations sequential.

## Artifacts, hashes, and observations

- `target/nts3/pass-through/pass_through.nts3unit`
  - SHA-256:
    `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`
  - 3,648 bytes, unchanged from the hardware-tested Task 01 artifact.
- `target/thumbv7em-none-eabihf/release/deps/libparameter_derive-e0bb6835d8cb3fc5.rlib`
  - SHA-256:
    `8c3b68f68569573db0444784619dce5a3d72d2c4456f20d13814c71870ed94db`
  - generated target code contains no parser dependency markers.
- Generated SDK metadata remains exactly 256 descriptor bytes plus 64 mapping
  bytes. TIME is milliseconds, 1..2000, default 500, X exponential;
  FEEDBACK is decimal percent, 0..1000, default 0, Y linear; remaining bytes are
  canonical SDK none entries.
- Generated valid/invalid parameter callbacks and all existing post-init runtime
  paths allocate zero bytes in the counting-allocator test.

## Unresolved risks / next-task notes

- Task 07 still owns `#[nts3::plugin]`. Therefore the complete normative Smooth
  Echo file cannot type-check yet; the unchanged parameter declaration itself is
  host-tested byte-for-byte and target-checked through
  `fixtures/parameter-derive`. Do not add a no-op export attribute: Task 07 must
  generate the real header and ABI exports.
- A direct standalone Smooth Echo check also exposed a dependency/toolchain
  issue for Task 07/10: current crates.io `wide 1.1.1` and `safe_arch 1.0.0`,
  selected by `fundsp 0.21.0`, declare Rust 1.89 while this workspace pins
  1.85.1. `--ignore-rust-version` compiled those dependencies but then stopped,
  as expected, on the not-yet-implemented `nts3::plugin`. Resolve the MSRV pin
  reproducibly rather than relying on `--ignore-rust-version` in release builds.
- Custom strings remain explicitly deferred. The sealed trait's static
  `string_value` hook remains available for a future allocation-free table API.
- Target hardware behavior is unchanged/unverified for parameter code until
  Task 07 emits a loadable unit. Stack and real-time CPU remain unknown.

## Next task prerequisite smoke commands

From PowerShell:

```powershell
cargo test --workspace
cargo check -p parameter-derive --target thumbv7em-none-eabihf
cargo check -p touch-probe --target thumbv7em-none-eabihf
cargo tree -p parameter-derive --target thumbv7em-none-eabihf -e normal,no-proc-macro
```

From Bash, run sequentially after Cargo commands:

```bash
./container/run.sh ./tests/abi/compare.sh
./container/run.sh ./fixtures/pass-through/verify.sh
```

# Task 09 handoff — ELF validation and truthful memory accounting

## Status

**DONE.** The public inspector, mandatory post-build validation, versioned
reports, native concrete-plugin probe, malformed ELF fixtures, conservative
limits, C baseline policy, and all native/container regressions pass.

## Summary and design decisions

- Added `./nts3.sh inspect <artifact>` to the existing user-selected public Bash
  launcher. The internal `nts3-cli` engine performs inspection and never starts
  Docker. Build now runs the exact same inspector as its mandatory final gate.
- Selected maintained `object 0.37.3` with only ELF/read features. `object`,
  `serde`, hashing, and CLI dependencies remain tooling-only and do not enter the
  Thumb target graph.
- Added JSON memory schema v1 and stable human output. Reports include artifact
  SHA-256, all `PT_LOAD` file/memory ranges, union totals, every allocated and
  writable section, writable-load RAM, target/API/header/resources, relocation
  counts, provenance versions/commits/hashes, and explicit unknown stack/CPU.
- Named strict policy `nts3-framework-v1` validates ELF32 little-endian ARM
  `ET_DYN`, System V/EABI5/hard-float, NTS-3 genericfx/API 2.0, exact generated
  header/resource records, exports, unique Thumb callbacks, segment/section
  alignment, no undefined symbols/TLS/PLT/debug/unsupported unwind, and only
  symbol-free `R_ARM_RELATIVE` relocations.
- Named comparison policy `korg-sdk-legacy-v1` allows the official C dummy's
  known loader-valid duplicate fallback header, local C++ relocations, and PLT.
  It still validates ABI/exports/loads/limits and warns that resource/probe data
  are unavailable. Framework builds cannot select this exception because their
  resource record selects the strict policy.
- The conservative limits are independently 32,768 bytes for stripped artifact,
  unioned `PT_LOAD` memory, and unioned writable `PT_LOAD` memory. This includes
  ELF headers/alignment gaps rather than relying on GNU `size` section sums.
- Added a dedicated native probe entry generated for each concrete plugin. A
  separate process installs the framework allocator, allocates the backing arena
  directly through `System`, constructs the exact plugin/parameters/runtime,
  seals allocation, and executes 256 render/lifecycle/parameter/touch/tempo
  iterations. Unrelated setup/report allocations occur only while the arena is
  inactive. Any allocation after sealing aborts the probe/build.
- Dynamic-allocation plugins require measured margin `max(16 KiB, 2% of the
  declaration)`. Allocation-free 1-byte fixtures are allowed because their
  measured high-water is zero.
- Standalone inspection reuses a prior matching-hash native probe record instead
  of erasing measured data; a fresh arbitrary artifact truthfully reports the
  probe as not run.
- Synthetic ELF integration tests mutate class/data/OSABI/type/machine/EABI/
  float ABI, target/API/resource schema/SDRAM, header/export/callbacks, PLT,
  debug/unwind/relocation data, alignment, and all three size thresholds, and
  assert specific diagnostics.

## Files added or changed

Added:

- `crates/nts3-cli/src/inspector.rs`
- `docs/memory.md`
- this handoff

Changed:

- `Cargo.lock` (`object 0.37.3`, `sha2 0.10.9` and dependencies)
- `crates/nts3-cli/{Cargo.toml,README.md,src/main.rs,tests/fake_tools.rs}`
- `crates/nts3-macros/src/lib.rs`
- `crates/nts3/src/{allocator.rs,lib.rs,runtime.rs}`
- `docs/building.md`
- `nts3.sh`
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` or either Smooth Echo source tree was
changed. Pre-existing unrelated workspace state (`plan/references/README.md`
deleted and untracked `external/` / NIH reference content) was not modified.

## Validation commands and results

Docker commands were run from Bash with `MSYS_NO_PATHCONV=1`; native Cargo
commands were run through PowerShell as requested.

1. Prerequisite native suite and target check — **PASS**:

   ```powershell
   cargo test --workspace
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   ```

2. Initial prerequisite Docker attempt — **BLOCKED transiently** because Docker
   Desktop was not running. After the user started Docker, every Docker command
   below passed.

3. Inspector/fake-tool/malformed-fixture tests in the container — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./container/run.sh cargo test --locked -p nts3-cli
   MSYS_NO_PATHCONV=1 ./container/run.sh cargo test --locked -p nts3-cli \
     --test fake_tools malformed_elf_invariants_have_specific_diagnostics
   ```

   Six unit/schema tests and three Unix integration tests passed. The malformed
   fixture test covers 22 targeted mutations, including all synthetic limits.

4. Mandatory real builds and standalone inspection — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p pass-through --release
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p touch-probe --release
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p smooth-echo-nts3-plug --release
   MSYS_NO_PATHCONV=1 ./nts3.sh inspect target/nts3/pass_through.nts3unit
   MSYS_NO_PATHCONV=1 ./nts3.sh inspect target/nts3/touch_probe.nts3unit
   MSYS_NO_PATHCONV=1 ./nts3.sh inspect target/nts3/smooth_echo.nts3unit
   MSYS_NO_PATHCONV=1 ./nts3.sh inspect \
     target/nts3/pass-through/c-dummy/dummy_genericfx.nts3unit
   ```

   All Rust units passed `nts3-framework-v1`; the official C dummy passed the
   documented legacy policy with warnings. Re-inspection preserved matching
   native measurements.

5. ABI, reproducibility, C baseline, and old ELF regression — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./container/run.sh ./tests/abi/compare.sh
   MSYS_NO_PATHCONV=1 ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

   ABI matched 149 records and all 376 bytes. Two package-clean pass-through
   builds remained byte-identical and all old symbol/relocation/size checks pass.

6. Complete container workspace suite — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./container/run.sh cargo test --locked --workspace
   ```

7. Final native formatting, tests, lint, and target checks — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p pass-through --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   ```

8. Launcher/script/source hygiene — **PASS**:

   ```bash
   bash -n nts3.sh container/run.sh fixtures/pass-through/build.sh \
     tests/abi/compare.sh tests/cli/launcher.sh
   ./tests/cli/launcher.sh
   git diff --check
   ```

## Artifacts, hashes, and measured memory

Generated artifacts and reports are gitignored under `target/nts3/`:

| Unit | SHA-256 | Artifact | Load union | Static RAM | Declared / measured SDRAM |
|---|---|---:|---:|---:|---:|
| pass-through | `d598283d80da55127e86b2182504de7f29635a3866352cd70ea08c5887ed21c2` | 4,860 | 2,734 | 212 | 1 / 0 |
| touch-probe | `9454b216f8bd8bfaf9353ff9cb569b609c6927c9150ed3d20dc3f36360180c9a` | 5,028 | 2,912 | 220 | 1 / 0 |
| Smooth Echo | `0f18f7a082959b5da20de7ef043ea5268ab24705177abd7d5713b67e1e1cd1d6` | 13,284 | 7,402 | 508 | 1,100,000 / 1,048,576 |

Smooth Echo native measurements:

- `size_of::<EchoPlug>()`: 176 bytes
- `size_of::<EchoParameters>()`: 44 bytes
- `size_of::<Runtime<EchoPlug>>()`: 256 bytes
- allocations: 2
- requested/high-water: 1,048,576 bytes (the two expected delay payloads)
- alignment padding: 0 bytes
- declaration margin: 51,424 bytes, above the 22,000-byte 2% policy
- 256 post-init stress renders plus lifecycle/events: 0 new allocations

The current reports are:

- `target/nts3/{pass_through,touch_probe,smooth_echo}.memory.{json,txt}`
- `target/nts3/{pass_through,touch_probe,smooth_echo}.nts3unit`

GNU `size -A` categories reconcile to the allocated section list. `PT_LOAD`
union is intentionally larger where ELF headers and alignment gaps are loader
memory but not named section bytes; `docs/memory.md` documents the distinction.

## Remaining risks / next-task notes

- Stack and NTS-3 real-time CPU are explicitly `unknown`; neither ELF inspection
  nor the native probe measures them.
- Touch Probe and Smooth Echo remain hardware-unverified. Pass-through retains
  the prior hardware result because its canonical hash is unchanged.
- The strict relocation policy is based on current hardware-proven framework
  output. The legacy C policy is named and warning-bearing rather than an ad hoc
  framework exception.
- Task 10 may consume the Smooth Echo report directly; its declared budget and
  DSP were not changed.

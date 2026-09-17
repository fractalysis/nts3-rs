# Task 07 handoff — plugin attribute and generated ABI exports

## Status

**DONE.** All implementation, host, target, compile-fail, ABI, final-link,
dead-strip, reproducibility, size, lint, and hardware checks pass. The user
reported that the generated pass-through artifact works perfectly on the NTS-3.

## Summary and design decisions

- Added and reexported `#[nts3::plugin(...)]` for concrete, non-generic
  `Nts3Plugin` impls. The checked-in Smooth Echo Rust source remains unchanged.
- The attribute validates and encodes the unit name, developer/unit IDs, SDRAM
  bytes, and Cargo package major/minor/patch. Each version component must fit
  Korg's 7-bit component range and is packed as `major << 16 | minor << 8 |
  patch`.
- Unit names use the SDK's exact 19-character 7-bit character set. Developer ID
  zero and all 16 upper/lower-case spellings of `KORG` are rejected. SDRAM must
  be in `1..=3 MiB`.
- A fixed root `macro_export` marker gives a direct compile error when a crate
  applies the plugin attribute more than once, including invocations in
  separate modules. Fixed ABI export names additionally prevent two plugin
  crates from silently coexisting in one final artifact.
- Generated Rust-2024-safe items include one 376-byte `unit_header` in
  `.unit_header`, all 12 required callbacks, one concrete process-global
  `ExportRuntime<P>`, and target-only allocator/panic installation. Host macro
  expansion installs neither a global allocator nor a panic handler.
- `ExportRuntime<P>` centralizes the SDK's serialized process-global callback
  assumption around `UnsafeCell<RuntimeController<P>>`; mutable access cannot
  escape a callback.
- Added packed 12-byte `.nts3_resources` schema v1:
  `magic = "N3RS"`, `u16 schema_version`, `u16 record_size`, and `u32
  sdram_bytes`. The generated symbol is `nts3_resources`.
- Both `unit_header` and `nts3_resources` are compiler-retained. The GNU final
  link explicitly roots those symbols and every callback before section GC,
  preserving only their reachable code from the static archive.
- Converted pass-through and Touch Probe to the public trait/derive/attribute.
  Their ordinary source contains no handwritten header, callback, panic, or
  allocator code.
- Added Smooth Echo as a workspace member. After the verified toolchain upgrade
  to Rust 1.98.1, FunDSP's public `wide` and `safe_arch` dependencies resolve
  normally from crates.io through `Cargo.lock`; no vendored package copies or
  `--ignore-rust-version` workaround remain.

## Files added or changed

Added:

- `crates/nts3/src/export.rs`
- `crates/nts3/tests/plugin_generated.rs`
- `crates/nts3/tests/plugin_ui.rs`
- `crates/nts3/tests/ui/plugin-{pass,fail-*}.rs` and `.stderr` snapshots
- this handoff

Changed:

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
- `container/{Dockerfile,README.md,run.sh}`
- `crates/nts3-macros/{README.md,src/lib.rs}`
- `crates/nts3/{README.md,src/lib.rs,src/runtime.rs}`
- `fixtures/pass-through/{Cargo.toml,README.md,build.sh,verify.sh,src/lib.rs}`
- `fixtures/touch-probe/{README.md,src/lib.rs}`
- `example/smooth-echo-nts3-plug/Cargo.toml` (workspace cleanup only)
- `tests/golden/{elf-comparison.txt,pass-through.map.txt,pass-through.objdump.txt,pass-through.readelf.txt}`
- `plan/agents/STATUS.md`

`example/smooth-echo-nts3-plug/src/lib.rs` was not changed; its SHA-256 is
`ebfeb5bbe6166a987c56f904d04191a86ab8e0a1453ab5c166a3b366e5b10aa0`.
No source under `external/logue-sdk` was edited.

## Commands and results

All native Cargo commands were run directly through PowerShell. Docker commands
were run through Bash.

1. Prerequisite checks before implementation — **PASS**:

   ```powershell
   cargo test --workspace
   cargo check -p parameter-derive --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   cargo tree -p parameter-derive --target thumbv7em-none-eabihf -e normal,no-proc-macro
   ```

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

2. Final host tests and compile diagnostics — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```

   Results include 33 `nts3` unit tests, generated runtime/header/resource
   integration coverage, all previous allocation/runtime tests, 24 parameter
   derive compile failures, nine plugin attribute compile failures, and one
   plugin pass case. Tests prove exact callback signatures, metadata/runtime
   consistency, host allocation while the target arena would be sealed, all
   KORG case variants, version packing, duplicate-plugin rejection, and the
   resource record's exact bytes.

3. Final target checks and lint — **PASS**:

   ```powershell
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo check -p pass-through --target thumbv7em-none-eabihf
   cargo check -p touch-probe --target thumbv7em-none-eabihf
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   cargo clippy -p nts3 --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p pass-through --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p touch-probe --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf -- -D warnings
   ```

4. Final Docker ABI and generated ELF regression — **PASS**:

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   bash -n container/run.sh fixtures/pass-through/build.sh \
     fixtures/pass-through/verify.sh tests/abi/compare.sh
   git diff --check
   ```

   ABI reports match all 149 records and all 376 header bytes. The generated
   pass-through built twice from a package-clean archive with identical hashes.
   The verifier checks one header, one resource section with exact schema bytes,
   all generated dynamic symbols, distinct Thumb callback addresses, no
   undefined dynamic symbols, no call relocations/PLT, and both 32 KiB limits.

5. Rust 1.98.1 migration and public dependency resolution — **PASS**:

   ```bash
   ./container/run.sh build-image
   ./container/run.sh /bin/bash -lc \
     'rustc --version --verbose; cargo --version; arm-none-eabi-gcc --version'
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

   ```powershell
   cargo test --workspace
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   cargo clippy --workspace --all-targets -- -D warnings
   ```

   The image reports Rust/Cargo 1.98.1 and LLVM 22.1.8. The container archive
   checksums are verified during image construction. `Cargo.lock` resolves
   `wide 1.1.1` and `safe_arch 1.0.0` from crates.io with registry checksums;
   the repository has no `vendor/` directory.

## Artifacts, hashes, and memory/ABI observations

Generated artifacts are gitignored under `target/nts3/pass-through/`:

- `pass_through.nts3unit` (current Rust 1.98.1 container build)
  - SHA-256: `d598283d80da55127e86b2182504de7f29635a3866352cd70ea08c5887ed21c2`
  - stripped file size: 4,860 bytes
- `pass_through.elf`, `pass_through.map`
- `pass_through.{readelf,objdump,nm,size,sha256}.txt`
- extracted/expected `nts3_resources` binaries

Measured stripped sections/loads:

- `.text`: 898 bytes
- `.unit_header`: 376 bytes, exactly one
- `.nts3_resources`: 12 bytes, exactly one, schema v1, declared SDRAM 1 byte
- `.bss`: 100 bytes
- total reported section bytes: 2,620
- writable PT_LOAD memory: 212 bytes
- required callback/header/resource symbols: all in `.dynsym`
- undefined dynamic symbols: none
- dynamic call relocations: none
- PLT: none
- every callback has a unique odd/Thumb address
- stack and real-time CPU: unknown

The current generated artifact remains over 27 KiB below the file limit and
retains the required ELF class, machine, ET_DYN/System V, EABI5 hard-float,
section, symbol, relocation, and callback-address invariants.

## Hardware result and subsequent toolchain experiment

The user loaded the generated pass-through artifact with SHA-256
`c56f4a4578287cc160814f555087c2a015c377374ad5a6d059a0d08b59e73570`
and reported that it works perfectly. This closes the Task 07 hardware gate.
The previously reported device firmware family is `1.1.x`.

A subsequent noncanonical experiment compiled the same source/archive with the
machine's current stable channel, Rust/Cargo 1.98.1 (LLVM 22.1.8), and performed
the final GNU link in the pinned Korg container. It passed all ELF, symbol,
unique-Thumb-callback, resource-byte, relocation, PLT, file-size, and writable
load checks:

- artifact: `target/nts3/pass-through-stable-1.98.1/pass_through.nts3unit`
- SHA-256: `49c9fb4525a35a995e77af0b1a0feebd81294ae1d8a0c3819e7c8e975596033a`
- stripped file: 4,860 bytes
- `.text`: 898 bytes
- writable PT_LOAD memory: 212 bytes

The user subsequently reported that this Rust 1.98.1 experiment also works
perfectly on hardware. Rust 1.98.1 is now the canonical exact pin in
`rust-toolchain.toml` and the derived container. The Linux host and Thumb target
archives are checksum-pinned in `container/Dockerfile`, the default image tag is
`nts3-rs-toolchain:rust-1.98.1`, and the image was rebuilt successfully. The
former `vendor/` metadata workaround was deleted; locked crates.io releases
`wide 1.1.1` and `safe_arch 1.0.0` now compile directly on the upgraded MSRV.
The current canonical hash differs after lockfile/source hygiene updates and is
ELF-validated but has not separately been loaded on hardware.

Touch Probe and Smooth Echo are target type-checked but are not packaged by this
task's pre-CLI build path. Their hardware status remains explicitly unverified.
Callback concurrency still follows the official SDK template's serialized
plain-mutable assumption. Stack and real-time CPU remain unmeasured.

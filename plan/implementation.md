# Phased implementation checklist

## Phase 0 — Freeze inputs and reproducibility

- [ ] Add a root Cargo workspace and commit lockfiles.
- [ ] Pin Rust 1.98.1 and install `thumbv7em-none-eabihf` in the derived image.
- [ ] Pin `xiashj/logue-sdk:latest` by the locally verified image digest, while
      allowing an explicit override. Record GNU compiler/linker versions.
- [ ] Pin SDK API 2.0 sources already under `external/logue-sdk` and FunDSP
      0.21.0.
- [ ] Build Korg `dummy-genericfx` through Bash as the environment smoke test.
- [ ] Invoke Docker only from the repository Bash launcher. On Windows, support
      WSL2 with Docker integration; do not support native Windows shells or Git
      Bash and do not add cross-environment path conversion.

## Phase 1 — Hard-gate Rust ELF spike

Build `fixtures/pass-through` with manual Rust definitions before writing proc
macros.

- [ ] `#![no_std]`, panic abort, one static instance, all callbacks, static unit
      header, no heap.
- [ ] Compile Rust to a PIC static archive using `thumbv7em-none-eabihf` and
      `-C target-cpu=cortex-m7 -C relocation-model=pic` with size optimization,
      one codegen unit, fat LTO where compatible, and section GC.
- [ ] Final-link with Korg's GNU ARM linker/compiler and
      `platform/nts-3_kaoss/ld/unit.ld`; do not invent a second linker script
      unless a documented incompatibility requires a narrowly maintained fork.
- [ ] Compare `readelf -hSWlrs`, `objdump`, `nm`, map and size output against the
      C dummy unit.
- [ ] Verify hard-float ABI, Thumb code, ELF type DYN, System V OSABI, API/header
      segment and exported callbacks.
- [ ] Avoid unresolved direct SDK calls: SDRAM/raw-input hooks are function
      pointers from runtime context. Inspect PLT code anyway; prior Rust logue
      experiments have encountered invalid ARM-mode PLTs.
- [ ] Load pass-through on hardware. This is a stop/go gate. If it freezes,
      resolve linker/PLT/ELF differences before higher-level work.

Expected output is a written `tests/golden/pass-through.readelf.txt` plus the
exact linker command in the build tool tests.

## Phase 2 — `nts3-sys`

- [ ] Implement raw structures/constants from `runtime.h`, `unit.h`, and
      `unit_genericfx.h`.
- [ ] Handle bitfields as explicit packed bytes with constructor/accessor
      methods rather than Rust bitfields.
- [ ] Add C layout probe printing `sizeof`, `_Alignof`, `offsetof`, enum/macro
      values and expected generic header size.
- [ ] Compare C output to Rust tests in the Docker image.
- [ ] Add compile-time size/alignment assertions and tests for serialized header
      bytes.
- [ ] Record SDK license notices for copied definitions.

## Phase 3 — `nts3` runtime core

### Runtime state

- [ ] Implement single-instance state machine and prevent use before successful
      init or after teardown.
- [ ] Map all plugin errors to Korg `k_unit_err_*` results.
- [ ] Make every invalid parameter ID and unknown touch phase non-panicking.
- [ ] Copy needed descriptor/context values; do not retain short-lived audio
      pointers between renders.

### SDRAM allocator

- [ ] Implement target-only aligned bump arena initialized from one
      `sdram_alloc` call.
- [ ] Track requested bytes, padding, current position, and high-water position.
- [ ] Support alignments required by Rust/FunDSP and test many randomized
      layouts on host.
- [ ] Seal after initialization and expose no public unseal operation.
- [ ] Free the original Korg allocation after plugin drop on teardown.
- [ ] Implement host tracking mode for the initialization memory probe.
- [ ] Test failed arena allocation and repeated init/teardown.

### Audio/context adapters

- [ ] Implement the audited `StereoBuffer` frame iterator.
- [ ] Add optional raw-input access refreshed each render call.
- [ ] Implement `InitContext` accessors for sample rate, maximum frames and touch
      dimensions.
- [ ] Convert UQ16.16 tempo without allocation.
- [ ] Implement typed touch phases and normalized positions.

## Phase 4 — Parameter system

- [ ] Implement integer-backed `Parameter` with bounds and normalized/plain
      accessors.
- [ ] Port/adapt wrl/baseplug's MIT-licensed `Smooth<f32>` algorithm from pinned
      commit `9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74`: one-pole `a/b`
      coefficients, status lifecycle, reset/target semantics and `1e-5` settle
      threshold. Replace its fixed block output array with allocation-free
      `next_plain`/`next_normalized` sample access suitable for variable NTS-3
      buffers; use generated begin/end-render hooks and retain the first value
      needed for Baseplug-compatible status updates. Use `libm::expf` under
      `no_std` and make nonpositive milliseconds immediate.
- [ ] Add Baseplug MIT attribution in source and third-party notices, plus
      golden equivalence tests against the original block algorithm for output,
      reset, retargeting and status behavior.
- [ ] Define a sealed `Nts3Parameters` trait used by generated code.
- [ ] Implement derive parsing and actionable compile errors using `trybuild`.
- [ ] Generate exactly eight descriptors and mappings, padding unused slots.
- [ ] Auto-initialize values and smoothers from the same metadata used for the
      header so runtime/header defaults cannot drift.
- [ ] Cover SDK display types and fraction encoding needed by Smooth Echo;
      defer custom strings until core output is valid, or implement them behind
      a small explicit enum/string-table API.
- [ ] Test clipping, inverted mappings, all curves/polarities, percent decimal
      format and time milliseconds.

## Phase 5 — Export macro

- [ ] Implement `#[nts3::plugin(...)]` on an `impl Nts3Plugin for Type`.
- [ ] Generate metadata, header and all ABI symbols. Do not require an extra
      export macro invocation.
- [ ] Convert Cargo package semver to Korg packed version automatically.
- [ ] Validate names, IDs, SDRAM ceiling and one-plugin-per-artifact rule.
- [ ] Emit a compact `.nts3_resources` section containing framework schema
      version and declared SDRAM arena bytes for artifact inspection.
- [ ] Confirm host tests compile without installing a target global allocator or
      panic handler.
- [ ] Add symbol-retention tests after archive and final link dead stripping.

At this point, make `example/smooth-echo-nts3-plug/src/lib.rs` type-check
unchanged. If the public API must change for soundness, update the architecture
document first; do not add boilerplate merely to simplify macro implementation.

## Phase 6 — Bash launcher, Rust tooling engine and container

Commands:

- [ ] `./nts3 doctor`: check Docker/image and mounts from Bash, then check the
      Rust target, Cargo, GNU tools and versions inside the container.
- [ ] `./nts3 check [-p package]`: target `cargo check` with correct features.
- [ ] `./nts3 build [-p package] --release`: build archive, final-link, strip,
      validate, package and report.
- [ ] `./nts3 inspect <artifact>`: validate an existing ELF/unit without
      rebuilding.
- [ ] `./nts3 new <name>`: generate a minimal plugin containing only a params
      struct, DSP struct and trait impl—never copied ABI boilerplate.

Implementation requirements:

- [ ] Make the repository-root `nts3` Bash launcher the only public build CLI;
      do not implement or document `cargo nts3`.
- [ ] Default to the pinned Docker image for reproducibility; provide an explicit
      expert local mode through the same launcher.
- [ ] Only the Bash launch layer invokes Docker; `nts3` may delegate to
      `container/run.sh`. The internal `crates/nts3-cli` Rust engine runs inside
      the container and never launches Docker.
- [ ] Support Bash on Linux/macOS and WSL2 on Windows. Do not add Git Bash,
      PowerShell, `cmd.exe`, `wsl.exe`, `cygpath` or MSYS path-conversion paths.
- [ ] The default flow requires only Bash and Docker on the calling system;
      Cargo, Rust and GNU tools execute inside the container.
- [ ] Forward Cargo diagnostics and return exact nonzero status.
- [ ] Use workspace/target caches without baking host-specific paths into ELF.
- [ ] Produce deterministic file names and a `--verbose` command transcript.
- [ ] Keep compiler/linker policy in tooling, not each plugin's Cargo.toml.
- [ ] Ensure plugin authors do not need `.cargo/config`, a linker script,
      `build.rs`, C source, or a crate-type stanza.

## Phase 7 — Artifact inspection and memory reporting

Use a maintained Rust ELF parser in the host CLI; it never enters target code.
Generate `<unit>.memory.txt` and `<unit>.memory.json` containing:

- [ ] Stripped artifact byte size and configured unit limit.
- [ ] Each `PT_LOAD` file/memory range and union totals (avoid double counting
      overlapping ranges).
- [ ] `.text`, `.rodata`, `.data`, `.bss`, `.got`, dynamic/relocation and header
      sizes.
- [ ] Static writable RAM requirement and 32 KiB limit.
- [ ] `size_of::<Plugin>`, `Parameters`, `Runtime<Plugin>` emitted by the native
      probe.
- [ ] Declared target SDRAM arena from `.nts3_resources`.
- [ ] Native initialization allocator high-water, margin and percent of 3 MiB.
- [ ] Stack usage as `unknown` unless measured; never report it as zero merely
      because the linker script reserves zero private stack.
- [ ] Toolchain/framework/SDK/FunDSP versions and source commit IDs.

Validation must also reject:

- Wrong class/data/machine/type/OSABI/EABI/float ABI.
- Missing/multiple `.unit_header`, malformed header or wrong target/API.
- Missing required dynamic symbols.
- Unexpected unresolved symbols, relocations or ARM-mode PLT stubs.
- Debug/unwind sections in final output when they threaten size or imply
  unsupported unwinding.
- Invalid segment alignment/size, code/load overflow, static RAM overflow, or
  SDRAM declaration >3 MiB.

Dynamic SDRAM cannot be inferred solely from machine code. The exact declared
arena can be read from the artifact; actual use comes from the native
initialization probe. Clearly label the distinction. On hardware, arena
reservation is exact and fails `unit_init` before plugin construction if Korg
cannot supply it.

## Phase 8 — Smooth Echo completion

- [ ] Keep `example/smooth-echo-nih-plug` unchanged as reference.
- [ ] Build the checked-in NTS-3 source with FunDSP 0.21/no default features.
- [ ] Verify the two Tap buffers consume the expected 1,048,576 bytes plus
      bounded alignment/transient overhead below 1,100,000 bytes.
- [ ] Ensure no allocation during reset, render, parameter, tempo or touch calls.
- [ ] Profile stripped ELF. If over 32 KiB, first use release/LTO/section-GC and
      remove formatting/panic baggage. If still too large, replace only the
      FunDSP nodes with small no_std equivalents behind the same plugin source
      shape; document the measured reason rather than weakening size checks.
- [ ] Add impulse, silence, bounded-output, parameter sweep, reset and in-place
      equivalence tests.
- [ ] Compare against reference behavior at 48 kHz within a documented floating
      tolerance; account for intentional FunDSP version differences.
- [ ] Confirm parameter defaults/mappings: X→time exponential, Y→feedback
      linear, 500 ms and 0% defaults.

## Phase 9 — hardening and documentation

- [ ] Run `cargo fmt`, clippy for host and target-supported crates, tests,
      `trybuild`, ABI probe and artifact fixtures in CI.
- [ ] Audit every unsafe block with a local safety comment.
- [ ] Run Miri on host-side buffer/state tests where supported.
- [ ] Document no-allocation-after-init and no-unwind rules.
- [ ] Audit copied/adapted Baseplug code and include its MIT license, pinned
      source URL/commit and modification notice in third-party documentation.
- [ ] Add quickstart, parameter/mapping table, memory guide and troubleshooting
      for ELF/size/allocator errors.
- [ ] Perform and record the hardware checklist from `validation.md`.

# Agent 01 task: reproducible toolchain and Rust ELF hardware spike

## Context

This project needs a minimal `no_std` Rust framework that emits Korg NTS-3
`genericfx` units. Before any framework abstraction, prove that Rust PIC objects
can be linked into the exact ELF format accepted by the device. The Korg target
is Cortex-M7 hard-float Thumb, API 2.0, ELF32 ET_DYN/System V, with approximately
32 KiB unit/load limits. The supplied Docker image builds the C++ dummy but has
no Rust. Prior third-party Rust logue work reports dangerous ARM/Thumb PLT
behavior, so readelf inspection and an actual load are blocking evidence.

## Prerequisites

None. Read `plan/agents/README.md` and set Task 01 status first.

## Required reading

- `plan/README.md`, `plan/architecture.md`, `plan/research-notes.md`
- `plan/implementation.md` Phase 0 and Phase 1
- `plan/validation.md` sections 3–4 and hardware pass-through checklist
- `external/logue-sdk/platform/nts-3_kaoss/README.md`
- SDK dummy `Makefile`, `config.mk`, `header.c`, `unit.cc`
- SDK `common/{runtime.h,unit.h,unit_genericfx.h,attributes.h,_unit_base.c}`
- SDK `ld/{unit.ld,rules.ld}` and `docker/run_cmd.sh`

## Task

1. Create the root Cargo workspace/toolchain files without adding the unfinished
   Smooth Echo fixture as a compiled member yet.
2. Add a derived, digest-pinned Docker environment containing a pinned Rust
   toolchain and `thumbv7em-none-eabihf`. Add a Bash launcher. Record actual GNU
   ARM versions instead of trusting README text.
3. Add `fixtures/pass-through` as a deliberately manual `no_std` Rust unit with
   audited static header and all required callbacks. It may duplicate ABI only
   for this spike; later tasks replace it.
4. Compile PIC Rust to a static archive and final-link with the Korg GNU tools
   and SDK linker script. Preserve the full manual command in a script/test.
5. Build the official C dummy and compare ELF headers, program headers,
   sections, dynamic symbols, relocations, map and disassembly.
6. Resolve System V OSABI, EABI5 hard float, Thumb PLT, symbol retention and
   stripping properly. Never apply an unexplained byte patch.
7. Produce `pass_through.nts3unit`, map, readelf/objdump reports and SHA-256.
8. Load it on an NTS-3 and verify stereo pass-through, selection, bypass,
   suspend/resume and unload/reload. If no device is available, mark the task
   `BLOCKED`; the user must explicitly waive hardware before Task 02 proceeds.

## Deliverables

- Root workspace and pinned toolchain configuration
- `container/{Dockerfile,run.sh,README.md}`
- `fixtures/pass-through/` and reproducible build script
- Normalized C/Rust ELF comparison under `tests/golden/`
- Handoff `plan/agents/handoffs/01-toolchain-elf-spike.md`

## Acceptance checks

- Korg C dummy builds through the supplied image using a Bash Docker command.
- A clean container builds the Rust unit twice with identical stripped hashes,
  or all nondeterminism is identified and removed.
- `readelf` proves ELF32 little-endian ARM ET_DYN, System V, correct EABI/hard
  float, expected load/header segments and required dynamic symbols.
- Undefined symbols/relocations and PLT disassembly are reviewed and recorded;
  no ARM-mode PLT can be reached from Thumb callbacks.
- Artifact and static load remain below documented limits.
- Real-device checks pass, or status is `BLOCKED` pending explicit user waiver.

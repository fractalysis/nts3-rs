# Agent 02 task: audited `nts3-sys` raw ABI

## Context

Agent 01 proved the low-level link route. Replace ad-hoc Rust ABI declarations
with a small reusable `no_std` crate matching Korg SDK API 2.0 exactly. This task
contains no safe runtime, parameters, proc macros, allocator, or CLI.

## Prerequisites

Task 01 must be `DONE` or user-`WAIVED`. Read its handoff and rerun its ELF smoke
build before changing code.

## Required reading

- `plan/agents/README.md`, `plan/architecture.md` section “Raw ABI crate”
- `plan/implementation.md` Phase 2; `plan/validation.md` section 3
- Task 01 handoff and generated ELF comparison
- SDK `common/{runtime.h,unit.h,unit_genericfx.h,attributes.h}`
- SDK `dummy-genericfx/header.c`

## Task

1. Create `crates/nts3-sys` with only raw constants, structures and callback
   pointer types needed by NTS-3 genericfx.
2. Use exact `repr(C)`/`repr(C, packed)` layout. Represent C bitfields as an
   explicit byte with safe constructors/accessors. Never create references to
   unaligned packed fields.
3. Include runtime descriptor/hooks, genericfx context, unit/header/parameter
   mappings, target/API/errors, display types, assignments, curves and all touch
   phases.
4. Add compile-time Rust layout assertions.
5. Add a C probe compiled against checked-in SDK headers that prints every
   shared size/alignment/offset and relevant constant. Compare it automatically
   with Rust results inside Docker.
6. Replace the pass-through fixture's manual definitions with `nts3-sys` while
   preserving its ELF and hardware behavior.
7. Preserve Korg BSD license attribution for copied definitions.

## Deliverables

- `crates/nts3-sys/`
- `tests/abi/` C probe and comparison harness
- Updated pass-through fixture
- Handoff `plan/agents/handoffs/02-raw-abi.md`

## Acceptance checks

- Host tests compare all C/Rust sizes, alignments, offsets and constants.
- Header serialization matches the C dummy byte-for-byte except declared unit
  metadata.
- `cargo check -p nts3-sys --target thumbv7em-none-eabihf` passes without `std`.
- Pass-through still passes Task 01 ELF inspection and produces no new symbols,
  relocations or meaningful size regression.
- Every unsafe operation has a local safety explanation.

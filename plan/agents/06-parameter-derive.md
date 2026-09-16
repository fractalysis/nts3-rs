# Agent 06 task: `Nts3Parameters` derive macro and diagnostics

## Context

Create the host-only procedural macro that turns the concise parameter fields in
the normative Smooth Echo example into the sealed parameter implementation,
default construction, eight descriptors and eight default mappings. Generated
target code must be static and small; `syn`/`quote` must never enter target
artifacts.

## Prerequisites

Tasks 01–05 complete/waived. Read Task 05 handoff and its sealed trait/tests.

## Required reading

- `plan/agents/README.md`
- `plan/architecture.md` parameter model and macro validation list
- `plan/implementation.md` Phase 4
- `plan/validation.md` macro compile tests
- `example/smooth-echo-nts3-plug/src/lib.rs` parameter declarations (normative)
- SDK parameter/header/mapping definitions and character restrictions

## Task

1. Create host-only `crates/nts3-macros` and reexport `Nts3Parameters` derive
   through `nts3`/its prelude.
2. Parse the exact `#[parameter(...)]` syntax used by Smooth Echo.
3. Generate `Default`, sealed parameter dispatch, constant eight-entry SDK
   descriptor/mapping arrays, count and smoother initialization from one metadata
   source. Pad unused slots automatically.
4. Default index to declaration order; support optional explicit `index` with
   duplicate/gap validation and document preset/reordering implications.
5. Validate count, i16 ranges, defaults, mapping ranges, names/characters,
   display/fraction fields, assignment, curve and smoothing combinations.
6. Add `trybuild` pass/fail cases with stable actionable diagnostics.
7. Replace manual parameter test sets where useful but retain low-level tests.

## Deliverables

- `crates/nts3-macros` derive implementation
- Macro reexport from facade/prelude
- `trybuild` suite and generated-byte tests
- Handoff `plan/agents/handoffs/06-parameter-derive.md`

## Acceptance checks

- Smooth Echo's parameter struct compiles unchanged on host and target.
- Generated descriptors/mappings exactly encode TIME and FEEDBACK and pad six
  slots with SDK none values.
- Generated default values and smoothing match source attributes.
- Every invalid case listed in `plan/validation.md` has a compile-fail test.
- Target dependency tree and artifact contain no proc-macro parser dependencies.
- Macro expansion adds no dynamic dispatch or heap allocation.

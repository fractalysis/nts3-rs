# Agent 07 task: plugin attribute, unit header and generated ABI exports

## Context

Remove all remaining per-plugin SDK boilerplate. The single
`#[nts3::plugin(...)]` attribute on an `impl Nts3Plugin` must generate the unit
header, concrete callbacks, target allocator/panic installation and compact
resource metadata. There must be no additional `nts3_export!` line, build.rs,
C header or handwritten callback in ordinary plugins.

## Prerequisites

Tasks 01–06 complete/waived. Read all handoffs, especially Task 01 link/symbol
requirements and Task 06 generated parameter constants.

## Required reading

- `plan/agents/README.md`
- `plan/architecture.md` normative API and generated adapter sections
- `plan/implementation.md` Phase 5
- `plan/validation.md` macro/ABI checks
- Entire `example/smooth-echo-nts3-plug/src/lib.rs`
- Task 01 pass-through callbacks/link reports and Task 02 raw header layout

## Task

1. Implement/reexport `#[nts3::plugin(...)]` on concrete trait impls.
2. Parse name, developer ID, unit ID and SDRAM bytes; derive packed version from
   Cargo package major/minor/patch.
3. Generate `unit_header` in `.unit_header`, all required concrete C ABI
   callbacks, runtime dispatch, target-only allocator and panic glue, and a
   versioned `.nts3_resources` record.
4. Validate SDK character/length rules, reserved IDs including KORG case
   variants, version fit, SDRAM <=3 MiB and one exported plugin per artifact.
5. Use Rust 2024 unsafe attributes correctly and preserve exports through static
   archive/final-link dead stripping without pulling unrelated code.
6. Convert manual pass-through and touch-probe fixtures to the public trait and
   macro. Keep the old manual spike only as a test reference if useful.
7. Add Smooth Echo as a workspace/API check member; it need not be packaged by a
   polished CLI yet, but host and target check must pass unchanged.

## Deliverables

- Plugin attribute and generated runtime glue
- Generated pass-through/touch-probe fixtures
- `.nts3_resources` schema and tests
- Smooth Echo host/target type-check integration
- Handoff `plan/agents/handoffs/07-plugin-export.md`

## Acceptance checks

- Normative Smooth Echo source remains unchanged and target-checks.
- Generated pass-through final-links with the exact Task 01 ELF shape and loads
  on hardware (or retained explicit hardware waiver).
- `nm/readelf` show one header and every required symbol after stripping.
- Invalid metadata has compile-fail diagnostics.
- Host tests do not install target global allocator/panic behavior.
- No ordinary fixture contains copied ABI callbacks/header/linker/panic code.

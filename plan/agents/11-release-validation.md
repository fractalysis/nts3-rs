# Agent 11 task: independent integration, documentation and release validation

## Context

This is an independent final verification task, not a lead role. Begin from zero
context, trust no prior claims without rerunning commands, and fix only concrete
integration/release defects. All implementation should already exist. Produce a
single release-readiness record or mark exact blockers.

## Prerequisites

Tasks 01–10 complete/waived. Read every handoff and note every waiver. A waiver
means “known unverified,” never “passed.”

## Required reading

- `plan/agents/README.md`
- All top-level `plan/*.md`, all prior task files and handoffs
- Public crate/container/tool docs and both examples
- CI configuration, reports and generated command transcripts
- SDK README and platform constraints

## Task

1. From a clean checkout/cache where practical, run formatting, host tests,
   clippy, trybuild, ABI C/Rust probe, target checks and all three container
   builds (pass-through, touch-probe, Smooth Echo).
2. Independently inspect final ELFs/reports and reconcile hashes, symbols,
   relocations, limits, SDRAM high-water and no-post-init-allocation claims.
3. Audit target dependency trees for `std`, host CLI/proc-macro/ELF parser
   leakage, threads/locks/JSON and unnecessary code.
4. Audit all unsafe blocks for safety comments and focused coverage. Run Miri on
   supported buffer/state tests.
5. Verify `cargo nts3 doctor/check/build/inspect/new` documentation and execute
   the quickstart exactly as a new plugin author would.
6. Ensure docs explain parameters, Baseplug-derived one-pole smoothing semantics,
   touch, memory categories, IDs, Docker Bash requirement, troubleshooting,
   stack/CPU unknowns and hardware status. Verify copied/adapted Baseplug code
   carries its pinned source, modification notice and MIT license attribution.
7. Execute the complete real-device checklist in `plan/validation.md`, including
   four simultaneous runtimes, or preserve explicit blocked/waived status.
8. Fix narrow integration/doc/test defects. Do not redesign APIs or relax
   policies; mark architectural failures blocked for a new user decision.
9. Produce the release acceptance report specified in validation.md.

## Deliverables

- Green CI/reproducible clean validation, or exact blocker report
- Completed docs under `docs/` and container README
- Final release acceptance report with versions, hashes and memory numbers
- Handoff `plan/agents/handoffs/11-release-validation.md`

## Acceptance checks

- Every command and expected artifact in `plan/README.md` Definition of Done is
  verified from scratch.
- Two clean release builds are reproducible.
- ABI/layout/malformed-ELF suites pass.
- Reports stay below all limits and distinguish known/measured/unknown values.
- Generated template and Smooth Echo require no hidden manual setup.
- Hardware evidence is attached or prominently marked unverified—not implied.
- `plan/agents/STATUS.md` accurately reflects every task and waiver.

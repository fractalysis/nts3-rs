# Agent 10 task: complete and optimize the Smooth Echo NTS-3 unit

## Context

The normative source was written before the framework. Make it build unchanged
unless a soundness flaw requires a documented architecture/API correction. It
uses FunDSP 0.21 without default features, two two-second `Tap<U1>` buffers, two
high-pass nodes, X-mapped smoothed milliseconds and Y-mapped feedback. The two
delay payloads are expected to use 1,048,576 SDRAM bytes inside a 1,100,000-byte
arena. Touch activity and XY remain separate but do not alter original DSP.

## Prerequisites

Tasks 01–09 complete/waived. Read every handoff, especially actual code/load and
memory reports. Run pass-through and touch-probe full builds first.

## Required reading

- `plan/agents/README.md`
- `plan/implementation.md` Phase 8
- `plan/validation.md` DSP, memory and Smooth Echo hardware sections
- Both Smooth Echo source trees and NTS-3 example README
- Task 09 report format and Task 03 allocator behavior

## Task

1. Build/package `example/smooth-echo-nts3-plug` through `./nts3` with
   FunDSP exactly 0.21.0/default-features=false.
2. Add host DSP tests for silence, impulse timing, parameter sweeps, feedback
   boundary/freeze, reset retention, finite/bounded long random runs and
   separate/in-place equivalence.
3. Ignore the NIH-plug version of the effect - it exists just to create the nts3-plug version.
4. Verify descriptor/default mapping bytes: 500 ms default, 0% feedback,
   exponential X time, linear Y feedback.
5. Verify actual two-delay allocation, alignment/transient overhead and margin
   below 1,100,000 bytes; verify zero allocations after initialization.
6. Optimize release code/load with LTO, section GC and removal of formatting/
   panic baggage if needed. Do not weaken checks or increase SDRAM silently.
7. If FunDSP cannot fit 32 KiB after evidence-based optimization, mark blocked
   and present measured alternatives. Replace nodes only after user approval and
   update the plan/example transparently.
8. Build final unit, map, reports and hashes; perform Smooth Echo hardware tests
   if available.

## Deliverables

- Passing Smooth Echo DSP/integration tests
- Final `.nts3unit`, map, memory reports and command transcript
- Reference comparison data/provenance
- Handoff `plan/agents/handoffs/10-smooth-echo.md`

## Acceptance checks

- Checked-in public source has no per-plugin framework boilerplate and target
  checks/builds.
- All DSP tests pass without NaN/Inf or post-init allocation.
- Artifact/load/static RAM limits pass with recorded margins.
- Declared and measured SDRAM pass, explaining every material allocation.
- Touch origin active/released behavior remains covered by tests.
- Hardware checklist passes, or artifact is explicitly marked hardware
  unverified under an earlier user waiver.

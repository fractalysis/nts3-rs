# Agent 05 task: target parameter and smoothing core

## Context

Implement the non-macro parameter engine used by generated code. NTS-3 exposes
at most eight signed-16-bit-context integer parameters; callbacks use i32 for
future compatibility. The framework owns parameters and passes `&mut Parameters`
to DSP, eliminating Arc/atomics/trait objects. Smoothing must be a credited
`no_std` adaptation of wrl/baseplug's MIT-licensed one-pole smoother, not a newly
designed linear ramp. Metadata generation belongs to the next task.

## Prerequisites

Tasks 01–04 complete/waived. Read handoffs and run workspace tests.

## Required reading

- `plan/agents/README.md`
- `plan/architecture.md` “Parameter model”
- `plan/implementation.md` Phase 4
- `plan/validation.md` parameter tests
- SDK README Parameter Descriptors, Types, Default Mappings and Curves
- Parameter declarations and use in `example/smooth-echo-nts3-plug/src/lib.rs`
- Local pinned copies `plan/references/baseplug-smooth.rs`,
  `plan/references/baseplug-LICENSE-MIT`, and provenance README; compare with
  Baseplug's permanent upstream source if network access is available.
- Baseplug `src/parameter.rs` gradient translation and `src/declick.rs`; reuse
  only portions that simplify this target API without importing desktop/std
  framework machinery.

## Task

1. Implement compact `Parameter` storing raw target state with `raw`, `plain`
   and normalized f32 accessors and checked/clamped updates.
2. Adapt Baseplug `Smooth<f32>` directly: preserve `SmoothStatus`, set/reset/
   destination semantics, `SETTLE = 1e-5`, and the one-pole coefficient/update
   formulas. Replace its `[T; MAX_BLOCKSIZE]` with compact per-sample
   `next_plain`/`next_normalized`, retaining the current render block's first
   output so private `begin_block/end_block` hooks can reproduce Baseplug's
   `update_status` timing. Use `libm::expf` for `no_std`; define nonpositive
   milliseconds as immediate. Keep copied-code attribution.
3. Add a host-only copy/reference implementation of the original algorithm and
   golden tests proving sequence equivalence under varied block partitioning,
   sample rates, resets and retargeting. Document that milliseconds are an
   exponential time constant rather than a linear completion duration.
4. Review Baseplug's gradient translation and declick code. Copy/adapt only if it
   removes complexity or improves correctness for current NTS-3 requirements;
   otherwise record why it was unnecessary. Preserve MIT notices for anything
   reused.
5. Define the sealed internal `Nts3Parameters` contract required by runtime:
   const descriptors/mappings/count, defaults, set/get, smoother setup/reset and
   optional string lookup.
6. Integrate parameter ownership and callback dispatch into `Runtime<P>`,
   replacing Task 04's stub. Invalid IDs must never index arrays or panic.
7. Implement manual test parameter sets covering all behavior needed by Smooth
   Echo before introducing proc macros.
8. Keep mutable plain fields unless concrete SDK/hardware evidence requires
   synchronization; record any such evidence rather than speculating.

## Deliverables

- Parameter primitives and sealed contract in `crates/nts3`
- Runtime parameter callback integration
- Comprehensive host tests and target check
- Baseplug MIT license/source attribution and modification notice in source and
  project third-party notices
- Handoff `plan/agents/handoffs/05-parameter-core.md`

## Acceptance checks

- Bounds/normalization cover negative, bipolar, inverted and degenerate ranges.
- Smoothing coefficients/output/status match pinned Baseplug golden vectors;
  settlement snaps exactly at the `1e-5` threshold, while nonpositive time is
  immediate and sample-rate changes/reset/retarget remain continuous.
- Header/default values and runtime values can be sourced from one manual
  metadata definition without drift.
- Unknown IDs safely return/no-op/null according to callback.
- No post-init allocations and no target `std` symbols.
- Runtime/plugin/parameter `size_of` values are measured and recorded.

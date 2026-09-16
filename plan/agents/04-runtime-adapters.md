# Agent 04 task: safe audio, context, touch, tempo and lifecycle adapters

## Context

Complete `Runtime<P>` around the allocator/state foundation. Korg supplies
interleaved stereo `f32` buffers that overlap completely or not at all, touch
phases independent of coordinates, UQ16.16 tempo, and a genericfx raw-input hook
whose pointer is valid only for one render call. The plugin-facing API must be
small and allocation-free.

## Prerequisites

Tasks 01–03 complete/waived. Read all handoffs; rerun ABI and allocator tests.

## Required reading

- `plan/agents/README.md`
- `plan/architecture.md` trait, touch, buffer and runtime sections
- `plan/implementation.md` Phase 3 audio/context items
- `plan/validation.md` touch, buffer and lifecycle sections
- SDK README Unit API, Runtime Descriptor/Context, raw input and touch sections
- SDK dummy `unit.cc`, `effect.h`, and `common/processor.h`
- Normative example method signatures in `example/smooth-echo-nts3-plug/src/lib.rs`

## Task

1. Define the `Nts3Plugin` trait with required associated Parameters placeholder
   and `process`, plus default lifecycle/touch/tempo hooks exactly as planned.
2. Implement `Runtime<P>` ownership and validated initialization order around
   plugin/parameter placeholders. Do not implement the parameter system yet;
   use a private test stub satisfying the temporary sealed interface.
3. Implement `InitContext` with sample rate, maximum frames and touch area.
4. Implement an audited `StereoBuffer`/frame iterator supporting separate and
   exact in-place buffers without aliasing references. Detect forbidden partial
   overlap in tests/debug mode.
5. Refresh and expose optional raw input on every render only.
6. Implement all touch phases, `is_active`, raw/clamped/normalized position and
   malformed-dimension behavior. Never infer active state from XY.
7. Convert UQ16.16 tempo and dispatch lifecycle/tempo/touch methods.
8. Validate null descriptor, target, API, 48 kHz, stereo geometry, hooks and
   frame bounds with correct SDK error codes.

## Deliverables

- Public `nts3::prelude` runtime-facing API
- Safe adapters and callback-independent runtime functions
- Focused host tests plus an observable `fixtures/touch-probe` DSP fixture
- Handoff `plan/agents/handoffs/04-runtime-adapters.md`

## Acceptance checks

- Separate and in-place rendering match for 0/1/odd/max/random frame counts with
  guard bytes intact; Miri passes where supported.
- Every validation failure maps correctly and cannot touch uninitialized state.
- Runtime allocates zero bytes during render/reset/resume/suspend/touch/tempo.
- All five touch phases pass, including active `(0,0)` versus inactive `(0,0)`.
- Raw input is reacquired each render and cannot escape its buffer lifetime.
- Existing pass-through builds and the host touch probe is observably correct.

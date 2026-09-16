# Agent 03 task: deterministic SDRAM arena and runtime state foundation

## Context

FunDSP delay nodes require `alloc::vec::Vec`, while NTS-3 exposes up to 3 MiB of
external SDRAM only through hooks received in `unit_init`. Implement the target
allocator and lifecycle foundation without yet implementing audio, touch or
parameter adapters. Framework allocations are allowed only during
initialization. Rust scalar state belongs to static RAM and has no GC/object
header.

## Prerequisites

Tasks 01–02 must be complete or explicitly waived. Read both handoffs and run
ABI/pass-through smoke tests.

## Required reading

- `plan/agents/README.md`
- `plan/architecture.md` sections “Runtime and allocation” and “Generated ABI adapter”
- `plan/implementation.md` Phase 3 SDRAM/runtime items
- `plan/validation.md` allocator and runtime lifecycle tests
- SDK runtime descriptor/hook documentation and C dummy `unit_init/teardown`
- `example/smooth-echo-nts3-plug/README.md` memory notes

## Task

1. Create the target-safe portion of `crates/nts3` and its private runtime-state
   module, depending only on `nts3-sys` and minimal `no_std` dependencies.
2. Implement aligned bump-arena state initialized from exactly one
   `sdram_alloc` result. Retain the original pointer for `sdram_free`.
3. Track budget, requested bytes, alignment padding, current position and
   high-water. Support target-required alignments and checked 32-bit arithmetic.
4. Implement global allocator glue that is inactive before init, active only
   during plugin construction/initialize, sealed before readiness, and reset
   only during teardown. Deallocation is a documented no-op within the arena.
5. Add target-only panic/allocation-error policy without formatting/unwinding.
6. Build a runtime state machine covering uninitialized, initializing, ready,
   suspended and tearing down. Do not construct generic plugin state until arena
   allocation succeeds; drop state before freeing arena.
7. Implement a host tracking/probe mode with the same accounting semantics.
8. Add exhaustive allocation/alignment/overflow/reinit tests.

## Deliverables

- `crates/nts3` allocator and runtime-state modules
- Host allocator probe/test support
- A small allocating fixture proving `Vec` construction after arena activation
- Handoff `plan/agents/handoffs/03-sdram-allocator.md`

## Acceptance checks

- Target `no_std` check passes.
- Randomized host tests prove non-overlap/alignment and exact high-water math.
- Exact-fit succeeds; one-byte-overflow fails deterministically.
- Allocation before activation and after sealing fails in tests.
- Repeated init/drop/reset returns to clean state with one Korg alloc/free pair.
- No target dependency introduces `std`, threads, locks, JSON, or a second heap.
- Pass-through remains within previous ELF limits.

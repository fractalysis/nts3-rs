# Agent 09 task: ELF validation and truthful memory accounting

## Context

Add `./nts3 inspect` and mandatory post-build validation. Static ELF usage is
measurable; dynamic SDRAM behavior is not inferable solely from machine code.
The attribute embeds the exact declared arena in `.nts3_resources`, while a
native initialization probe measures allocator high-water. Stack and real-time
CPU remain unknown until measured and must be labeled honestly.

## Prerequisites

Tasks 01–08 complete/waived. Read their handoffs and inspect actual generated
ELFs before selecting an ELF parser.

## Required reading

- `plan/agents/README.md`
- `plan/implementation.md` Phase 7
- `plan/validation.md` ABI, memory/CPU, report sections
- SDK platform limits/linker scripts
- Task 01 C/Rust ELF goldens, Task 03 accounting, Task 07 resource schema
- `example/smooth-echo-nts3-plug/README.md`

## Task

1. Implement `./nts3 inspect <artifact>` in the container-side Rust tooling
   engine using a maintained ELF parser and wire it as the mandatory final step
   of `build`. Expert local mode may run the same engine on the host.
2. Validate class/data/machine/type/OSABI/EABI/hard-float, target/API/header,
   required exports, program headers, alignments, undefined symbols,
   relocations/PLT and prohibited/debug/unwind baggage.
3. Compute stripped file size, each `PT_LOAD` filesz/memsz and union ranges,
   static writable RAM and section breakdown without double counting.
4. Read/version-check `.nts3_resources` and enforce declared SDRAM <=3 MiB.
5. Build a dedicated native initialization probe that records
   `size_of::<Plugin/Parameters/Runtime>` and actual allocation high-water/margin
   for the concrete plugin. Isolate unrelated host allocations.
6. Emit stable human and JSON reports with toolchain/SDK/framework/FunDSP
   provenance and hashes. Label stack/CPU unknown unless genuinely measured.
7. Fail builds conservatively on approximately 32 KiB unit/load/static RAM
   limits. If exact loader semantics differ, document evidence and encode a
   named policy rather than an ad hoc exception.
8. Test malformed synthetic/fixture ELFs and report schema snapshots.

## Deliverables

- Inspector integrated into `crates/nts3-cli` and exposed only through `./nts3`
- Versioned memory JSON schema and text output
- Native high-water/size probe
- Invalid ELF fixture tests
- Handoff `plan/agents/handoffs/09-inspection-memory.md`

## Acceptance checks

- C dummy and valid Rust fixtures pass; each malformed invariant fails with a
  specific diagnostic.
- Reports reconcile with GNU size/readelf while explaining different totals.
- Reports distinguish artifact/load/static RAM/declared SDRAM/measured SDRAM.
- Post-init render/lifecycle loops show zero new allocations.
- Stack is not falsely reported as zero.
- Build fails when any synthetic threshold is exceeded.
- Smooth Echo report either passes or gives measured actionable overage; do not
  change its budget or DSP in this task.

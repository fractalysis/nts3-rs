# Agent task status

Allowed states: `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, `DONE`, `WAIVED`.
Only the user may set `WAIVED`. Agents update their own row and must link their
handoff when finished or blocked.

| Task | State | Handoff / blocker |
|---|---|---|
| 01 Toolchain and ELF spike | DONE | [handoff](handoffs/01-toolchain-elf-spike.md) |
| 02 Raw ABI | DONE | [handoff](handoffs/02-raw-abi.md) |
| 03 SDRAM allocator | DONE | [handoff](handoffs/03-sdram-allocator.md) |
| 04 Runtime adapters | DONE | [handoff](handoffs/04-runtime-adapters.md) |
| 05 Parameter core | DONE | [handoff](handoffs/05-parameter-core.md) |
| 06 Parameter derive | NOT_STARTED | — |
| 07 Plugin export | NOT_STARTED | — |
| 08 Build CLI | NOT_STARTED | — |
| 09 Inspection and memory | NOT_STARTED | — |
| 10 Smooth Echo | NOT_STARTED | — |
| 11 Release validation | NOT_STARTED | — |

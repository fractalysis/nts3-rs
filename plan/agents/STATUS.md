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
| 06 Parameter derive | DONE | [handoff](handoffs/06-parameter-derive.md) |
| 07 Plugin export | DONE | [handoff](handoffs/07-plugin-export.md) |
| 08 Build CLI | DONE | [handoff](handoffs/08-build-cli.md) |
| 09 Inspection and memory | NOT_STARTED | — |
| 10 Smooth Echo | NOT_STARTED | — |
| 11 Release validation | NOT_STARTED | — |

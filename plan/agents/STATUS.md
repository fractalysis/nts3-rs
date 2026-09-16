# Agent task status

Allowed states: `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, `DONE`, `WAIVED`.
Only the user may set `WAIVED`. Agents update their own row and must link their
handoff when finished or blocked.

| Task | State | Handoff / blocker |
|---|---|---|
| 01 Toolchain and ELF spike | IN_PROGRESS | — |
| 02 Raw ABI | NOT_STARTED | — |
| 03 SDRAM allocator | NOT_STARTED | — |
| 04 Runtime adapters | NOT_STARTED | — |
| 05 Parameter core | NOT_STARTED | — |
| 06 Parameter derive | NOT_STARTED | — |
| 07 Plugin export | NOT_STARTED | — |
| 08 Build CLI | NOT_STARTED | — |
| 09 Inspection and memory | NOT_STARTED | — |
| 10 Smooth Echo | NOT_STARTED | — |
| 11 Release validation | NOT_STARTED | — |

# Zero-context agent execution protocol

This directory replaces the former multi-agent workstream document. There is no
lead agent and no conversational memory between tasks. Run the task files in
numeric order, assigning each file to a fresh LLM context.

## Required procedure for every agent

1. Read this file, your complete task file, `plan/README.md`, and every source
   named under **Required reading** in the task file.
2. Read `plan/agents/STATUS.md`. Start only when every prerequisite task is
   `DONE` or explicitly `WAIVED` by the user. Never waive a gate yourself.
3. Inspect the actual workspace and rerun prerequisite smoke tests. Filesystem
   artifacts, tests, and handoff notes are the only trusted state.
4. Change your row to `IN_PROGRESS` before implementation.
5. Implement only the task's scope, but fix prerequisite defects that directly
   prevent it. Record such fixes.
6. Run every command in the task's **Acceptance checks**. Run all Docker commands
   from Bash; with Git Bash use `MSYS_NO_PATHCONV=1` for raw `docker` commands.
7. Write `plan/agents/handoffs/NN-<task>.md` containing:
   - summary and design decisions;
   - files added/changed;
   - exact commands and pass/fail results;
   - produced artifact paths and hashes where applicable;
   - memory/ABI observations;
   - unresolved risks or user decisions.
8. Mark the row `DONE` only when all acceptance checks pass. Otherwise mark it
   `BLOCKED`, preserve useful artifacts, describe the blocker, and ask the user
   for the smallest necessary decision. Do not weaken tests or limits.

## Global non-negotiable rules

- `example/smooth-echo-nts3-plug/src/lib.rs` is the normative public API fixture.
  Do not add per-plugin ABI/header/linker/panic/allocator boilerplate to make
  framework implementation easier.
- Keep target crates `no_std`; prove this with `thumbv7em-none-eabihf` builds.
- Do not edit `external/logue-sdk` except for disposable experiments. Copy no
  generated build products into it.
- Every unsafe block needs a local safety explanation and focused test.
- Keep host-only proc-macro, Docker, ELF-parser and CLI dependencies out of the
  target artifact.
- Do not silently increase memory budgets, relax 32 KiB checks, replace FunDSP,
  or patch the final ELF without documenting and testing why.
- Generated binaries are CI/output artifacts unless deliberately checked in as
  a small golden fixture with provenance.
- Hardware-unverified is not hardware-passed. Stack/CPU unknown is not zero.

## Ordered tasks

1. [01-toolchain-elf-spike.md](01-toolchain-elf-spike.md)
2. [02-raw-abi.md](02-raw-abi.md)
3. [03-sdram-allocator.md](03-sdram-allocator.md)
4. [04-runtime-adapters.md](04-runtime-adapters.md)
5. [05-parameter-core.md](05-parameter-core.md)
6. [06-parameter-derive.md](06-parameter-derive.md)
7. [07-plugin-export.md](07-plugin-export.md)
8. [08-build-cli.md](08-build-cli.md)
9. [09-inspection-memory.md](09-inspection-memory.md)
10. [10-smooth-echo.md](10-smooth-echo.md)
11. [11-release-validation.md](11-release-validation.md)

The sequence favors reliable handoffs over parallelism. A task may prepare
interfaces needed by its immediate successor, but must not absorb later tasks.

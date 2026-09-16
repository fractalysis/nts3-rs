# Agent 08 task: reproducible `cargo nts3` build workflow

## Context

Automate the already-proven manual process; do not replace it with an opaque new
link route. Plugin authors should need no `.cargo/config`, build.rs, linker
script, C file, crate-type setting or manual Docker command. Docker remains the
default reproducible backend and all Docker invocations occur through Bash.
Artifact inspection/memory analysis beyond essential post-link sanity belongs to
Task 09.

## Prerequisites

Tasks 01–07 complete/waived. Read Task 01's exact commands and Task 07's symbol
retention details. Rebuild generated pass-through manually first.

## Required reading

- `plan/agents/README.md`
- `plan/implementation.md` Phase 6
- `plan/architecture.md` crate boundaries
- `example/smooth-echo-nts3-plug/README.md` intended commands
- Container/build scripts and all prior handoffs

## Task

1. Create host-only `crates/cargo-nts3` with subcommands:
   `doctor`, `check`, `build`, and `new`. Reserve `inspect` wiring for Task 09.
2. `doctor` verifies Docker/image digest, SDK mount, Rust target, GNU tools,
   versions, Bash/path conversion and writable cache/output.
3. `check` runs the target no_std check with pinned policy.
4. `build` invokes Cargo for PIC static archive, final GNU/Korg link, strip and
   deterministic `.nts3unit`/map/report input paths. Preserve verbose command
   transcript and exact nonzero status.
5. Make Docker the default; expose clearly labeled expert local mode. Avoid
   recursive container invocation.
6. `new` generates only a concise parameter struct, DSP struct and annotated
   trait impl using the public facade.
7. Centralize release/LTO/panic/target/link flags in tooling, removing reliance
   on per-plugin profile/crate-type boilerplate where possible.
8. Add CLI integration tests with fake tools plus one real container build.

## Deliverables

- `crates/cargo-nts3`
- Installed/in-workspace `cargo nts3` command and Bash Docker launcher integration
- Deterministic build output layout and command transcript
- Template fixture/snapshot
- Handoff `plan/agents/handoffs/08-build-cli.md`

## Acceptance checks

- Clean environment: `doctor`, `check`, and pass-through/touch-probe builds pass.
- Two clean builds produce identical stripped hashes or documented/remediated
  nondeterminism.
- Generated template target-checks without manual ABI/build boilerplate.
- Windows Git Bash path handling is tested/documented with
  `MSYS_NO_PATHCONV=1` where needed.
- Smooth Echo reaches final-link stage through the CLI; size/memory policy is
  added next, not bypassed.
- Host-only CLI dependencies do not appear in target dependency tree/artifact.

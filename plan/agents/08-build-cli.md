# Agent 08 task: reproducible Bash build workflow

## Context

Automate the already-proven manual process; do not replace it with an opaque new
link route. Plugin authors should need no host Cargo installation,
`.cargo/config`, build.rs, linker script, C file, crate-type setting or manual
Docker command. The public entry point is the repository-root Bash launcher
`./nts3`. On Windows, it is run from WSL2 with Docker integration; native
Windows shells and Git Bash are not supported.

Use a split design: Bash owns Docker orchestration, while a tooling-only Rust
engine runs inside the pinned container and owns package selection, Cargo/GNU
commands, templates and diagnostics. The Rust engine must never launch Docker.
Cargo is still required to compile plugins, but only inside the container by
default. Do not provide or document a `cargo nts3` command.

Docker remains the default reproducible backend. Artifact inspection and memory
analysis beyond essential post-link sanity belongs to Task 09.

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

1. Add an executable repository-root `nts3` Bash launcher with public
   subcommands `doctor`, `check`, `build`, and `new`. Reserve `inspect` wiring
   for Task 09.
2. Create tooling-only `crates/nts3-cli` as the internal Rust engine. It runs in
   the pinned container by default and never searches for Bash, calls Docker,
   or performs Windows/WSL path conversion.
3. Have the launcher determine the repository/workspace root, invoke Docker
   from the Bash layer (directly or through `container/run.sh`), mount the
   workspace and caches, and run the Rust engine in the container. Forward
   arguments, stdout/stderr, signals and the exact nonzero status. Prevent
   recursive container invocation with an explicit internal contract rather
   than environment guessing.
4. Support Linux, macOS and WSL2 Bash. On Windows, require launching from WSL2
   with a working `docker` command. Do not add Git Bash, `cygpath`,
   `MSYS_NO_PATHCONV`, PowerShell, `cmd.exe`, `wsl.exe`, or native-Windows Cargo
   compatibility paths.
5. `doctor` verifies the outer Docker connection, pinned image/digest, workspace
   and SDK mounts, writable cache/output, then verifies the Rust target, Cargo,
   GNU tools and exact versions inside the container. Diagnostics must clearly
   identify whether an outer-launcher or in-container check failed.
6. `check` runs the target no_std Cargo check with pinned policy inside the
   container.
7. `build` invokes Cargo for the PIC static archive, then the proven GNU/Korg
   final link and strip flow, producing deterministic `.nts3unit`, map and
   report-input paths. Preserve a verbose command transcript and exact nonzero
   status.
8. Keep Docker as the default. Expose a clearly labeled expert local mode
   through the same `./nts3` launcher; local mode may require host Cargo and GNU
   tools, but it must bypass Docker explicitly and must not be selected
   automatically.
9. `new` generates only a concise parameter struct, DSP struct and annotated
   trait impl using the public facade. It must work through the default
   container path without host Cargo or Python.
10. Centralize release/LTO/panic/target/link flags in tooling, removing reliance
    on per-plugin profile/crate-type boilerplate where possible.
11. Add launcher tests with a fake `docker`, Rust-engine integration tests with
    fake Cargo/GNU tools, and one real container build. Test argument/status
    forwarding and paths containing spaces.

## Deliverables

- Repository-root executable `nts3` Bash launcher
- `crates/nts3-cli` internal Rust tooling engine
- WSL2/Linux/macOS Docker-launch integration with no `cargo nts3` entry point
- Deterministic build output layout and command transcript
- Template fixture/snapshot
- Handoff `plan/agents/handoffs/08-build-cli.md`

## Acceptance checks

- From WSL2, a clean environment with Bash and Docker but no host Cargo can run
  `./nts3 doctor`, `./nts3 check`, and pass-through/touch-probe builds.
- The Bash launch layer invokes `docker` directly in WSL2; no Git Bash
  discovery, `cygpath`, `MSYS_NO_PATHCONV`, `wsl.exe`, PowerShell or
  native-Windows path bridge appears in the implementation.
- Linux/macOS behavior is covered by launcher tests; platform-specific behavior
  is limited to clear capability checks rather than alternate command stacks.
- Two clean builds produce identical stripped hashes or documented/remediated
  nondeterminism.
- Generated template target-checks without manual ABI/build boilerplate.
- Smooth Echo reaches final-link stage through `./nts3`; size/memory policy is
  added next, not bypassed.
- Tooling-only dependencies do not appear in the target dependency tree or
  artifact.
- No `cargo-nts3` binary, Cargo alias, or documented `cargo nts3` invocation is
  added.

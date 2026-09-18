# Building NTS-3 units

Run the repository-root launcher from Bash on Linux or macOS. On Windows, run it
from WSL2 with Docker integration. The calling environment needs only Bash and
Docker; Rust, Cargo, and GNU ARM tools run in the pinned image.

```bash
./container/run.sh build-image
./nts3.sh doctor
./nts3.sh check -p pass-through
./nts3.sh build -p pass-through --release
./nts3.sh inspect target/nts3/pass_through.nts3unit
./nts3.sh new my-effect
```

`build` always applies the target CPU, PIC, panic-abort, release/LTO,
dead-section, symbol-retention, SDK linker, and strip policy. A plugin manifest
does not need a static-library crate type, linker script, build script, C file,
or Cargo configuration. For manifests without a static-library target, tooling
records an explicit `rustc` aggregation step before using the same proven GNU
final-link command. For a library target named `my_effect`, outputs are:

- `target/nts3/my_effect.nts3unit`
- `target/nts3/my_effect.elf`
- `target/nts3/my_effect.map`
- `target/nts3/my_effect.commands.txt`
- `target/nts3/my_effect.{readelf,nm,size}.txt`
- `target/nts3/my_effect.memory.{json,txt}`

Every build finishes by running the same ELF inspector exposed by `inspect`.
It also runs the concrete plugin's isolated native initialization probe and
fails on ABI, relocation, load, static RAM, declared SDRAM, measured SDRAM, or
post-initialization allocation violations. `inspect` validates an existing
artifact without rebuilding it; native probe fields are then explicitly absent.

`--verbose` echoes each recorded build command. Tool failures retain their exit
status.

## Expert local mode

`./nts3.sh --local <command>` explicitly bypasses Docker and requires compatible
host Rust/Cargo and GNU ARM tools. It is never selected automatically and is not
the reproducible default.

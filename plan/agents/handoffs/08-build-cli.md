# Task 08 handoff — reproducible Bash build workflow

## Status

**DONE.** The public launcher, container contract, internal tooling engine,
templates, fake-tool tests, real target builds, reproducibility regression, and
Smooth Echo final link all pass.

## Summary and design decisions

- Added executable repository-root `./nts3.sh` with `doctor`, `check`, `build`,
  and `new`. The user explicitly chose this filename instead of the plan's
  extensionless spelling. `inspect` is explicitly reserved for Task 09. The default is the pinned
  container; `--local` is a clearly announced expert-only bypass.
- Bash alone owns container startup. It resolves its own workspace root, checks
  the outer Docker connection/image label/workspace/SDK/output, mounts the
  workspace, read-only SDK, Cargo registry/git caches, and uses the explicit
  `__engine nts3-container-v1` contract to avoid recursive startup. `exec` is
  used at both boundaries for argument, stream, signal, and status forwarding.
- Added tooling-only `crates/nts3-cli`. It never invokes Docker or performs path
  translation. It owns package selection, target flags, release/LTO/panic
  policy, Cargo, the proven GNU SDK final link, stripping, command transcripts,
  concise diagnostics, and generation.
- Preserved the Task 01 pass-through route exactly when a package already emits
  a static archive; its resulting hash remains canonical. For normal plugin
  manifests without `crate-type`, Cargo first emits the locked PIC rlib and an
  explicit transcripted `rustc` aggregation step creates the static archive.
  GNU final linking, rooted exports, SDK linker script, and stripping are then
  identical. Smooth Echo and Touch Probe prove this no-boilerplate route.
- Every build emits deterministic flat paths under `target/nts3/`: `.nts3unit`,
  `.elf`, `.map`, `.commands.txt`, and readelf/nm/size report inputs. Essential
  post-link sanity checks cover ELF32/little-endian/System-V/ARM/ET_DYN/
  hard-float, required exports, unique Thumb callback addresses, and undefined
  dynamic symbols; deeper artifact and memory policy remains Task 09.
- The pinned image now carries a checked outer-launch contract label and seeds
  writable Cargo cache mount points. The lower-level container helper now uses
  one portable Bash/Docker command stack.
- `new` creates `plugins/<name>`, adds it to workspace members, and writes only a
  small manifest plus a parameter struct, DSP struct, and annotated public-trait
  implementation. No crate type or target/build ABI boilerplate is generated.
- Smooth Echo's library target is named `smooth_echo`, matching its documented
  `target/nts3/smooth_echo.nts3unit` path. Its Rust source was not changed.

## Files added or changed

Added:

- `nts3.sh` (executable public launcher; user-selected filename)
- `crates/nts3-cli/{Cargo.toml,README.md,src/main.rs}`
- `crates/nts3-cli/tests/fake_tools.rs`
- `crates/nts3-cli/tests/fixtures/new-plugin.rs`
- `tests/cli/launcher.sh`
- `docs/building.md`
- this handoff

Changed:

- `Cargo.toml`, `Cargo.lock`
- `container/{Dockerfile,README.md,run.sh}`
- `fixtures/pass-through/build.sh`
- `example/smooth-echo-nts3-plug/Cargo.toml`
- `tests/golden/pass-through.map.txt` (only the new flat ELF output path)
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` and no Smooth Echo Rust source was edited.
The pass-through manifest retains its historical Task 01 static-library setting
so its hardware-proven archive route and bytes remain a regression reference;
new and existing ordinary plugins do not require that setting.

## Commands and results

Native Cargo commands were run through PowerShell. Docker commands were run
from Bash. The public launcher was exercised end-to-end against real Docker;
the fake Docker harness additionally covered exact failure-status forwarding
and a workspace path containing spaces.

1. Prerequisite workspace and generated pass-through smoke — **PASS**:

   ```powershell
   cargo test --workspace
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   ```

   ```bash
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

2. Image rebuild and doctor contract — **PASS**:

   ```bash
   docker build --build-arg BASE_IMAGE=xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba \
     --tag nts3-rs-toolchain:rust-1.98.1 container
   docker run --rm --init <workspace/cache mounts> \
     nts3-rs-toolchain:rust-1.98.1 \
     /workspace/nts3.sh __engine nts3-container-v1 doctor
   ```

   Doctor verified Rust/Cargo 1.98.1, installed
   `thumbv7em-none-eabihf`, GNU ARM GCC 9.2.1 20191025, binutils 2.34,
   workspace/SDK mounts, writable output, and writable cache. Final local image
   ID: `sha256:138ef2f0824103bd33612037915656f0191348c18dd45f321b89cbe04a87693e`.

3. Launcher and fake tool integration — **PASS**:

   ```bash
   ./tests/cli/launcher.sh
   docker run --rm --volume <workspace>:/workspace --workdir /workspace \
     nts3-rs-toolchain:rust-1.98.1 cargo test --locked -p nts3-cli
   ```

   Launcher tests proved exact status `37`, argument forwarding, explicit
   internal contract, mounts, and spaces in workspace/package paths. Rust tests
   proved fake Cargo/GNU sequencing, deterministic output names/transcript, and
   exact Cargo failure status `43`.

4. Public launcher target checks/builds — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./nts3.sh doctor
   MSYS_NO_PATHCONV=1 ./nts3.sh check -p smooth-echo-nts3-plug
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p pass-through --release
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p touch-probe --release
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p smooth-echo-nts3-plug --release
   ```

   Pass-through, Touch Probe, and Smooth Echo all reached and passed GNU final
   link, strip, report generation, and essential post-link sanity.

5. Reproducibility and old ELF regression — **PASS**:

   ```bash
   docker run --rm --volume <workspace>:/workspace --workdir /workspace \
     nts3-rs-toolchain:rust-1.98.1 \
     /workspace/fixtures/pass-through/verify.sh
   ```

   Two package-clean builds matched exactly. The official C dummy comparison,
   export/Thumb uniqueness, relocation/PLT, 32 KiB, and writable-load checks
   still pass.

6. Generated template target check — **PASS**:

   ```powershell
   cargo run --quiet --locked -p nts3-cli -- new task08-check
   cargo check -p task08-check --target thumbv7em-none-eabihf
   ```

   The temporary generated package was removed after validation and the root
   member list restored. Its checked source is retained as the template fixture.

7. Final native suite and target dependency audit — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   cargo tree -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf `
     -e normal,no-proc-macro
   ```

   The target tree contains `fundsp`, `nts3`, and their target dependencies; it
   does not contain `nts3-cli`, serde, or serde_json.

8. Script/source hygiene — **PASS**:

   ```bash
   bash -n nts3.sh container/run.sh fixtures/pass-through/build.sh \
     tests/cli/launcher.sh
   git diff --check
   ```

## Artifacts, hashes, and observations

Generated artifacts are gitignored under `target/nts3/`:

| Unit | SHA-256 | Stripped bytes | `.text` | `.bss` |
|---|---|---:|---:|---:|
| `pass_through.nts3unit` | `d598283d80da55127e86b2182504de7f29635a3866352cd70ea08c5887ed21c2` | 4,860 | 898 | 100 |
| `touch_probe.nts3unit` | `9454b216f8bd8bfaf9353ff9cb569b609c6927c9150ed3d20dc3f36360180c9a` | 5,028 | 1,068 | 104 |
| `smooth_echo.nts3unit` | `0f18f7a082959b5da20de7ef043ea5268ab24705177abd7d5713b67e1e1cd1d6` | 13,284 | 5,170 | 268 |

Each has one 376-byte unit header, one 12-byte resource record, every required
export, distinct odd dynamic callback values, and no undefined dynamic symbol.
Pass-through remains byte-identical to Task 07's canonical artifact. Smooth
Echo declares 1,100,000 SDRAM bytes; actual SDRAM high-water, complete load/RAM
accounting, stack, and CPU remain for Tasks 09–10. Hardware status is unchanged:
pass-through's canonical hash family is hardware-proven; newly packaged Touch
Probe and Smooth Echo are hardware-unverified.

## Remaining risks / next task

- Task 09 must add full artifact limits, unioned segment accounting, resource
  parsing, memory JSON/text, relocation/PLT policy, and the public `inspect`
  implementation. Current checks intentionally stop at essential link sanity.
- The no-crate-type aggregation step is explicit and tested but produces a
  different archive optimization root than a manifest-declared static library;
  therefore the historical pass-through keeps the direct route as the binary
  regression fixture. Both routes use the same pinned Rust flags and exact GNU
  final link.
- Touch Probe and Smooth Echo still require hardware validation. Worst-case
  stack and real-time CPU remain unknown.

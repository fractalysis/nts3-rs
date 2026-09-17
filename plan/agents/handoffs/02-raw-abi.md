# Task 02 handoff — audited `nts3-sys` raw ABI

## Status

**DONE.** Host/target checks, the independent C/Rust ABI comparison, header
serialization, and the pass-through ELF regression checks all pass.

## Summary and design decisions

- Added a dependency-free, `#![no_std]` `nts3-sys` crate containing only the
  NTS-3 genericfx SDK API 2.0 raw surface needed by later runtime work.
- Raw callback aliases are `unsafe extern "C" fn`; nullable callbacks in runtime
  structures use `Option<...>`, whose pointer-sized layout is compile-time and
  C-probe checked.
- SDK packed structures are represented with `repr(C, packed)`; naturally
  aligned hook/context structures use `repr(C)`. Packed fields are private and
  exposed only through by-value constructors/accessors, preventing downstream
  references to unaligned fields.
- C bitfields are explicit one-byte `repr(transparent)` values:
  `UnitParamFormat` and `GenericfxCurve`. Their checked constructors reject
  out-of-range components and accessors expose raw/component values.
- Added compile-time assertions for every shared Rust structure's size,
  alignment, and offsets (including pointer-width-dependent structures).
- Added a native C probe built directly against checked-in Korg headers. It and
  a Rust probe emit an ordered 149-record report covering all shared layouts,
  constants, callback signatures, and all 376 serialized bytes of Korg's dummy
  genericfx header. The Docker harness diffs the reports exactly.
- The SDK image lacked a native C compiler, so the derived image now installs
  pinned Ubuntu `gcc=4:9.3.0-1ubuntu2` and
  `libc6-dev=2.31-0ubuntu9.18`. Actual GCC executable version is 9.4.0.
- Replaced all duplicated ABI declarations in `pass-through` with `nts3-sys`.
  The final stripped unit remains byte-identical to Task 01's hardware-tested
  artifact, proving no symbols, relocations, bytes, or size changed.
- Korg's BSD 3-Clause text and source attribution are retained in the crate.

## Files added or changed

Added:

- `crates/nts3-sys/Cargo.toml`
- `crates/nts3-sys/src/lib.rs`
- `crates/nts3-sys/examples/abi_probe.rs`
- `crates/nts3-sys/{README.md,LICENSE-KORG-BSD-3-Clause}`
- `tests/abi/{README.md,probe.c,compare.sh}`
- this handoff

Changed:

- `Cargo.toml`, `Cargo.lock`
- `container/{Dockerfile,README.md}`
- `fixtures/pass-through/{Cargo.toml,README.md,src/lib.rs}`
- `tests/golden/pass-through.map.txt` (only Rust archive/object identity hashes
  changed; linked addresses and bytes did not)
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` was edited.

## Commands and results

Docker commands were run from Bash; direct Cargo commands were run from
PowerShell as requested.

1. Prerequisite Task 01 smoke test before edits — **PASS**:

   ```bash
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

   SHA-256 was the Task 01 value
   `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`.

2. Derived image rebuild with native C toolchain — **PASS**:

   ```bash
   ./container/run.sh build-image
   ```

   Local image ID for the final build:
   `sha256:e200fbce9e124f6007687335444038d29a6b0f2949ad8d811376d8b264f0bbb2`.

3. Host formatting/tests and host/target checks — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo check -p nts3-sys --target thumbv7em-none-eabihf
   cargo check -p pass-through --target thumbv7em-none-eabihf
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p nts3-sys --target thumbv7em-none-eabihf -- -D warnings
   cargo clippy -p pass-through --target thumbv7em-none-eabihf -- -D warnings
   ```

   `nts3-sys`: 3 tests passed; `pass-through`: 2 tests passed.

4. Final ELF regression followed by C/Rust ABI comparison — **PASS**:

   ```bash
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ./container/run.sh ./tests/abi/compare.sh
   ```

   ABI result: `149 records`, exact 376-byte header match.
   ELF result: reproducible, 3,648 bytes, writable PT_LOAD memory 116 bytes.

5. Script and whitespace checks — **PASS**:

   ```bash
   bash -n container/run.sh fixtures/pass-through/build.sh \
     fixtures/pass-through/verify.sh tests/abi/compare.sh
   git diff --check
   ```

## Artifacts, hashes, and ABI observations

Generated artifacts are gitignored under `target/`:

- `target/abi/{c.txt,rust.txt}`
  - both SHA-256:
    `3fbdcd3af3d02e235df545de70509212853d0d7aa73cbd98641ca51935976d92`
  - both 4,458 bytes and 149 lines
- `target/nts3/pass-through/pass_through.nts3unit`
  - SHA-256:
    `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`
  - 3,648 bytes; exact Task 01 hardware-tested binary

Important measured layouts:

- `UnitParam`: size 32, alignment 1
- `GenericfxParamMapping`: size 8, alignment 1
- `UnitHeader`: size 312, alignment 1
- `GenericfxUnitHeader`: size 376, alignment 1
- On 64-bit probe host: hooks size/alignment 32/8, packed descriptor 48/1,
  genericfx context 16/8
- On 32-bit target (compile-time asserted): hooks 16/4, packed descriptor 32/1,
  genericfx context 12/4

The C and Rust dummy header bytes are identical, including parameter fraction
and mapping curve/polarity bitfields. The pass-through remains relocation-free,
has no PLT or undefined dynamic symbols, and preserves distinct Thumb callback
addresses.

## Unresolved risks / decisions

- No new hardware run was required because the resulting pass-through unit is
  byte-for-byte identical to the Task 01 artifact already accepted on firmware
  `1.1.x`. Hardware status for any future binary remains unverified until run.
- Native apt package dependencies are version-pinned at their requested top-level
  package versions, but transitive Ubuntu package resolution still depends on
  the Ubuntu 20.04 repositories. This affects only the host C probe, not target
  output.
- Stack/CPU behavior remains unknown, unchanged from Task 01.

## Next task prerequisite smoke commands

From Bash:

```bash
./container/run.sh ./tests/abi/compare.sh
./container/run.sh ./fixtures/pass-through/verify.sh
```

From PowerShell:

```powershell
cargo test --workspace
cargo check -p nts3-sys --target thumbv7em-none-eabihf
```

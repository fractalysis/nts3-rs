# Rust pass-through fixture

This is the smallest framework-authored unit. Its source contains only a
parameter declaration, an `Nts3Plugin` implementation, and one
`#[nts3::plugin(...)]` attribute. The attribute generates the unit header,
resource metadata, runtime storage, all C callbacks, allocator, and panic
policy; no ABI code is copied into the fixture.

Build and perform the C/Rust ELF comparison in the pinned container:

```bash
./container/run.sh ./fixtures/pass-through/verify.sh
```

`build.sh` preserves the complete Cargo and GNU final-link command. Outputs are
under `target/nts3/pass-through/`:

- `pass_through.nts3unit` (stripped loadable ELF)
- `pass_through.elf` and `pass_through.map`
- readelf, objdump, nm, size, and SHA-256 reports
- the official C dummy and its map under `c-dummy/`

Normalized C/Rust reports are refreshed under `tests/golden/`. The verifier
requires ELF32 little-endian ARM ET_DYN, System V OSABI, EABI5 hard-float,
exactly one unit header and one `.nts3_resources` section, every generated
symbol in `.dynsym`, a distinct odd (Thumb) address for every callback, no
undefined symbol, and no call relocation or PLT in the Rust output. Distinct
callback addresses are a hardware-proven loader requirement: LLVM's default
alias-based identical-function merging was silently rejected. The verifier also
enforces the 32 KiB file and writable-load limits and builds the artifact twice.

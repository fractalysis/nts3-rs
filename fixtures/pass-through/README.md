# Manual Rust pass-through spike

This crate intentionally duplicates the minimum NTS-3 ABI. It is evidence for
the Rust/static-archive/GNU-linker path, not reusable framework API; Task 02
replaces these definitions with audited `nts3-sys` types.

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
exactly one unit header, every callback in `.dynsym`, a distinct odd (Thumb)
address for every required callback, no undefined symbol, and no call relocation
or PLT in the Rust output. Distinct callback addresses are a hardware-proven
loader requirement: LLVM's default alias-based identical-function merging was
silently rejected. The verifier also enforces the 32 KiB file and writable-load
limits and builds the Rust artifact twice.

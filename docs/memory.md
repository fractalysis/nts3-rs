# Artifact and memory reports

`./nts3.sh build` always emits `<unit>.memory.json` and `<unit>.memory.txt` after
stripping and before reporting success. `./nts3.sh inspect <artifact>` runs the
same validator on an existing ELF. The JSON schema is versioned by the top-level
`schema_version` field; the current version is **1**.

The named `nts3-framework-v1` policy enforces:

- ELF32, little-endian ARM `ET_DYN`, System V ABI, EABI5 hard-float;
- the NTS-3 genericfx target, SDK API 2.0, one 376-byte unit header, all required
  dynamic exports, distinct Thumb callback addresses, and resource schema v1;
- no undefined dynamic symbols, PLT, TLS, debug/symbol-table baggage, RELA, or
  relocation other than symbol-free `R_ARM_RELATIVE`;
- at most the one 8-byte ARM cantunwind index emitted by the proven Rust build;
- SDK `max-page-size=128` load alignment, valid file/memory ranges, and no
  writable-executable load;
- a conservative 32 KiB ceiling independently for the stripped artifact,
  unioned `PT_LOAD` memory, and unioned writable `PT_LOAD` memory;
- declared SDRAM in `1..=3 MiB` and, when building, native initialization
  high-water below the declaration with a margin of at least 16 KiB or 2%,
  whichever is larger (the margin rule applies when dynamic allocation is used);
- zero successful allocations through 256 post-initialization render and
  lifecycle/event stress iterations.

The official Korg C dummy predates `.nts3_resources` and uses local C++
relocations and PLT entries known to be accepted by the loader. It is inspected
under the explicit `korg-sdk-legacy-v1` comparison policy. That policy still
checks the ABI, exports, target/API, loads, and 32 KiB limits, but reports
resource/probe quantities as unavailable and warns about the legacy relocation
model. Framework builds never use this exception.

## Meanings of reported quantities

- **Artifact bytes**: bytes in the stripped `.nts3unit` file. This includes ELF
  section tables and non-loaded metadata, so it is not GNU `size`'s `text + data
  + bss` total.
- **PT_LOAD file union**: union of loadable file-offset ranges. Overlaps are
  counted once.
- **PT_LOAD memory union**: union of loadable virtual-memory ranges. It includes
  zero-fill and alignment gaps and is the conservative load-limit policy.
- **Static writable RAM**: union of writable `PT_LOAD` memory ranges. The report
  also lists writable sections (`.dynamic`, `.got`, `.data`, `.bss`, `.stack`,
  and any others) without summing overlapping ranges twice.
- **Declared target SDRAM**: exact arena reservation embedded in
  `.nts3_resources`. It is not inferred from machine code.
- **Measured initialization SDRAM**: high-water from constructing the concrete
  plugin and parameters with the same bump allocator in a dedicated native
  process. Unrelated process allocations occur while the arena is inactive and
  go to the system allocator.
- **Plugin/Parameters/Runtime sizes**: native `size_of` measurements. They are
  useful structural diagnostics, not target SDRAM usage.

Worst-case stack and NTS-3 real-time CPU are always `unknown` until measured on
hardware. The linker script's four-byte `.stack` sentinel is static load
metadata, not a stack watermark and never justifies reporting zero usage.

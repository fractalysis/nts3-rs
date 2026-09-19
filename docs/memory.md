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

`build` and standalone `inspect` disassemble every exported callback and reject
an own machine-code stack frame above 624 bytes. Controlled pass-through units
that differed only in forced `unit_init` frame size established the boundary on
the tested NTS-3: 624 bytes works and 632 bytes hard-locks the device.

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
- **Callback own-frame stack**: `build` and `inspect` disassemble every exported
  callback, write `<unit>.stack.txt`, and reject any own frame above 624 bytes.
  This catches the empirically demonstrated callback-frame failure before use.

The checked-in NTS-3 SDK does **not** publish the firmware callback-thread stack
size. Its `unit.ld` sets `__stack_size = 0` because a unit uses a firmware-owned
stack rather than reserving a private one; that is not a zero-byte limit. The
4,096-byte arrays in NTS-3 `wasm.cc` files belong to the browser simulator's
WebAudio worker and are not hardware specifications.

The 624-byte limit applies to an exported callback's own frame on the tested
firmware/device, not to a documented total stack allocation. Worst-case
transitive stack and NTS-3 real-time CPU remain `unknown`. The callback report
does not include firmware caller usage, nested callees, recursion, interrupts,
or a hardware watermark. The linker script's four-byte `.stack` sentinel is
static load metadata and never justifies reporting zero usage.

# nts3-sys

Audited, `no_std` raw Rust definitions for the Korg NTS-3 generic effect ABI at
SDK API 2.0. The crate intentionally contains no allocator, runtime state,
parameter framework, proc macro, or build tooling.

The definitions are derived from Korg's `runtime.h` and `unit_genericfx.h` and
retain Korg's BSD 3-Clause attribution in
[`LICENSE-KORG-BSD-3-Clause`](LICENSE-KORG-BSD-3-Clause). Packed structures use
by-value constructors and accessors so callers do not need references to
unaligned fields. C bitfields are represented by explicit one-byte wrapper
types.

Run the independent C/Rust ABI comparison in the pinned container:

```bash
./container/run.sh ./tests/abi/compare.sh
```

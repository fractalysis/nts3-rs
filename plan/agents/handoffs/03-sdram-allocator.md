# Task 03 handoff — deterministic SDRAM arena and runtime state

## Status

**DONE.** Host allocator/lifecycle tests, target `no_std` checks, target static
archive link probe, ABI comparison, and pass-through ELF regression all pass.

## Summary and design decisions

- Added dependency-minimal `#![no_std]` crate `nts3`, depending only on
  `nts3-sys` on target.
- Implemented one-block aligned bump allocation with a target-like 32-bit
  address model. Successful allocations account separately for payload bytes,
  alignment padding, current position, high-water position, and allocation
  count. Failed allocations do not mutate accounting.
- The arena retains the exact pointer returned by Korg. Initialization checks the
  3 MiB ceiling and `sdram_avail`, invokes `sdram_alloc` exactly once, then
  returns the original pointer to the matching `sdram_free` exactly once after
  state drop.
- Individual deallocation is intentionally a no-op for arena-owned memory. The
  allocator accepts allocations only in its active construction phase, rejects
  them after sealing, and cannot be publicly unsealed.
- Added a private `RuntimeState<T>` over `MaybeUninit<T>` with
  Uninitialized/Initializing/Ready/Suspended/TearingDown transitions. Arena
  activation precedes construction; sealing precedes Ready; teardown hook and
  `T` drop precede whole-arena release.
- Added host `HostArena` probe support. Its inactive global allocator path
  delegates to `System`, while its active and sealed behavior uses the same arena
  and accounting implementation as target. Probe values cannot escape the
  closure.
- Added a real `Vec<u64>` host fixture, both as an integration test and runnable
  example. Its one 1,024-byte capacity allocation is served after activation and
  dropped before reset.
- Stable Rust 1.85.1 does not support `#[alloc_error_handler]`. The target
  `GlobalAlloc` therefore enters a cold non-formatting spin fault directly on
  inactive, sealed, exhausted, or invalid allocation instead of returning null.
  The hidden `__install_runtime_glue!` macro installs the global allocator and
  non-formatting panic handler in the final plugin crate. A target staticlib
  probe using this macro and `alloc::vec::Vec` linked successfully. Task 07's
  plugin attribute must emit this macro exactly once.
- The no-lock `UnsafeCell` singleton follows the SDK C++ template's serialized
  plain-mutable lifecycle. Rendering starts only after sealing. This assumption
  remains subject to later hardware/concurrency evidence.

## Files added or changed

Added:

- `crates/nts3/Cargo.toml`
- `crates/nts3/README.md`
- `crates/nts3/src/lib.rs`
- `crates/nts3/src/allocator.rs`
- `crates/nts3/src/runtime_state.rs`
- `crates/nts3/tests/allocating_fixture.rs`
- `crates/nts3/examples/allocating_probe.rs`
- this handoff

Changed:

- `Cargo.toml` (adds `crates/nts3` workspace member)
- `Cargo.lock`
- `plan/agents/STATUS.md`

No source under `external/logue-sdk` was edited. Pre-existing unrelated
untracked SDK/reference/example files were not modified.

## Validation commands and results

Docker commands were run from Bash as required.

1. Prerequisite and final ABI/ELF smoke checks — **PASS**:

   ```bash
   ./container/run.sh ./tests/abi/compare.sh
   ./container/run.sh ./fixtures/pass-through/verify.sh
   ```

   ABI reports matched all 149 records and all 376 header bytes. The
   pass-through remained reproducible and byte-identical to the hardware-tested
   Task 01/02 unit.

2. Host tests, target check, and lint — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo check -p nts3 --target thumbv7em-none-eabihf
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p nts3 --target thumbv7em-none-eabihf -- -D warnings
   ```

   `nts3`: 12 unit tests and 1 allocating integration fixture passed. Workspace
   tests also passed (`nts3-sys`: 3; pass-through: 2).

3. Runnable host allocation fixture — **PASS**:

   ```powershell
   cargo run -p nts3 --example allocating-probe
   ```

   The fixture observed one allocation, 1,024 requested bytes, and a 1,024-byte
   high-water mark inside a 16 KiB host arena.

4. Target root/staticlib link probe — **PASS**:

   A temporary `#![no_std]`, `crate-type = ["staticlib"]` crate invoked
   `nts3::__install_runtime_glue!()`, constructed and pushed to
   `alloc::vec::Vec<u32>`, and was built with:

   ```bash
   cargo build --manifest-path target/allocator-link-probe/Cargo.toml \
     --target thumbv7em-none-eabihf --release
   ```

   This specifically proved the allocator/panic glue must expand in the final
   crate (dependency-level lang items alone are not sufficient) and that the
   stable target link succeeds with `alloc::Vec`.

5. Source hygiene — **PASS**:

   ```bash
   git diff --check
   ```

## Artifacts, hashes, and memory observations

Generated artifacts are gitignored:

- `target/debug/examples/allocating_probe.exe`
  - SHA-256:
    `92b41d031afeb43bb571cd0136e46c23f7d8f8f8b81f04f6589d310d093e1b3e`
- temporary target staticlib probe before the final Docker clean
  - SHA-256:
    `8372647d1599d94c965f78bc8896a1706b189f3a9c23eeefb8f462340d200869`
- `target/nts3/pass-through/pass_through.nts3unit`
  - SHA-256:
    `8f2f70bcc64d54b394692eede58573fc3620032fff935ccd5ec44d26fdeae5a6`
  - 3,648 bytes; writable PT_LOAD memory remains 116 bytes.

`AllocationStats` is six `u32` fields (24 bytes) and all arena counters use
checked `u32` arithmetic. Randomized tests cover 32,768 successful layouts
(512 arenas × 64 allocations), non-overlap, absolute-address alignment, and
exact requested/padding/high-water equations. Dedicated tests cover all powers
of two through 1 MiB, exact fit, one-byte overflow, address wraparound,
inactive/sealed rejection, null Korg allocation, unavailable budget,
construction failure cleanup, and three clean init/drop/reset cycles.

The target dependency tree is only:

```text
nts3
└── nts3-sys
```

No target `std`, threads, locks, JSON, host allocator, proc-macro, or second heap
is introduced.

## Unresolved risks / next-task notes

- `RuntimeState<T>` is intentionally private and callback-independent. Task 04
  should build `Runtime<P>` and descriptor/context validation around it rather
  than exposing it to plugin authors.
- Task 07 must emit `nts3::__install_runtime_glue!()` exactly once from the
  plugin attribute. This root expansion is required for Rust's global allocator
  and panic lang items.
- Target OOM/panic behavior is a deliberate infinite fault loop with no
  formatting or unwind. There is no diagnostic channel at this layer.
- The SDK template supports the current serialized/no-lock assumption, but
  callback preemption/concurrency is still not documented conclusively.
- Stack and CPU usage remain unknown. No new runtime-containing unit has yet
  been exercised on hardware; the unchanged pass-through binary retains its
  prior hardware result.

## Next task prerequisite smoke commands

From Bash:

```bash
./container/run.sh ./tests/abi/compare.sh
./container/run.sh ./fixtures/pass-through/verify.sh
```

From PowerShell:

```powershell
cargo test -p nts3 --all-targets
cargo check -p nts3 --target thumbv7em-none-eabihf
cargo run -p nts3 --example allocating-probe
```

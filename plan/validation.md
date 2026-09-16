# Validation and acceptance gates

## 1. Host unit tests

### Parameters

- Generated header and runtime defaults are byte/value consistent.
- Every display/fraction type used by the example encodes correctly.
- Values below/above bounds clamp before storage.
- Unknown IDs cannot index arrays.
- Smoothing matches the pinned Baseplug one-pole reference sequence and
  coefficients across sample rates, block partitions, resets and mid-flight
  retargeting. Nonpositive milliseconds update immediately. Active →
  Deactivating → Inactive settlement uses the Baseplug `1e-5` threshold and
  ultimately snaps exactly to target; tests must not incorrectly interpret
  `smoothing_ms` as a linear ramp completion time.

### Touch

For every phase, test raw and normalized coordinates. Mandatory assertion:

```text
Began(0,0).is_active()       == true
Stationary(0,0).is_active()  == true
Ended(0,0).is_active()       == false
Cancelled(0,0).is_active()   == false
```

Also test non-1024 dimensions, zero dimensions and out-of-range coordinates.

### Buffer safety

Run identical DSP against separate and exact in-place storage for frame counts
0, 1, odd, maximum, and several randomized values. Verify guard bytes around
buffers. Test partial overlap detection in debug/test mode. Miri-test the
iterator where possible.

### Runtime lifecycle

Cover init rejection for null/wrong target/API/rate/geometry/hooks, successful
init, reset, suspend/resume, render, teardown, and a second complete lifecycle.
No callback may dereference uninitialized or torn-down state.

### Allocator

Test alignment from 1 through the target's required maximum, exact-fit and
one-byte-overflow, high-water accounting, sealed allocations, and arena reset.
Use a target-like 32-bit pointer arithmetic model in addition to native tests.

## 2. Macro compile tests

Use `trybuild` for valid examples and each diagnostic:

- More than eight parameters.
- Bad min/default/max or mapping range.
- Name too long/non-ASCII/illegal character.
- Unsupported type/curve/assignment.
- Duplicate explicit index.
- Reserved/invalid developer ID.
- Name >19, bad semantic version component, SDRAM >3 MiB.
- Attribute applied to the wrong trait/item.
- More than one exported plugin.

Snapshot diagnostics so author-facing quality does not regress.

## 3. ABI conformance

Inside the pinned Docker image, compile a C probe against the checked-in Korg
headers and compare Rust:

- `sizeof`, alignment and offset of all shared structs.
- Header and mapping byte serialization.
- Target/API/error/phase/mapping constants.
- Function pointer signatures.

Then compare pass-through Rust and C dummy ELF headers, program headers,
sections, symbols and relocations. Keep normalized golden outputs in `tests/`.

## 4. Cross-build gates

Every release/CI build runs:

```bash
cargo test --workspace
cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
cargo nts3 build -p pass-through --release
cargo nts3 build -p touch-probe --release
cargo nts3 build -p smooth-echo-nts3-plug --release
cargo nts3 inspect <each artifact>
```

Execute Docker through Bash. On Git Bash set `MSYS_NO_PATHCONV=1` for raw Docker
commands.

Fail on any artifact invariant or memory limit. Save `.map`, readelf summary and
memory JSON as CI artifacts.

## 5. Memory/CPU gates

### Static and artifact

- Stripped unit and load limits pass conservatively.
- Static writable data is below 32 KiB with useful headroom.
- Reports show each segment/section, not only GNU `size` totals.
- Release artifact contains no accidental desktop/`std` dependency.

### SDRAM

- Smooth Echo declares 1,100,000 bytes, below 3 MiB.
- Native initialization probe peak is below declaration with at least the
  selected policy margin (recommend max of 16 KiB or 2%). If not, increase the
  declaration after checking the device allowance.
- Two delays account for 1,048,576 payload bytes; unexplained overhead is listed.
- Allocation counter remains unchanged through long render, reset, touch,
  parameter and tempo stress loops.

### Stack and CPU

ELF inspection does not prove worst-case stack or real-time CPU. If supported,
add stack-size metadata; otherwise mark unknown. On hardware use a stack
watermark only if memory ownership/API make that safe. Record audio dropout/CPU
stress results at the maximum runtime buffer and with four simultaneous unit
instances.

## 6. DSP tests

At 48 kHz:

- Silence remains finite and near zero.
- Impulse delay onset agrees with selected time.
- Time sweeps are smoothed and do not produce non-finite samples.
- Feedback 0, typical values, 0.99 boundary and freeze branch remain bounded as
  designed.
- Reset clears delay/filter state while retaining parameter values.
- Separate and in-place output match within floating tolerance.
- A long randomized run produces no NaN/Inf and no allocation.

Create reference WAV/data from the NIH version or a small extracted reference
harness. Compare behavior rather than host-wrapper details; record any FunDSP
0.15→0.21 numerical differences.

## 7. Required hardware checklist

### Pass-through fixture

- Install/select unit; verify clean stereo audio.
- Bypass/select/suspend/resume repeatedly.
- Load the same unit in each of four runtimes.
- Power-cycle and reload saved program.

### Touch probe

- Make active state audibly/observably distinct from inactive state.
- Touch exact/near bottom-left: verify active at coordinates `(0,0)`.
- Release without moving: verify inactive while last coordinates remain `(0,0)`.
- Verify began, movement, stationary refresh, ended and UI-change cancellation.

### Smooth Echo

- X changes time over 1–2000 ms with expected curve; Y changes feedback.
- Defaults are 500 ms and 0%.
- Sweep X rapidly and listen/record for smooth modulation.
- Exercise near-unity freeze, release, reset, suspend and resume.
- Run sustained stereo full-scale and silence inputs; check clipping, NaN-like
  failure, noise, dropouts and channel correctness.
- Instantiate four copies with different settings to validate per-runtime SDRAM
  assumptions.
- Repeat unload/reload to exercise teardown and arena return.

Record firmware, unit hash, toolchain versions and results. No plan should call
the hardware work complete based only on ELF validation.

## 8. Release acceptance report

Publish one concise report with:

- Commit/toolchain/SDK versions.
- Commands and hashes of all three unit files.
- ABI inspection result.
- Code/load/static RAM/SDRAM numbers and margins.
- Allocation high-water and post-init allocation count.
- DSP test summary.
- Hardware matrix or explicit `BLOCKED: no device` status.
- Known limitations, especially stack/CPU quantities not statically proven.

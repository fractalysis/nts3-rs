# Task 10 handoff — Smooth Echo completion and optimization

## Status

**BLOCKED: hardware hard fault under investigation.** The first Smooth Echo
artifact installed successfully but selecting it completely locked the NTS-3;
even the power button stopped responding until physical power was disconnected.
The failing hash was
`0f18f7a082959b5da20de7ef043ea5268ab24705177abd7d5713b67e1e1cd1d6`.
No KORG Kontrol Editor diagnostics were available.

Inspection found that the failing artifact was tagged FPv5/FP-D16 for ARMv8,
because LLVM upgrades `-C target-cpu=cortex-m7`, while Korg's SDK explicitly
compiles for `-mfpu=fpv4-sp-d16`. Integer-only hardware fixtures could not expose
this mismatch. The build policy now leaves the Rust target at its
`thumbv7em-none-eabihf` VFPv4-D16 baseline and rejects any post-link artifact
without `Tag_FP_arch: VFPv4-D16`. The corrected Smooth Echo candidate
`8f5742e648e3595a7fbf90d9fc6da3bb6d0f2e2707535ade168a48d4845a1d39`
also fully locked the device when selected, ruling out the FP architecture tag
as the sole cause.

The next diagnostic artifact is `target/nts3/smooth_echo_init_probe.nts3unit`
(SHA-256 `27eef9cb37aadebe75ec353e1fe74d850cfe7c9a8ee36181b1ae1a40b3187f2f`).
It performs the exact FunDSP construction, two delay allocations, sample-rate
initialization, and reset path with the same 176-byte plugin / 44-byte parameters
/ 256-byte runtime layout, but its render callback is plain pass-through. It also
hard-locked the device, proving the fault occurs before the original per-sample
delay/filter processing.

The arena-only probe
`64771eeb06fe66d40f992d0137e6388bc03504692ae2ac004ac50f40dc85ffd0`
and plain two-Vec probe
`dcdd267d5f01927626b7cad7878b56dec37f760af6fec45999ff0fee9000b87f`
both load, select, and pass audio correctly. This clears the 1,100,000-byte Korg
arena request, framework lifecycle, two arena-backed allocations totaling
1,048,576 bytes, full SDRAM payload writes, and reset-time clearing.

The Params (`9d2abeaa...`), Filter (`6365fbb1...`), and Tap (`94f0887a...`)
probes all load, select, and pass audio correctly. Every component and every
pair of large memory operations therefore works in isolation; only the combined
runtime construction failed.

Disassembly identified the material combined difference: returning a complete
`Runtime<EchoPlug>` temporary made `unit_init` reserve 592 stack bytes plus 48
bytes of saved registers, a 640-byte callback footprint before nested calls.
The working Tap probe reserved 280 bytes plus saves, Filter 144 bytes plus saves,
and Vec 92 bytes plus saves. The framework now initializes parameter, plugin,
and context fields directly in final `MaybeUninit<Runtime<P>>` storage, with
partial-initialization cleanup on plugin error. The corrected Smooth Echo
`unit_init` reserves 144 bytes plus 48 bytes of saves (192 bytes), eliminating
448 bytes from its own callback stack. Focused state tests verify that only
fully initialized values are published and failed construction releases the
arena without double-drop.

A lower-risk confirmation build is
`target/nts3/smooth_echo_init_probe.nts3unit`, SHA-256
`2b3aca6dff43ad63c53c256707b6685ad4f8d1586d9a8b2c6114c544fe665be0`.
It contains the same combined initialization and reduced stack frame but still
renders pass-through. Test this first. The resulting full-DSP candidate is
`target/nts3/smooth_echo.nts3unit`, SHA-256
`3c3c42cc3d1ebcd30528f714947dd7972e42b7fae17dc6c7bb3c6a9add21bd90`.
All host DSP, target, allocation, and inspector checks pass; hardware selection
is pending.

## Summary and design decisions

- Kept `example/smooth-echo-nts3-plug/src/lib.rs` unchanged. The public source
  still contains only parameter/DSP/plugin code and no per-plugin ABI, linker,
  panic, or allocator boilerplate.
- Retained the exact locked dependency `fundsp = "=0.21.0"` with
  `default-features = false`; the final report records FunDSP 0.21.0.
- Added one callback-level host integration test which drives the generated ABI
  and exact plugin DSP through repeated full lifecycles. It covers:
  - silence and finite near-zero output;
  - a 100 ms impulse repeat at 48 kHz (measured onset within two samples of
    sample 4,800);
  - full-range time/feedback sweeps and smoothing;
  - feedback 0, 0.5, exact 0.99 boundary, >0.99 freeze, and 1.0 freeze;
  - reset clearing delay/filter state while retaining parameter targets;
  - exact in-place versus separate-buffer equivalence within `1e-6`;
  - 128,000 deterministic random stereo frames with finite output and peak
    below the documented test bound;
  - two 524,288-byte initialization allocations and zero allocations during all
    post-init render/reset/parameter callbacks exercised by the test.
- The integration test compares the concrete exported header's complete 256
  descriptor bytes and 64 mapping bytes, including six unused slots. Defaults
  are 500 ms and 0%; TIME is X/exponential and FEEDBACK is Y/linear.
- The framework native arena probe independently confirms two allocations,
  1,048,576 requested/high-water bytes, zero alignment/transient overhead,
  51,424 bytes declaration margin, and zero allocations across 256 sealed
  post-init stress renders/events.
- Existing Touch Probe tests still cover active `Began(0,0)` versus inactive
  `Ended(0,0)` while retaining coordinates. Smooth Echo's checked-in touch
  fields remain independent and do not alter DSP.
- No extra size optimization was required. Fat LTO, one codegen unit,
  `opt-level=z`, panic abort, no forced unwind tables, and GNU section GC yield a
  12,444-byte corrected candidate, comfortably below all 32 KiB limits.
- Corrected a hardware-discovered toolchain flaw: `target-cpu=cortex-m7` selected
  LLVM FPv5/FP-ARMv8 instead of the SDK's VFPv4-D16 baseline. The CLI no longer
  supplies that upgrade and now checks the GNU readelf FP architecture tag.
- Removed the pass-through fixture's historical `crate-type = ["staticlib"]` so
  it uses the same no-boilerplate archive route as all other plugins; this also
  keeps its native probe reliable after a clean build.
- Per the user-edited Task 10 instruction, the NIH-plug tree was ignored and no
  reference harness/golden data was produced.

## Files added or changed

- Added `example/smooth-echo-nts3-plug/tests/dsp.rs`.
- Changed `example/smooth-echo-nts3-plug/Cargo.toml` to add the test-only local
  `nts3-sys` dependency.
- Updated `Cargo.lock` for the test dependency and diagnostic fixture.
- Added `fixtures/smooth-echo-{init,arena,vec,params,filter,tap}-probe/`
  with workspace entries to isolate render, arena reservation, arena-backed
  writes, parameters/libm, filters, and taps on hardware.
- Changed `crates/nts3-cli/src/main.rs` and
  `crates/nts3-cli/tests/fake_tools.rs` to enforce the SDK VFPv4-D16 baseline.
- Changed `crates/nts3/src/runtime.rs` and `runtime_state.rs` to construct the
  concrete runtime directly in final static storage and test publication/error
  cleanup without a full-runtime stack temporary.
- Removed the obsolete staticlib stanza from `fixtures/pass-through/Cargo.toml`.
- Updated `plan/implementation.md`, `plan/agents/STATUS.md`, and this handoff.

The pre-existing user edit to `plan/agents/10-smooth-echo.md`, the untracked NIH
reference tree, and untracked `external/` tree were not modified by this task.

## Commands and results

Docker/public-launcher commands were run from Bash. Cargo commands were run from
PowerShell as requested.

1. Required prerequisite full builds before implementation — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p pass-through --release
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p touch-probe --release
   ```

   Resulting hashes:

   - pass-through: `d598283d80da55127e86b2182504de7f29635a3866352cd70ea08c5887ed21c2`
   - Touch Probe: `9454b216f8bd8bfaf9353ff9cb569b609c6927c9150ed3d20dc3f36360180c9a`

2. Focused Smooth Echo DSP integration test — **PASS**:

   ```powershell
   cargo test -p smooth-echo-nts3-plug --test dsp
   ```

   One comprehensive test passed in both native Windows and the pinned Linux
   container.

3. Full native formatting/test/lint/target suite — **PASS**:

   ```powershell
   cargo fmt --all -- --check
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo check -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf
   cargo clippy -p smooth-echo-nts3-plug --target thumbv7em-none-eabihf -- -D warnings
   ```

4. Final public target check/build/inspection — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./nts3.sh check -p smooth-echo-nts3-plug
   MSYS_NO_PATHCONV=1 ./nts3.sh build -p smooth-echo-nts3-plug --release
   MSYS_NO_PATHCONV=1 ./nts3.sh inspect target/nts3/smooth_echo.nts3unit
   ```

   Inspection status: `PASS`, policy `nts3-framework-v1`, no warnings or
   errors.

5. Complete pinned-container suite — **PASS**:

   ```bash
   MSYS_NO_PATHCONV=1 ./container/run.sh cargo test --locked --workspace
   ```

6. Source hygiene — **PASS**:

   ```bash
   git diff --check
   ```

## Final artifacts, hashes, and reports

Generated files are gitignored under `target/nts3/`:

| File | SHA-256 |
|---|---|
| `smooth_echo.nts3unit` | `3c3c42cc3d1ebcd30528f714947dd7972e42b7fae17dc6c7bb3c6a9add21bd90` |
| `smooth_echo.elf` | `a18702df75f73f23a86c528e7c752a5095ff69d34b2992a764c4562fe201d1b9` |
| `smooth_echo.map` | `e63df2ac72da7d40899d656abb59a5d3fc14bec49aa453b004930254a4571e9d` |
| `smooth_echo.memory.json` | `c0a604ecf3b1471088615d2c53107d0bd467cd7564184e91fa6c43a6a895f0be` |
| `smooth_echo.memory.txt` | `6ed9831d2dfae0e063af3630c54f6564746e08a016c227431e885a09c2ef679a` |
| `smooth_echo.commands.txt` | `e1cfc1b7cce71cbf31dab8c0bc4f05a006e1aef840ec8cd45c37c19aeb7613cc` |

The exact release compile, archive aggregation, GNU final link, strip, report,
and native-probe commands are in
`target/nts3/smooth_echo.commands.txt`.

## Measured code and memory facts

- Stripped artifact: 12,536 / 32,768 bytes; margin 20,232.
- PT_LOAD memory union: 6,990 / 32,768 bytes; margin 25,778.
- Static writable RAM: 380 / 32,768 bytes; margin 32,388.
- `.text`: 4,982 bytes; `.data`: 0; `.bss`: 268; `.got`: 12.
- `unit_init` own stack footprint from disassembly: 192 bytes, down from 640;
  nested-call worst-case remains unmeasured.
- Plugin / Parameters / Runtime native sizes: 176 / 44 / 256 bytes.
- Declared SDRAM: 1,100,000 bytes.
- Measured SDRAM: two allocations, 1,048,576 payload bytes, zero alignment
  padding, high-water 1,048,576, declaration margin 51,424 bytes.
- Post-initialization probe: 256 renders/events, zero new allocations.
- ELF: ELF32 little-endian ARM ET_DYN, System V, EABI5 hard-float, one 376-byte
  header, one 12-byte resource record, all unique Thumb callbacks, no rejected
  symbol/PLT/relocation/debug/unwind condition.
- Worst-case stack and NTS-3 real-time CPU remain unknown, as stated in reports.

## Hardware blocker

The original FPv5 Smooth Echo, corrected VFPv4-D16 Smooth Echo, and exact
pass-through-rendering FunDSP initialization probe all hard-faulted immediately
when selected. All six narrower component probes work, isolating the combined
initialization stack footprint. Test the in-place-construction full Smooth Echo
candidate next. After selection succeeds, the final hardware checks remain:

- X range/curve and Y feedback behavior; defaults 500 ms / 0%;
- rapid X smoothing;
- near-unity freeze, release, reset, suspend, and resume;
- sustained stereo full-scale and silence for clipping/noise/dropouts/channels;
- four simultaneous copies with different settings;
- unload/reload and arena return;
- firmware version and observed CPU/dropout behavior.

No software or ELF test can convert this to a hardware pass. Stack and CPU are
also not statically proven.

## Next step

Load `target/nts3/smooth_echo.nts3unit` with the hash above and report the
hardware matrix, or explicitly waive hardware validation. Once resolved, mark
Task 10 DONE and Task 11 may start with:

```bash
MSYS_NO_PATHCONV=1 ./nts3.sh build -p pass-through --release
MSYS_NO_PATHCONV=1 ./nts3.sh build -p touch-probe --release
MSYS_NO_PATHCONV=1 ./nts3.sh build -p smooth-echo-nts3-plug --release
```

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

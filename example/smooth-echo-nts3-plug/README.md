# Smooth Echo — NTS-3 framework example

This is the target authoring experience for the framework specified in
[`../../plan/`](../../plan/). It is a rewrite of
[`../smooth-echo-nih-plug`](../smooth-echo-nih-plug) and intentionally does not
depend on NIH-plug.

The example is an **API contract for the implementation agent**. It will compile
once the planned `crates/nts3`, `crates/nts3-sys`, `crates/nts3-macros`, and build
tooling have been implemented.

## Intended commands

Run from the repository root in Bash (WSL2 on Windows):

```bash
./nts3.sh check -p smooth-echo-nts3-plug
./nts3.sh build -p smooth-echo-nts3-plug --release
./nts3.sh inspect target/nts3/smooth_echo.nts3unit
./container/run.sh cargo test --workspace
```

The public build interface is the Bash launcher; no host Cargo installation or
`cargo nts3` command is required.

The build command should produce both:

- `target/nts3/smooth_echo.nts3unit`
- `target/nts3/smooth_echo.memory.json`

The memory report must separate stripped ELF/load size, static `.data`/`.bss`,
declared SDRAM arena size, measured initialization high-water usage, and any
quantities that cannot be proven statically (notably worst-case stack usage).

## Parameter smoothing

`SmoothedParameter` is planned as a compact `no_std` adaptation of
[wrl/baseplug's MIT-licensed smoother](https://github.com/wrl/baseplug/blob/9ab965bb8ee4c6dffe91c8c78ff944e6d4a49c74/src/smooth.rs).
It uses Baseplug's one-pole exponential coefficients and `1e-5` settling
threshold, exposed here through per-sample `next_plain()` rather than a fixed
block output array. Consequently `smoothing_ms = 100.0` is an exponential time
constant, not a linear ramp that finishes after exactly 100 ms.

## Memory notes

FunDSP is built without `std`, but its two `Tap<U1>` delay lines allocate. At
48 kHz and a two-second maximum delay, each delay buffer rounds up to 131,072
`f32` samples (524,288 bytes), or 1,048,576 bytes total. The plugin reserves a
1,100,000-byte SDRAM arena, leaving roughly 51 KiB for alignment and allocator
overhead. The build-time initialization probe must verify this assumption.

Rust scalar fields, parameter state, and FunDSP's small node structs live in the
unit's static RAM image and should be only hundreds of bytes. They do not carry
per-variable runtime metadata. The build must nevertheless fail if either the
NTS-3 static RAM/load limit or the SDRAM budget is exceeded.

## Touch behavior

`TouchEvent::is_active()` derives activity from the SDK phase, independently of
coordinates. This example stores activity and normalized XY separately to show
that `(0, 0, touched)` and `(0, 0, released)` are distinct states. The DSP does
not otherwise change the original Smooth Echo behavior.

## IDs

The example IDs are non-reserved placeholders. Replace the developer ID with a
unique/registered value before distributing the unit.

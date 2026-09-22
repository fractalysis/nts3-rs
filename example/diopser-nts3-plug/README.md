# Diopser for NTS-3

A render-time-allocation-free, `no_std` NTS-3 port of the NIH-plug
[Diopser](../../external/nih-plug/plugins/diopser/) phase-rotation effect.
There is no plugin UI: the NTS-3 X axis controls frequency and Y controls
resonance.

## Fixed settings

The other Diopser controls are compile-time constants:

- filter stages: **100** (the literal original default of zero stages would be
  a pass-through);
- bypass: off;
- spread: 0 octaves;
- spread style: octaves;
- automation precision: maximum/one sample;
- hidden “very important” setting: on.

Frequency and resonance use the framework's `SmoothedParameter` with 100 ms
one-pole time constants, matching the original smoothing duration. Both NTS-3
pad mappings use exponential curves. Resonance spans Q=0.01..30.0 with a
default of Q=0.5. The effect is fully wet; the NTS-3 effect slot provides
normal effect enable/bypass behavior.

## NTS-3 constraints

The implementation allocates one 1,600-byte stage-state buffer from external
SDRAM during initialization, keeping it out of the 32 KiB static SRAM image and
out of the initialization stack frame. Rendering performs no allocations and
uses only small per-sample stack locals. Since spread is fixed at zero, all
stages share one coefficient set.

The SDK identifies the processor as an ARM Cortex-M7 (STM32H725) and builds for
`-mfpu=fpv4-sp-d16`. Cortex-M7 has scalar floating-point and packed integer DSP
instructions, but not NEON/Advanced SIMD, so this port replaces Diopser's
`f32x2` implementation with scalar stereo processing.

The declared 20,000-byte SDRAM arena contains that single 1,600-byte buffer and
retains the framework build policy's required 16 KiB safety margin. The
developer and unit IDs are example placeholders and must be replaced before
distribution.

## Build

From the repository root:

```bash
./nts3.sh check -p diopser-nts3-plug
./nts3.sh build -p diopser-nts3-plug --release
```

Diopser is licensed under GPL-3.0-or-later. This port retains the original
copyright and license.

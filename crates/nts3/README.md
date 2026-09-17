# `nts3`

Target-safe framework core for Korg NTS-3 `genericfx` units.

`nts3::prelude` exposes the allocation-free runtime author API:

- `Nts3Plugin` lifecycle, processing, touch, and tempo hooks;
- `InitContext` with copied sample-rate, maximum-frame, and touch-area values;
- `StereoBuffer`, whose iterator supports separate and exact in-place stereo
  buffers without exposing aliased references;
- typed `TouchPhase`/`TouchEvent` values with raw, clamped, and normalized
  positions independent of active state.

The runtime validates the SDK descriptor before state construction, owns plugin
and parameter state, refreshes optional raw input for each render, bounds frame
counts, and dispatches all lifecycle hooks. The parameter contract is a sealed
temporary placeholder until the parameter engine and derive are added.

The allocator reserves exactly one Korg SDRAM block and suballocates it with
checked 32-bit aligned bump arithmetic while a plugin is being constructed. The
arena is sealed before readiness. Individual deallocation is intentionally a
no-op: plugin state is dropped first during teardown, then the untouched pointer
returned by `sdram_alloc` is passed once to `sdram_free`.

On target, allocation before activation, after sealing, or beyond the declared
budget enters a non-formatting, non-unwinding fault loop. The generated plugin
adapter will invoke the hidden runtime-glue macro once in the final plugin crate
to install this allocator and panic policy.

Host code can use `nts3::host::HostArena` with `FrameworkAllocator` as its global
allocator to measure construction allocations with the same bump accounting.
Values allocated in a probe arena must not escape the probe closure. See
`examples/allocating_probe.rs`.

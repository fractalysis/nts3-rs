# `nts3`

Target-safe framework core for Korg NTS-3 `genericfx` units.

The current foundation reserves exactly one Korg SDRAM block and suballocates it
with checked 32-bit aligned bump arithmetic while a plugin is being constructed.
The arena is sealed before readiness. Individual deallocation is intentionally
a no-op: plugin state is dropped first during teardown, then the untouched
pointer returned by `sdram_alloc` is passed once to `sdram_free`.

On target, allocation before activation, after sealing, or beyond the declared
budget enters a non-formatting, non-unwinding fault loop. The generated plugin
adapter will invoke the hidden runtime-glue macro once in the final plugin crate
to install this allocator and panic policy.

Host code can use `nts3::host::HostArena` with `FrameworkAllocator` as its global
allocator to measure construction allocations with the same bump accounting.
Values allocated in a probe arena must not escape the probe closure. See
`examples/allocating_probe.rs`.

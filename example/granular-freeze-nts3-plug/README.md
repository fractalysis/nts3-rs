# Granular Freeze

An NTS-3 granular effect with a firmware-1.1-compatible capture path. Untouched audio is passed through bit-for-bit. Touching the pad starts a fresh live capture and crossfades to grains immediately. The write head freezes as soon as the grain/chaos-dependent capture span is available. Releasing crossfades back to live audio.

NTS-3 firmware 1.1 gates the ordinary render input while the effect is untouched, and its `get_raw_input` hook is not usable. Consequently this firmware cannot provide audio from before touch. At zero chaos the fallback records exactly one grain, freezes immediately, and repeatedly grains that first captured segment. Increasing chaos extends the capture toward the two-second maximum so that random starts have a wider range.

## Controls

- **X / GRAIN LENGTH:** 50–1000 ms with the NTS-3 exponential mapping.
- **Y / CHAOS:** widens the uniformly random start range from the beginning of the touch capture toward the newest complete segment.
- **Touch:** begins capture and granular playback. Capture freezes after `grain length + chaos × (2 seconds - grain length)`.

At zero chaos every new grain starts from the audio captured immediately after the pad was pressed, and the write head stops once one complete grain has been recorded. For example, 50 ms and zero chaos monitors live input for the initial 50 ms, then starts stable grains from that fixed segment. Partial grains are never synthesized while capture is filling. Increasing chaos both lengthens the capture and allows starts progressively later in it, reaching a full two-second capture at maximum chaos. Stereo channels always share a grain start.

## Compile-time configuration

The constants are near the top of `src/lib.rs`:

- `BUFFER_SECONDS = 2`
- `MAX_GRAIN_SECONDS = 1.0`
- `GRAIN_AMOUNT = 2.0`
- `CROSSFADE_MILLISECONDS = 5.0`

`GRAIN_AMOUNT` controls density through `hop = grain_length / GRAIN_AMOUNT`. If `MAX_GRAIN_SECONDS` changes, keep the `GRAIN LENGTH` parameter's `max` and `mapping_max` millisecond values in sync. Values at or above `BUFFER_SECONDS` are rejected so maximum-length grains still leave room for chaos once capture is complete.

The grain window is isolated in `grain_window()`. It uses `(1 - x²)³`, a compact polynomial approximation to `exp(-3x²)` around the peak, with exact zero endpoints and no exponential calls. The fixed unity output gain is peak-safe for the two steady half-overlapped windows; unlike mean normalization, it does not intermittently boost unrelated random grains above the input peak.

## Resource notes

The stereo history is one 768,000-byte `Vec<[f32; 2]>` allocated during initialization in SDRAM. Rendering uses three fixed grain slots (two steady voices plus one transition voice), performs no allocation, uses integer ring reads without interpolation, and computes the window with multiplies only. The unit declares 820,000 SDRAM bytes, leaving margin over the measured history allocation.

Build from the repository root:

```bash
./nts3.sh check -p granular-freeze-nts3-plug
./nts3.sh build -p granular-freeze-nts3-plug --release
```

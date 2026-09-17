# Touch probe fixture

This allocation-free DSP fixture makes NTS-3 touch lifecycle state observable:

- left-channel gain is `0.25` (began), `0.5` (moved), `1.0` (stationary),
  `0.0` (ended), or `-0.25` (cancelled);
- right-channel gain follows normalized X only while touch is active.

Consequently a began event at `(0, 0)` remains audible on the left while an
ended event at the same coordinates is silent. Host tests cover every phase.
Task 07 will add generated ABI exports and a unit header; until then this is a
host/target-checkable DSP fixture with a private temporary parameter stub.

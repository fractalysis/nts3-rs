# NTS-3 ABI conformance probe

`probe.c` is compiled directly against the checked-in Korg NTS-3 SDK headers.
It reports every shared structure's size, alignment, and field offsets; all raw
constants exposed by `nts3-sys`; callback signature compatibility; and the full
serialized 376-byte SDK dummy genericfx header. The Rust example emits the same
ordered report and `compare.sh` requires byte-for-byte equality.

Run through the pinned Docker image (from Bash):

```bash
./container/run.sh ./tests/abi/compare.sh
```

Reports are generated under `target/abi/`.

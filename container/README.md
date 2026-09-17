# Reproducible NTS-3 build container

The image derives from the locally verified Korg SDK environment by immutable
manifest digest and installs the checksum-verified Rust 1.98.1 host distribution
plus `thumbv7em-none-eabihf` standard library. It also installs pinned Ubuntu
native GCC/libc development packages so the SDK-header ABI probe can execute in
the container. The base can be changed only via an explicit
`NTS3_BASE_IMAGE` override.

```bash
./container/run.sh build-image
./container/run.sh ./fixtures/pass-through/build.sh
```

Run these scripts from a supported Bash environment. On Windows, use WSL2 with
Docker integration. Generated files are written to `target/nts3/`. The Rust build pins LLVM's
`-mergefunc-use-aliases=0`: real NTS-3 testing proved that firmware silently
rejects required callbacks when identical exported functions share an alias
address.

## Verified tools

Base image:
`xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba`
(Korg environment label `0.5.12`, Ubuntu 20.04.6).

Actual binaries in that image (the SDK README's 10.3 label does not match):

- `arm-none-eabi-gcc`: 9.2.1 20191025, Ubuntu package 15:9-2019-q4
- GNU `ld`, `ar`, `readelf`, `objdump`, and `strip`: 2.34
- Rust/Cargo: 1.98.1
- native GCC (ABI probe): Ubuntu GCC 9 package
- target: `thumbv7em-none-eabihf`

To inspect the environment:

```bash
./container/run.sh /bin/bash -lc \
  'rustc --version --verbose; cargo --version; arm-none-eabi-gcc --version; arm-none-eabi-ld --version'
```

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK="${ROOT}/external/logue-sdk/platform/nts-3_kaoss"
MANIFEST="${ROOT}/fixtures/pass-through/Cargo.toml"
OUT="${ROOT}/target/nts3/pass-through"
ARCHIVE="${ROOT}/target/thumbv7em-none-eabihf/release/libpass_through.a"
ELF="${OUT}/pass_through.elf"
UNIT="${OUT}/pass_through.nts3unit"
MAP="${OUT}/pass_through.map"

mkdir -p "${OUT}"
rm -f "${ELF}" "${UNIT}" "${MAP}"

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"
export CARGO_INCREMENTAL=0
# NTS-3 firmware rejects a unit when LLVM represents multiple required exports
# as aliases at one address. Keep MergeFunctions optimization, but require a
# distinct body/symbol address rather than an alias for each exported callback.
export RUSTFLAGS="-C target-cpu=cortex-m7 -C relocation-model=pic -C panic=abort -C force-unwind-tables=no -C llvm-args=-mergefunc-use-aliases=0 --remap-path-prefix=${ROOT}=<WORKSPACE>"

cargo build \
    --locked \
    --manifest-path "${MANIFEST}" \
    --target thumbv7em-none-eabihf \
    --release

# This is the complete final-link command. Every ABI export is an explicit
# archive root, so archive extraction and --gc-sections cannot discard it.
arm-none-eabi-gcc \
    "${ARCHIVE}" \
    -mcpu=cortex-m7 \
    -mthumb \
    -mno-thumb-interwork \
    -mlittle-endian \
    -mfloat-abi=hard \
    -mfpu=fpv4-sp-d16 \
    -nostartfiles \
    -shared \
    --entry=0 \
    -specs=nano.specs \
    -specs=nosys.specs \
    -Wl,-z,max-page-size=128 \
    -Wl,--gc-sections \
    -Wl,--no-warn-mismatch \
    -Wl,--library-path="${SDK}/ld" \
    -Wl,--script="${SDK}/ld/unit.ld" \
    -Wl,-Map="${MAP}",--cref \
    -Wl,--undefined=unit_header \
    -Wl,--undefined=unit_init \
    -Wl,--undefined=unit_teardown \
    -Wl,--undefined=unit_reset \
    -Wl,--undefined=unit_resume \
    -Wl,--undefined=unit_suspend \
    -Wl,--undefined=unit_render \
    -Wl,--undefined=unit_get_param_value \
    -Wl,--undefined=unit_get_param_str_value \
    -Wl,--undefined=unit_set_param_value \
    -Wl,--undefined=unit_set_tempo \
    -Wl,--undefined=unit_tempo_4ppqn_tick \
    -Wl,--undefined=unit_touch_event \
    -lc \
    -lm \
    -lgcc \
    -o "${ELF}"

cp "${ELF}" "${UNIT}"
arm-none-eabi-strip "${UNIT}"

arm-none-eabi-readelf -hAWSlrds "${UNIT}" > "${OUT}/pass_through.readelf.txt"
arm-none-eabi-objdump -dr "${UNIT}" > "${OUT}/pass_through.objdump.txt"
arm-none-eabi-nm -D "${UNIT}" > "${OUT}/pass_through.nm.txt"
arm-none-eabi-size -A "${UNIT}" > "${OUT}/pass_through.size.txt"
sha256sum "${UNIT}" > "${OUT}/pass_through.sha256"

printf 'Built %s\n' "${UNIT}"
cat "${OUT}/pass_through.sha256"

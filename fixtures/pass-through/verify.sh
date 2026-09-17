#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${ROOT}/target/nts3/pass-through"
C_PROJECT="${ROOT}/external/logue-sdk/platform/nts-3_kaoss/dummy-genericfx"
C_OUT="${OUT}/c-dummy"
GOLDEN="${ROOT}/tests/golden"
RUST_UNIT="${OUT}/pass_through.nts3unit"
CALLBACKS=(
    unit_init unit_teardown unit_reset unit_resume unit_suspend unit_render
    unit_get_param_value unit_get_param_str_value unit_set_param_value
    unit_set_tempo unit_tempo_4ppqn_tick unit_touch_event
)
REQUIRED=(unit_header nts3_resources "${CALLBACKS[@]}")

mkdir -p "${C_OUT}" "${GOLDEN}"

"${ROOT}/fixtures/pass-through/build.sh"
first_hash="$(sha256sum "${RUST_UNIT}" | awk '{print $1}')"

# A package clean makes the second archive independent of Cargo's prior output.
cargo clean --manifest-path "${ROOT}/fixtures/pass-through/Cargo.toml"
"${ROOT}/fixtures/pass-through/build.sh"
second_hash="$(sha256sum "${RUST_UNIT}" | awk '{print $1}')"
if [[ "${first_hash}" != "${second_hash}" ]]; then
    echo "non-reproducible pass-through unit: ${first_hash} != ${second_hash}" >&2
    exit 1
fi

# Build the unmodified official dummy with the actual GNU tools from the image.
mkdir -p "${C_OUT}" "${GOLDEN}"
make -C "${C_PROJECT}" clean
make -C "${C_PROJECT}" GCC_BIN_PATH=/usr/bin
cp "${C_PROJECT}/build/dummy_genericfx.elf" "${C_OUT}/dummy_genericfx.elf"
cp "${C_PROJECT}/build/dummy_genericfx.map" "${C_OUT}/dummy_genericfx.map"
cp "${C_OUT}/dummy_genericfx.elf" "${C_OUT}/dummy_genericfx.nts3unit"
arm-none-eabi-strip "${C_OUT}/dummy_genericfx.nts3unit"

header="$(arm-none-eabi-readelf -h "${RUST_UNIT}")"
grep -Fq 'Class:                             ELF32' <<<"${header}"
grep -Fq "Data:                              2's complement, little endian" <<<"${header}"
grep -Fq 'OS/ABI:                            UNIX - System V' <<<"${header}"
grep -Fq 'Type:                              DYN (Shared object file)' <<<"${header}"
grep -Fq 'Machine:                           ARM' <<<"${header}"
grep -Fq 'Version5 EABI, hard-float ABI' <<<"${header}"

section_count="$(arm-none-eabi-readelf -SW "${RUST_UNIT}" | grep -c ' \.unit_header ' || true)"
[[ "${section_count}" == 1 ]]
resource_section_count="$(arm-none-eabi-readelf -SW "${RUST_UNIT}" | grep -c ' \.nts3_resources ' || true)"
[[ "${resource_section_count}" == 1 ]]
arm-none-eabi-objcopy --dump-section ".nts3_resources=${OUT}/nts3_resources.bin" "${RUST_UNIT}"
printf 'N3RS\001\000\014\000\001\000\000\000' > "${OUT}/nts3_resources.expected.bin"
cmp "${OUT}/nts3_resources.expected.bin" "${OUT}/nts3_resources.bin"

dyn_symbols="$(arm-none-eabi-readelf --dyn-syms -W "${RUST_UNIT}")"
for symbol in "${REQUIRED[@]}"; do
    if ! grep -Eq "[[:space:]]${symbol}$" <<<"${dyn_symbols}"; then
        echo "missing dynamic symbol: ${symbol}" >&2
        exit 1
    fi
done

# Real-device testing proved that the NTS-3 silently rejects an ELF when LLVM
# emits two required callbacks as aliases at one address. Each callback must
# also carry the low Thumb bit in its dynamic symbol value.
declare -A callback_at_address=()
for symbol in "${CALLBACKS[@]}"; do
    address="$(awk -v symbol="${symbol}" '$8 == symbol {print $2; exit}' <<<"${dyn_symbols}")"
    if (( (16#${address} & 1) == 0 )); then
        echo "callback does not have the Thumb bit set: ${symbol}=${address}" >&2
        exit 1
    fi
    if [[ -n "${callback_at_address[${address}]:-}" ]]; then
        echo "callbacks alias one address (rejected by NTS-3): ${callback_at_address[${address}]} and ${symbol}=${address}" >&2
        exit 1
    fi
    callback_at_address[${address}]="${symbol}"
done

undefined="$(arm-none-eabi-readelf --dyn-syms -W "${RUST_UNIT}" | awk '$7 == "UND" && NF >= 8 {print}')"
if [[ -n "${undefined}" ]]; then
    echo "unexpected undefined dynamic symbols:" >&2
    echo "${undefined}" >&2
    exit 1
fi

relocations="$(arm-none-eabi-readelf -rW "${RUST_UNIT}")"
if grep -Eq 'R_ARM_(JUMP_SLOT|CALL|THM_CALL)' <<<"${relocations}"; then
    echo "unexpected call/PLT relocation in Rust unit" >&2
    echo "${relocations}" >&2
    exit 1
fi
if arm-none-eabi-readelf -SW "${RUST_UNIT}" | grep -Eq ' \.plt([[:space:]]|\.)'; then
    echo "unexpected PLT section in Rust unit" >&2
    exit 1
fi

artifact_bytes="$(wc -c < "${RUST_UNIT}")"
if (( artifact_bytes > 32768 )); then
    echo "artifact exceeds 32 KiB: ${artifact_bytes}" >&2
    exit 1
fi

rw_bytes=0
while read -r mem_size; do
    [[ -z "${mem_size}" ]] && continue
    rw_bytes=$((rw_bytes + 16#${mem_size#0x}))
done < <(arm-none-eabi-readelf -lW "${RUST_UNIT}" | awk '$1 == "LOAD" && $0 ~ / RW / {print $6}')
if (( rw_bytes > 32768 )); then
    echo "static writable load exceeds 32 KiB: ${rw_bytes}" >&2
    exit 1
fi

report() {
    local input="$1"
    local stem="$2"
    {
        echo '### ELF, ABI attributes, sections, segments, relocations, symbols, dynamic table'
        arm-none-eabi-readelf -hAWSlrds "${input}"
        echo
        echo '### Dynamic symbols'
        arm-none-eabi-nm -D "${input}"
        echo
        echo '### Section sizes'
        arm-none-eabi-size -A "${input}"
    } | sed -e "s#${ROOT}#<WORKSPACE>#g" -e 's/[[:space:]]\+$//' \
        > "${GOLDEN}/${stem}.readelf.txt"
    arm-none-eabi-objdump -dr "${input}" \
        | sed -e "s#${ROOT}#<WORKSPACE>#g" -e 's/[[:space:]]\+$//' \
        > "${GOLDEN}/${stem}.objdump.txt"
}

report "${RUST_UNIT}" pass-through
report "${C_OUT}/dummy_genericfx.nts3unit" c-dummy
sed -e "s#${ROOT}#<WORKSPACE>#g" -e 's/[[:space:]]\+$//' \
    "${OUT}/pass_through.map" > "${GOLDEN}/pass-through.map.txt"
sed -e "s#${ROOT}#<WORKSPACE>#g" -e 's/[[:space:]]\+$//' \
    "${C_OUT}/dummy_genericfx.map" > "${GOLDEN}/c-dummy.map.txt"

{
    echo 'NTS-3 C dummy / Rust pass-through comparison'
    echo '============================================'
    echo
    echo "Reproducible Rust SHA-256: ${second_hash}"
    echo "Rust artifact bytes: ${artifact_bytes}"
    echo "Rust writable PT_LOAD memory bytes: ${rw_bytes}"
    echo
    echo 'GNU size (stripped artifacts):'
    arm-none-eabi-size "${C_OUT}/dummy_genericfx.nts3unit" "${RUST_UNIT}"
    echo
    echo 'Relocations:'
    echo '-- C dummy --'
    arm-none-eabi-readelf -rW "${C_OUT}/dummy_genericfx.nts3unit"
    echo '-- Rust pass-through --'
    arm-none-eabi-readelf -rW "${RUST_UNIT}"
    echo
    echo 'PLT sections:'
    echo '-- C dummy --'
    arm-none-eabi-readelf -SW "${C_OUT}/dummy_genericfx.nts3unit" | grep -E ' \.([a-z.]*plt|rel\.plt) ' || true
    echo '-- Rust pass-through --'
    arm-none-eabi-readelf -SW "${RUST_UNIT}" | grep -E ' \.([a-z.]*plt|rel\.plt) ' || echo '(none)'
    echo
    echo 'Result: Rust has no undefined dynamic symbols, call relocations, or PLT.'
    echo 'Therefore no ARM-mode PLT is reachable from its Thumb callbacks; --long-plt is unnecessary for this fixture.'
} | sed "s#${ROOT}#<WORKSPACE>#g" > "${GOLDEN}/elf-comparison.txt"

printf 'PASS: reproducible=%s bytes=%s writable_load=%s\n' \
    "${second_hash}" "${artifact_bytes}" "${rw_bytes}"

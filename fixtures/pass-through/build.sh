#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
CLI_OUT="${ROOT}/target/nts3"
OUT="${CLI_OUT}/pass-through"

"${ROOT}/nts3.sh" --local build -p pass-through --release --verbose

mkdir -p "${OUT}"
for suffix in elf nts3unit map readelf.txt nm.txt size.txt commands.txt; do
    cp "${CLI_OUT}/pass_through.${suffix}" "${OUT}/pass_through.${suffix}"
done
arm-none-eabi-objdump -dr "${OUT}/pass_through.nts3unit" >"${OUT}/pass_through.objdump.txt"
sha256sum "${OUT}/pass_through.nts3unit" >"${OUT}/pass_through.sha256"
printf 'Built %s\n' "${OUT}/pass_through.nts3unit"
cat "${OUT}/pass_through.sha256"

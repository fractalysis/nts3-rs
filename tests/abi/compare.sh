#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK_ROOT="${ROOT}/external/logue-sdk/platform/nts-3_kaoss"
SDK="${SDK_ROOT}/common"
OUT="${ROOT}/target/abi"
C_PROBE="${OUT}/c_probe"
C_REPORT="${OUT}/c.txt"
RUST_REPORT="${OUT}/rust.txt"

mkdir -p "${OUT}"

cc \
    -std=gnu11 \
    -Wall \
    -Wextra \
    -Werror \
    -I"${SDK}" \
    "${ROOT}/tests/abi/probe.c" \
    -o "${C_PROBE}"
"${C_PROBE}" > "${C_REPORT}"

cargo run \
    --locked \
    --quiet \
    -p nts3-sys \
    --example abi_probe \
    > "${RUST_REPORT}"

if ! diff -u "${C_REPORT}" "${RUST_REPORT}"; then
    echo "FAIL: Korg C and nts3-sys ABI reports differ" >&2
    exit 1
fi

printf 'PASS: C and Rust ABI reports match (%s records, %s-byte header)\n' \
    "$(wc -l < "${C_REPORT}")" \
    "$(awk -F= '/^HEADER.bytes=/{print length($2) / 2}' "${C_REPORT}")"

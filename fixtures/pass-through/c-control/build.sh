#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
SDK="${ROOT}/external/logue-sdk/platform/nts-3_kaoss"
DUMMY="${SDK}/dummy-genericfx"
COMMON="${SDK}/common"
OUT="${ROOT}/target/nts3/pass-through/c-control"
OBJ="${OUT}/obj"
ELF="${OUT}/c_pass_through.elf"
UNIT="${OUT}/c_pass_through.nts3unit"

mkdir -p "${OBJ}"
COMMON_FLAGS=(
    -mcpu=cortex-m7 -mthumb -mno-thumb-interwork -mlittle-endian
    -mfloat-abi=hard -mfpu=fpv4-sp-d16 -fsingle-precision-constant
    -fcheck-new -fPIC -Os -g
    -DTHUMB_NO_INTERWORKING -DTHUMB_PRESENT -DSTM32H725xE
    -DCORTEX_USE_FPU=TRUE -DARM_MATH_CM7 -D__FPU_PRESENT
    -I"${DUMMY}" -I"${COMMON}" -I"${ROOT}/external/logue-sdk/platform/ext/CMSIS/CMSIS/Include"
)

arm-none-eabi-gcc -c "${COMMON_FLAGS[@]}" -std=c11 -fno-exceptions \
    "${ROOT}/fixtures/pass-through/c-control/header.c" -o "${OBJ}/header.o"
arm-none-eabi-gcc -c "${COMMON_FLAGS[@]}" -std=c11 -fno-exceptions \
    "${COMMON}/_unit_base.c" -o "${OBJ}/_unit_base.o"
arm-none-eabi-g++ -c "${COMMON_FLAGS[@]}" -std=c++11 -fno-use-cxa-atexit \
    -fno-rtti -fno-exceptions -fno-non-call-exceptions \
    "${DUMMY}/unit.cc" -o "${OBJ}/unit.o"

arm-none-eabi-gcc "${OBJ}/header.o" "${OBJ}/_unit_base.o" "${OBJ}/unit.o" \
    -mcpu=cortex-m7 -mthumb -mno-thumb-interwork -mlittle-endian \
    -mfloat-abi=hard -mfpu=fpv4-sp-d16 -fsingle-precision-constant \
    -fcheck-new -Os -g -nostartfiles -shared --entry=0 \
    -specs=nano.specs -specs=nosys.specs \
    -Wl,-z,max-page-size=128 \
    -Wl,-Map="${OUT}/c_pass_through.map",--cref,--no-warn-mismatch \
    -Wl,--library-path="${SDK}/ld",--script="${SDK}/ld/unit.ld" \
    -lc -lm -o "${ELF}"

cp "${ELF}" "${UNIT}"
arm-none-eabi-strip "${UNIT}"
sha256sum "${UNIT}" | tee "${OUT}/c_pass_through.sha256"
arm-none-eabi-readelf -hAWSlrds "${UNIT}" > "${OUT}/c_pass_through.readelf.txt"
printf 'Built hardware-control unit: %s\n' "${UNIT}"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
WORK="${TMP}/workspace with spaces"
BIN="${TMP}/bin"
mkdir -p "${WORK}/external/logue-sdk/platform/nts-3_kaoss/ld" "${BIN}"
cp "${ROOT}/nts3.sh" "${WORK}/nts3.sh"
printf '[workspace]\nmembers = []\n' >"${WORK}/Cargo.toml"
printf 'SECTIONS {}\n' >"${WORK}/external/logue-sdk/platform/nts-3_kaoss/ld/unit.ld"

cat >"${BIN}/docker" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
    "version --format") printf '27.0.0\n' ;;
    "image inspect")
        case "$4" in
            *image-contract*) printf 'nts3-engine-v1\n' ;;
            *rust-version*) printf '1.98.1\n' ;;
            *base-image*) printf 'xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba\n' ;;
            *) printf 'sha256:fake-image\n' ;;
        esac
        ;;
    "run --rm")
        : >"${FAKE_DOCKER_LOG}"
        for argument in "$@"; do printf '%s\n' "$argument" >>"${FAKE_DOCKER_LOG}"; done
        exit "${FAKE_DOCKER_STATUS:-0}"
        ;;
    *) printf 'unexpected fake docker invocation: %s\n' "$*" >&2; exit 90 ;;
esac
FAKE
chmod +x "${BIN}/docker" "${WORK}/nts3.sh"

set +e
PATH="${BIN}:${PATH}" FAKE_DOCKER_LOG="${TMP}/args" FAKE_DOCKER_STATUS=37 \
    "${WORK}/nts3.sh" check -p 'package with spaces'
status=$?
set -e
[[ "${status}" == 37 ]]
grep -Fx -- "${WORK}:/workspace" "${TMP}/args"
grep -Fx -- 'package with spaces' "${TMP}/args"
grep -Fx -- '__engine' "${TMP}/args"
grep -Fx -- 'nts3-container-v1' "${TMP}/args"

PATH="${BIN}:${PATH}" FAKE_DOCKER_LOG="${TMP}/doctor-args" \
    "${WORK}/nts3.sh" doctor >"${TMP}/doctor.out"
grep -Fq 'outer-launcher Docker connection passed' "${TMP}/doctor.out"
grep -Fq 'outer-launcher pinned image passed' "${TMP}/doctor.out"

printf 'PASS: launcher arguments, spaced paths, mounts, and status forwarding\n'

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
IMAGE="${NTS3_DOCKER_IMAGE:-nts3-rs-toolchain:rust-1.98.1}"
BASE_IMAGE="${NTS3_BASE_IMAGE:-xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba}"
IMAGE_CONTRACT="nts3-engine-v1"
INTERNAL_TOKEN="nts3-container-v1"

fail_outer() {
    printf 'nts3: outer-launcher check failed: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat >&2 <<'EOF'
Usage:
  ./nts3.sh [--local] doctor
  ./nts3.sh [--local] check -p <package>
  ./nts3.sh [--local] build -p <package> [--release] [--verbose]
  ./nts3.sh [--local] new <name>

The default backend is the pinned Docker image. --local is an expert mode that
explicitly uses Cargo and GNU ARM tools from the current environment.
EOF
}

if [[ "${1:-}" == "__engine" ]]; then
    [[ "${2:-}" == "${INTERNAL_TOKEN}" ]] || fail_outer "invalid internal invocation contract"
    shift 2
    cd -- "${ROOT}"
    exec cargo run --quiet --locked -p nts3-cli -- "$@"
fi

MODE=docker
if [[ "${1:-}" == "--local" ]]; then
    MODE=local
    shift
fi
if [[ $# -eq 0 ]]; then
    usage
    exit 2
fi
case "$1" in
    doctor|check|build|new|inspect|-h|--help|help) ;;
    *) usage; exit 2 ;;
esac

if [[ "${MODE}" == "local" ]]; then
    printf 'nts3: expert local mode; reproducible container checks are bypassed\n' >&2
    cd -- "${ROOT}"
    exec cargo run --quiet --locked -p nts3-cli -- "$@"
fi

command -v docker >/dev/null 2>&1 || fail_outer "docker was not found"
[[ -f "${ROOT}/Cargo.toml" ]] || fail_outer "workspace root is missing Cargo.toml"
[[ -f "${ROOT}/external/logue-sdk/platform/nts-3_kaoss/ld/unit.ld" ]] || fail_outer "SDK mount source is missing"
mkdir -p -- "${ROOT}/target/nts3" || fail_outer "output directory is not writable"
probe="${ROOT}/target/nts3/.nts3-launcher-write"
printf 'ok' >"${probe}" || fail_outer "output directory is not writable"
rm -f -- "${probe}"

if [[ "$1" == "doctor" ]]; then
    server_version="$(docker version --format '{{.Server.Version}}')" || fail_outer "cannot connect to the Docker service"
    printf 'outer-launcher Docker connection passed: %s\n' "${server_version}"
fi

contract="$(docker image inspect --format '{{ index .Config.Labels "org.nts3.image-contract" }}' "${IMAGE}")" || \
    fail_outer "image ${IMAGE} is unavailable; run ./container/run.sh build-image"
[[ "${contract}" == "${IMAGE_CONTRACT}" ]] || \
    fail_outer "image ${IMAGE} does not satisfy the tooling contract"
rust_label="$(docker image inspect --format '{{ index .Config.Labels "org.nts3.rust-version" }}' "${IMAGE}")" || \
    fail_outer "cannot inspect Rust version label on ${IMAGE}"
[[ "${rust_label}" == "1.98.1" ]] || fail_outer "image ${IMAGE} has the wrong Rust version"
base_label="$(docker image inspect --format '{{ index .Config.Labels "org.nts3.base-image" }}' "${IMAGE}")" || \
    fail_outer "cannot inspect base image label on ${IMAGE}"
[[ "${base_label}" == "${BASE_IMAGE}" ]] || fail_outer "image ${IMAGE} has the wrong SDK base digest"
image_id="$(docker image inspect --format '{{.Id}}' "${IMAGE}")" || fail_outer "cannot inspect image ${IMAGE}"
if [[ "$1" == "doctor" ]]; then
    printf 'outer-launcher pinned image passed: %s (%s)\n' "${IMAGE}" "${image_id}"
    printf 'outer-launcher workspace/SDK/output checks passed\n'
fi

exec docker run --rm --init \
    --volume "${ROOT}:/workspace" \
    --volume "${ROOT}/external/logue-sdk:/workspace/external/logue-sdk:ro" \
    --volume "nts3-cargo-registry:/opt/cargo/registry" \
    --volume "nts3-cargo-git:/opt/cargo/git" \
    --workdir /workspace \
    "${IMAGE}" \
    /workspace/nts3.sh __engine "${INTERNAL_TOKEN}" "$@"

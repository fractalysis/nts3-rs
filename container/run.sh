#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
if command -v cygpath >/dev/null 2>&1; then
    ROOT_DOCKER="$(cygpath -w "${ROOT}")"
    SCRIPT_DIR_DOCKER="$(cygpath -w "${SCRIPT_DIR}")"
else
    ROOT_DOCKER="${ROOT}"
    SCRIPT_DIR_DOCKER="${SCRIPT_DIR}"
fi
IMAGE="${NTS3_DOCKER_IMAGE:-nts3-rs-toolchain:rust-1.85.1}"
BASE_IMAGE="${NTS3_BASE_IMAGE:-xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba}"

usage() {
    cat <<EOF
Usage:
  container/run.sh build-image
  container/run.sh shell
  container/run.sh <command> [args...]

Environment overrides:
  NTS3_DOCKER_IMAGE  Derived image name (default: ${IMAGE})
  NTS3_BASE_IMAGE    Explicit SDK base image reference
EOF
}

case "${1:-shell}" in
    build-image)
        shift
        MSYS_NO_PATHCONV=1 docker build \
            --build-arg "BASE_IMAGE=${BASE_IMAGE}" \
            --tag "${IMAGE}" \
            "${SCRIPT_DIR_DOCKER}" "$@"
        ;;
    shell)
        shift || true
        MSYS_NO_PATHCONV=1 docker run --rm -it \
            --volume "${ROOT_DOCKER}:/workspace" \
            --workdir /workspace \
            "${IMAGE}" /bin/bash "$@"
        ;;
    -h|--help)
        usage
        ;;
    *)
        MSYS_NO_PATHCONV=1 docker run --rm \
            --volume "${ROOT_DOCKER}:/workspace" \
            --workdir /workspace \
            "${IMAGE}" "$@"
        ;;
esac

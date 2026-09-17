#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd -P)"
IMAGE="${NTS3_DOCKER_IMAGE:-nts3-rs-toolchain:rust-1.98.1}"
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
        exec docker build \
            --build-arg "BASE_IMAGE=${BASE_IMAGE}" \
            --tag "${IMAGE}" \
            "${SCRIPT_DIR}" "$@"
        ;;
    shell)
        shift || true
        exec docker run --rm -it \
            --volume "${ROOT}:/workspace" \
            --workdir /workspace \
            "${IMAGE}" /bin/bash "$@"
        ;;
    -h|--help)
        usage
        ;;
    *)
        exec docker run --rm \
            --volume "${ROOT}:/workspace" \
            --workdir /workspace \
            "${IMAGE}" "$@"
        ;;
esac

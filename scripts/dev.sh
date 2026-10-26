#!/usr/bin/env bash
# Runs a command inside the Linux build container, e.g. scripts/dev.sh cargo test
set -euo pipefail

cd "$(dirname "$0")/.."
IMAGE=compas-tools-dev

if ! podman image exists "$IMAGE"; then
  podman build -t "$IMAGE" -f Containerfile .
fi

# Cargo, npm and the target directory live in a named volume so repeated runs
# stay fast and build output stays out of the checkout.
tty=(); [ -t 0 ] && tty=(-t)
# Extra podman options, e.g. DEV_PODMAN_ARGS="-v /some/dir:/dir:ro".
read -r -a extra <<< "${DEV_PODMAN_ARGS:-}"
exec podman run --rm -i "${tty[@]}" \
  --userns=keep-id:uid=1000,gid=1000 \
  -e APPIMAGE_EXTRACT_AND_RUN=1 \
  -e CARGO_HOME=/cache/cargo \
  -e CARGO_TARGET_DIR=/cache/target \
  -e npm_config_cache=/cache/npm \
  -e HOME=/cache/home \
  -v "$PWD":/work:z \
  -v compas-tools-cache:/cache:z \
  -w /work \
  "${extra[@]}" \
  "$IMAGE" "${@:-bash}"

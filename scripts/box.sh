#!/usr/bin/env bash
# Run a command inside the native dev container (Rust + GPUI system deps + Xvfb).
# Usage: scripts/box.sh cargo test -p quiver-theme
# Cache volumes: target/ and the cargo registry survive between runs (a cold GPUI build is ~5 min).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${QUIVER_BOX_IMAGE:-quiver-native-dev}"
SLOT="${QUIVER_BOX_SLOT:-main}"
docker volume inspect quiver-cargo-home >/dev/null 2>&1 || { docker volume create quiver-cargo-home >/dev/null; docker run --rm -v quiver-cargo-home:/h alpine chown "$(id -u):$(id -g)" /h; }
docker volume inspect "quiver-target-$SLOT" >/dev/null 2>&1 || { docker volume create "quiver-target-$SLOT" >/dev/null; docker run --rm -v "quiver-target-$SLOT":/h alpine chown "$(id -u):$(id -g)" /h; }
docker image inspect "$IMAGE" >/dev/null 2>&1 || docker build -t "$IMAGE" "$ROOT/docker/native"
exec docker run --rm \
  -v "$ROOT":/work \
  -v "quiver-target-$SLOT":/work/target \
  -v quiver-cargo-home:/cargo-home \
  -w /work \
  -e CARGO_TERM_COLOR=never \
  --user "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/cargo-home \
  "$IMAGE" "$@"

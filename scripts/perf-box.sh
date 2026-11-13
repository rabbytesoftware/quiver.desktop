#!/usr/bin/env bash
# Run a command inside the native dev container with what the perf tooling adds on top of scripts/box.sh:
# a checkout of quiver.core (QUIVER_CORE_DEV_PATH, default: the sibling checkout), a build volume for the old
# Tauri app / bun / Go caches, and the webkit sandbox switched off (it cannot start under docker's seccomp).
#   scripts/perf-box.sh cargo xtask perf build
#   scripts/perf-box.sh cargo xtask perf run --runs 5
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${QUIVER_BOX_IMAGE:-quiver-native-dev}"
SLOT="${QUIVER_BOX_SLOT:-perf}"
CORE="${QUIVER_CORE_DEV_PATH:-$ROOT/../../../../quiver.core/develop/worktree}"
for v in quiver-cargo-home "quiver-target-$SLOT" quiver-perf-build; do
  docker volume inspect "$v" >/dev/null 2>&1 || { docker volume create "$v" >/dev/null; docker run --rm -v "$v":/h alpine chown "$(id -u):$(id -g)" /h; }
done
docker image inspect "$IMAGE" >/dev/null 2>&1 || docker build -t "$IMAGE" "$ROOT/docker/native"
exec docker run --rm --security-opt seccomp=unconfined --shm-size=1g \
  -v "$ROOT":/work -v "quiver-target-$SLOT":/work/target -v quiver-cargo-home:/cargo-home \
  -v quiver-perf-build:/perf -v "$(cd "$CORE" && pwd)":/core:ro \
  -w /work -e CARGO_TERM_COLOR=never -e PERF_NAME="${PERF_NAME:-}" \
  -e WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-}" \
  -e WEBKIT_FORCE_SANDBOX=0 -e GOCACHE=/perf/go-cache -e GOPATH=/perf/go -e GOFLAGS=-buildvcs=false \
  --user "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/cargo-home \
  "$IMAGE" "$@"

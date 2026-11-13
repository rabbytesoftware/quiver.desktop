#!/usr/bin/env bash
# Native app, two builds:
#   release : the shipped profile (Cargo.toml [profile.release]: LTO, opt-level s, stripped), no bridge. Used for
#             size, RSS, CPU and wake-ups.
#   bridge  : release profile with LTO off + `--features e2e-bridge`, like the e2e box. Used for the first-frame
#             signal and for scripted scrolling. Size/CPU of this build are NOT the shipped numbers.
# Outputs: /perf/native/{release,bridge}/{quiverdesktop,quiver}
set -euo pipefail
cd /work
export CARGO_TARGET_DIR=/work/target
install_to() { # <profile dir> <target subdir>
  mkdir -p "/perf/native/$1"
  install -m 0755 "$2/quiver" "/perf/native/$1/quiverdesktop"
  install -m 0755 /perf/bin/quiver "/perf/native/$1/quiver"
}
if [ "${1:-all}" != bridge ]; then
  QUIVER_DESKTOP_RELEASE_TAG=stable-1.0 cargo build --release -p quiver-app --bins
  install_to release target/release
fi
if [ "${1:-all}" != release ]; then
  CARGO_TARGET_DIR=/work/target/bridge CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  QUIVER_DESKTOP_RELEASE_TAG=stable-1.0 cargo build --release -p quiver-app --bins --features e2e-bridge
  install_to bridge target/bridge/release
fi
ls -l /perf/native/*/

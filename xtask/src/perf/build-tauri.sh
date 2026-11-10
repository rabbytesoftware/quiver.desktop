#!/usr/bin/env bash
# The old Tauri app, as docker/e2e/scenarios/build.sh built it (git show 4be62b1^:...): sources copied into the
# build volume (nothing is written into the checkout), bun for the frontend, `tauri build --no-bundle` so the
# embedded frontend is served (a bare `cargo build --release` would point the webview at localhost:1420).
# Output: /perf/tauri/{quiverdesktop,quiver}
set -euo pipefail
export PATH=/perf/bun/bin:$PATH
if ! command -v bun >/dev/null; then
  mkdir -p /perf/bun-dl /perf/bun/bin
  curl -fsSL -o /perf/bun-dl/bun.zip https://github.com/oven-sh/bun/releases/latest/download/bun-linux-x64.zip
  python3 -I -c "import zipfile;zipfile.ZipFile('/perf/bun-dl/bun.zip').extractall('/perf/bun-dl/x')"
  install -m 0755 /perf/bun-dl/x/*/bun /perf/bun/bin/bun
fi
bun --version
SRC=/perf/tauri-src
mkdir -p "$SRC"
find "$SRC" -mindepth 1 -maxdepth 1 ! -name node_modules ! -name target -exec rm -rf {} +
tar -C /work -cf - --exclude=./target --exclude=./node_modules --exclude=./.git --exclude=./crates \
  --exclude=./docker --exclude=./src-tauri/target --exclude=./dist . | tar -C "$SRC" -xf -
cd "$SRC"
bun install --frozen-lockfile || bun install
TRIPLE=$(rustc -vV | sed -n 's/^host: //p')
mkdir -p src-tauri/binaries
install -m 0755 /perf/bin/quiver "src-tauri/binaries/quiver-$TRIPLE"
CARGO_TARGET_DIR=/perf/tauri-target QUIVER_DESKTOP_RELEASE_TAG=stable-1.0 \
  bun run tauri build --no-bundle --config '{"bundle":{"macOS":{"files":{}}}}'
mkdir -p /perf/tauri
install -m 0755 /perf/tauri-target/release/quiverdesktop /perf/tauri/quiverdesktop
install -m 0755 /perf/bin/quiver /perf/tauri/quiver
ls -l /perf/tauri

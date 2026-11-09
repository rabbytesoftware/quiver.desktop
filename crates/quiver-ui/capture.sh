#!/usr/bin/env bash
# Screenshot the gallery under Xvfb. Run through the container:
#   QUIVER_BOX_SLOT=ui scripts/box.sh bash crates/quiver-ui/capture.sh actions:light actions:dark
# Each argument is page:mode[:open]; PNGs land in docker/native/evidence/gallery-<page>-<mode>[-<open>].png
set -euo pipefail
OUT=docker/native/evidence
BIN=/work/target/debug/examples/gallery
mkdir -p "$OUT"
Xvfb :1 -screen 0 1280x800x24 >/dev/null 2>&1 &
sleep 1
export DISPLAY=:1 ZED_ALLOW_EMULATED_GPU=1 VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
unset WAYLAND_DISPLAY
openbox >/dev/null 2>&1 &
sleep 1
for spec in "$@"; do
  IFS=: read -r page mode open <<<"$spec"
  QUIVER_GALLERY_PAGE="$page" QUIVER_GALLERY_MODE="$mode" QUIVER_GALLERY_OPEN="${open:-}" "$BIN" >/dev/null 2>&1 &
  pid=$!
  sleep "${CAPTURE_WAIT:-12}"
  import -window root "$OUT/gallery-$page-$mode${open:+-$open}.png"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
done

#!/usr/bin/env bash
# Screenshot the app (--demo) under Xvfb, client area only. Run through the container:
#   QUIVER_BOX_SLOT=views scripts/box.sh bash crates/quiver-app/capture.sh home home-scrolled:scroll=800
# Each argument is name[:action...] (actions separated by ':'): scroll=<px>, click=<x>,<y>, hover=<x>,<y>, key=<xdotool key>.
# PNGs (1200x800, the window's client area) land in docker/native/evidence/<name>.png
set -euo pipefail
OUT=docker/native/evidence
BIN=${QUIVER_BIN:-/work/target/debug/quiver}
mkdir -p "$OUT"
Xvfb :1 -screen 0 1600x1000x24 >/dev/null 2>&1 &
sleep 1
export DISPLAY=:1 ZED_ALLOW_EMULATED_GPU=1 VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
unset WAYLAND_DISPLAY
openbox >/dev/null 2>&1 &
sleep 1
"$BIN" --demo >/tmp/quiver-demo.log 2>&1 &
pid=$!
sleep "${CAPTURE_WAIT:-14}"
win=$(xdotool search --name '^Quiver$' | head -1)
info=$(xwininfo -id "$win")
X=$(awk '/Absolute upper-left X/ {print $4}' <<<"$info"); Y=$(awk '/Absolute upper-left Y/ {print $4}' <<<"$info")
WIDTH=$(awk '/Width:/ {print $2}' <<<"$info"); HEIGHT=$(awk '/Height:/ {print $2}' <<<"$info")
for spec in "$@"; do
  IFS=: read -r name rest <<<"$spec"
  IFS=: read -ra actions <<<"$rest"
  for action in "${actions[@]}"; do
    kind=${action%%=*}; arg=${action#*=}
    case $kind in
      scroll) xdotool mousemove $((X + 700)) $((Y + 400)); for _ in $(seq 1 $((arg / 50))); do xdotool click 5; done ;;
      click) IFS=, read -r cx cy <<<"$arg"; xdotool mousemove $((X + cx)) $((Y + cy)) click 1 ;;
      hover) IFS=, read -r cx cy <<<"$arg"; xdotool mousemove $((X + cx)) $((Y + cy)) ;;
      rclick) IFS=, read -r cx cy <<<"$arg"; xdotool mousemove $((X + cx)) $((Y + cy)) click 3 ;;
      key) xdotool key "$arg" ;;
    esac
    sleep 1.5
  done
  mkdir -p "$(dirname "$OUT/$name.png")"
  import -window root -crop "${WIDTH}x${HEIGHT}+${X}+${Y}" +repage "$OUT/$name.png"
done
kill "$pid" 2>/dev/null || true
tail -5 /tmp/quiver-demo.log || true

#!/usr/bin/env bash
# Captures every AFTER-board screen (light, then the screens that differ in dark) into docker/native/evidence/boards-<mode>/.
#   QUIVER_BOX_SLOT=visual scripts/box.sh bash crates/quiver-app/capture_boards.sh [light|dark|both]
# One app launch per screen: QUIVER_DEMO_ROUTE / QUIVER_DEMO_THEME pick the screen (see demo/route.rs).
set -uo pipefail
MODE=${1:-both}
OUT=docker/native/evidence
MC=github.com/rabbyte/minecraft
# name|route|actions (capture.sh syntax)
SCREENS=(
  "home||"
  "home-scrolled||:scroll=800"
  "recommended|recommended|"
  "search|search:server|"
  "library|library|"
  "collections|collections|"
  "collection|collection:github.com/rabbyte/game-servers|"
  "remote|remote|"
  "settings-general|settings|"
  "settings-engine|settings:engine|"
  "arrow-running|arrow:$MC|"
  "arrow-activity|arrow:github.com/rabbyte/caddy|"
  "arrow-nobanner|arrow:github.com/rabbyte/terraria|"
  "arrow-methods|arrow:$MC:methods|"
  "arrow-settings|arrow:$MC:settings|"
  "arrow-app|arrow-app|"
  "arrow-404|arrow:github.com/none/missing|"
  "arrow-error|arrow:error|"
)
run_mode() {
  local mode=$1
  for entry in "${SCREENS[@]}"; do
    IFS='|' read -r name route actions <<<"$entry"
    QUIVER_DEMO_ROUTE=$route QUIVER_DEMO_THEME=$mode CAPTURE_WAIT=${CAPTURE_WAIT:-9} \
      bash crates/quiver-app/capture.sh "boards-$mode/$name$actions" || echo "capture failed: $name ($mode)"
  done
}
[[ $MODE != dark ]] && run_mode light
[[ $MODE != light ]] && run_mode dark

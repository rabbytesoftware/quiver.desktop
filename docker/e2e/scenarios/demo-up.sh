#!/usr/bin/env bash
# Leaves the box as a hands-on demo of arrow apps: the Quiver app running on
# the box's display with its own daemon, quiver.chat installed from its real
# ARROW.md through the GitHub stand-in and running, and three fixture arrows
# (E2E Static App 1 and 2, E2E Echo App) running beside it, with the app
# showing quiver.chat's interface. Run it detached inside a box that is up:
#
#   docker compose -f docker/e2e/docker-compose.yml up -d
#   docker compose -f docker/e2e/docker-compose.yml exec -d e2e bash scenarios/demo-up.sh
#
# then open http://localhost:6080/vnc.html. Progress goes to
# results/demo/demo-up.log. It builds first unless SKIP_BUILD=1 and every
# artifact is already there. It needs quiver.chat mounted at
# /workspace/quiver.chat (see README.md, "Arrow apps").
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin demo "a hands-on arrow apps demo"
exec >>"$SCENARIO_DIR/demo-up.log" 2>&1

case "$(uname -m)" in
	aarch64) CORE_ASSET=quiver-linux-arm64 ;;
	*) CORE_ASSET=quiver-linux-amd64 ;;
esac
APP_DIR="$RUN_DIR/demo-app"
CHAT_NS="github.com/rabbytesoftware/quiver.chat"

[ -f "$CHAT_CHECKOUT/ARROW.md" ] || fail "no quiver.chat checkout at $CHAT_CHECKOUT"

needed=("$BUILD_BIN/quiver-$CORE_V1" "$BUILD_BIN/quiverdesktop-$DESK_V1"
	"$CHAT_DIST/quiver-chat-linux-arm64.tar.gz" "$CHAT_DIST/quiver-chat-linux-amd64.tar.gz")
missing=0
for artifact in "${needed[@]}"; do [ -f "$artifact" ] || missing=1; done
if [ "${SKIP_BUILD:-0}" != "1" ] || [ "$missing" = "1" ]; then
	say "Building from source"
	WDIO_ONLY=1 bash "$HERE/build.sh" || fail "build failed"
fi

say "Starting from a clean home"
kill_everything
pkill -f 'quiver-chat-linux-|server\.py |^sleep 3000$' 2>/dev/null || true
rm -rf "$HOME/.quiver"
mkdir -p "$HOME/.quiver"

say "Standing up the GitHub stand-in"
bash "$E2E_DIR/fixtures/upstream-up.sh" || fail "upstream did not come up"
reset_upstream
publish_core_asset "$CORE_V1"
publish_manifest rabbytesoftware/quiver.core "$CORE_V1" /workspace/build/src/quiver.core/ARROW.md
mark_latest rabbytesoftware/quiver.core "$CORE_V1"
git_tag rabbytesoftware/quiver.core "$CORE_V1"
publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" /workspace/build/src/quiver.desktop/ARROW.md
git_tag rabbytesoftware/quiver.desktop "$DESK_V1"
publish_manifest rabbytesoftware/quiver.chat develop "$CHAT_CHECKOUT/ARROW.md"
publish_release rabbytesoftware/quiver.chat nightly \
	"$CHAT_DIST/quiver-chat-linux-arm64.tar.gz" "$CHAT_DIST/quiver-chat-linux-amd64.tar.gz"
publish_manifest rabbytesoftware/e2e-echo-app develop "$E2E_DIR/fixtures/arrows/e2e-echo-app/ARROW.md"
publish_release rabbytesoftware/e2e-echo-app v1 "$E2E_DIR/fixtures/arrows/e2e-echo-app/server.py"
for n in 1 2; do
	publish_manifest "rabbytesoftware/e2e-static-app-$n" develop "$UPSTREAM_STATE/fixtures/e2e-static-app-$n/ARROW.md"
done

say "Launching Quiver"
mkdir -p "$APP_DIR"
install -m 0755 "$BUILD_BIN/quiverdesktop-$DESK_V1" "$APP_DIR/quiverdesktop"
install -m 0755 "$BUILD_BIN/quiver-$CORE_V1" "$APP_DIR/quiver"
launch_desktop "$APP_DIR/quiverdesktop"
wait_for_desktop_window 90
wait_for_daemon 90

CLI="$HOME/.quiver/self/quiver"
for _ in $(seq 1 120); do [ -x "$CLI" ] && break; sleep 0.5; done
[ -x "$CLI" ] || fail "the daemon never self-installed its CLI at $CLI"

# add_and_run NAMESPACE -- registers the arrow, installs it and starts it with
# the real CLI, and prints its catalog identity.
add_and_run() {
	"$CLI" arrow add "$1" >&2 || fail "quiver arrow add $1"
	local identity
	identity="$(arrow_field "$1" '.data.namespace')"
	"$CLI" install "$identity" >&2 || fail "quiver install $identity"
	"$CLI" run "$identity" --detach >&2 || fail "quiver run $identity"
	printf '%s' "$identity"
}

say "Installing and starting the arrows"
declare -a IDENTITIES=()
for ns in github.com/rabbytesoftware/e2e-static-app-1 github.com/rabbytesoftware/e2e-static-app-2 \
	github.com/rabbytesoftware/e2e-echo-app "$CHAT_NS"; do
	IDENTITIES+=("$(add_and_run "$ns")")
done
for identity in "${IDENTITIES[@]}"; do
	deadline=$(( SECONDS + 60 ))
	until [ "$(arrow_field "$identity" '.data.active_run.surface.ready')" = "true" ]; do
		[ "$SECONDS" -lt "$deadline" ] || fail "$identity never had a ready interface"
		sleep 0.5
	done
	ok "$identity is running with a ready interface"
done

# The sidebar lists the visible arrows sorted by name, and the app's own
# components are hidden by default, so only the four arrows above are rows.
# quiver.chat's row is its rank among their names. Opening a running arrow
# shows its app straight away. Rows start 138 px below the client area's top
# and are 40 px apart.
say "Opening quiver.chat"
declare -a NAMES=()
for identity in "${IDENTITIES[@]}"; do
	NAMES+=("$(arrow_field "$identity" '.data.name')")
done
CHAT_NAME="$(arrow_field "${IDENTITIES[3]}" '.data.name')"
CHAT_ROW="$(printf '%s\n' "${NAMES[@]}" | sort -f | grep -n -x -F -- "$CHAT_NAME" | head -1 | cut -d: -f1)"
[ -n "$CHAT_ROW" ] || fail "quiver.chat's name $CHAT_NAME is not among the sidebar rows"
sleep 3
click_in_app 120 $(( 138 + (CHAT_ROW - 1) * 40 ))
sleep 2
screenshot ready
say "DEMO READY: http://localhost:6080/vnc.html (the box's port)"

#!/usr/bin/env bash
# SCENARIO 2
#
#   "Core installed headless -> user adds quiver.desktop through either CLI or
#    API -> the desktop app should connect automatically and sideload the
#    headless daemon as its own when initiated together."
#
# The claim under test is about EXISTING behaviour, not new code:
# SidecarManager::ensure_running (src-tauri/src/connection/local/sidecar.rs)
# health-probes the default local address before spawning anything, and
# SidecarManager::reap never kills a daemon it did not spawn. Both sides
# independently derive $HOME/.quiver/quiver.sock, so a headless daemon and a
# release build of the app land on the same address by construction.
#
# The proof is a process count that never leaves 1, taken at every point where
# a second daemon could appear -- and a screenshot of the real window that
# came up over the adopted one.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin "2-sideload" \
	"headless core, desktop installed through core's own catalog, app adopts the running daemon"

DESK_ARROW="$DESK_NS@$DESK_V1"
CORE_ARROW="$CORE_NS@$CORE_V1"
INSTALL_PATH="${XDG_DATA_HOME:-$HOME/.local/share}/Quiver/Quiver.AppImage"

# --- preconditions ---------------------------------------------------------

say "Preconditions: a clean machine and one published release of each"
reset_state
reset_upstream
assert_daemon_count 0
assert_desktop_count 0

publish_manifest rabbytesoftware/quiver.core "$CORE_V1" "/workspace/build/src/quiver.core/ARROW.md"
publish_release rabbytesoftware/quiver.core "$CORE_V1" "$BUILD_BIN/quiver-$CORE_V1"
mark_latest rabbytesoftware/quiver.core "$CORE_V1"
git_tag rabbytesoftware/quiver.core "$CORE_V1"

publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" "/workspace/build/src/quiver.desktop/ARROW.md"
cp "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1" "/tmp/$DESKTOP_ASSET"
publish_release rabbytesoftware/quiver.desktop "$DESK_V1" "/tmp/$DESKTOP_ASSET"
mark_latest rabbytesoftware/quiver.desktop "$DESK_V1"
git_tag rabbytesoftware/quiver.desktop "$DESK_V1"

DESK_URL="https://github.com/rabbytesoftware/quiver.desktop/releases/download/$DESK_V1/$DESKTOP_ASSET"
DESK_SUM="$(sha256_of "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1")"

# --- act: core, headless ---------------------------------------------------

say "Installing quiver.core headless"
start_headless_daemon "$BUILD_BIN/quiver-$CORE_V1"
HEADLESS_PID="$(daemon_pid)"
assert_daemon_count 1
info "the headless daemon is pid $HEADLESS_PID"

# The self-arrow has to be Ready before anything can depend on it.
# quiver.desktop's manifest declares a `tools:` edge on
# quiver.core@stable-26.5*, and installOneDep BeginInstalls any dependency
# whose runtime aggregate does not exist yet -- with NIL variables. Before
# requireReferenced, quiver.core's own manifest could not satisfy that, so
# installing quiver.desktop failed on its own dependency unless something had
# walked core's self-arrow to Ready by hand. This call sends no variables
# either, for the same reason.
say "Bootstrapping the core self-arrow so it can be depended on"
api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW")/install" \
	'{"variables":{}}' 202 >/dev/null
wait_for_state "$CORE_ARROW" ready 120

# --- act: add + install quiver.desktop THROUGH CORE ------------------------

say "Adding quiver.desktop through the CLI, exactly as a user would"
# Real `quiver arrow add`, against the real manifest resolver: this reaches
# https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/stable-1.0/ARROW.md
# and parses whatever comes back. No seeding, no manifest injection.
quiver arrow add "$DESK_ARROW" 2>&1 | tee "$SCENARIO_DIR/cli-add.log"
assert_eq "200" "$(api_status GET "/v0/arrow/$(ns_enc "$DESK_ARROW")")" \
	"quiver.desktop is in the catalog after 'quiver arrow add'"
assert_eq "Quiver" "$(arrow_field "$DESK_ARROW" '.data.name')" \
	"the resolved manifest's product name"
assert_daemon_count 1

say "Installing it through the CLI"
# --data is how the CLI passes lifecycle variables. quiver.desktop's manifest
# has the caller resolve the release asset and hand both values in (the same
# contract install.sh implements against the real releases API); this is that
# caller.
quiver install "$DESK_ARROW" --detach \
	--data "QUIVER_RELEASE_ASSET_URL=$DESK_URL" \
	--data "QUIVER_RELEASE_CHECKSUM=$DESK_SUM" 2>&1 | tee "$SCENARIO_DIR/cli-install.log"
wait_for_state "$DESK_ARROW" ready 300

assert_file "$INSTALL_PATH" "the installed Quiver.AppImage"
assert_eq "$DESK_SUM" "$(sha256_of "$INSTALL_PATH")" \
	"the installed file's sha256 matches the published asset"
[ -x "$INSTALL_PATH" ] || fail "the install lifecycle left $INSTALL_PATH non-executable"
ok "the installed artifact carries the executable bit"

# The real fetch really went over the wire, checksum and all.
grep -q '"event": "asset.served"' "$RESULTS/upstream.log" \
	|| fail "the upstream fixture never served the desktop asset"
ok "the upstream fixture served the desktop asset over HTTPS"

assert_daemon_count 1
assert_eq "$HEADLESS_PID" "$(daemon_pid)" "the daemon pid after installing the desktop app"

# --- act: launch the app ---------------------------------------------------

say "Launching the installed desktop app on the real X display"
launch_desktop "$INSTALL_PATH"
wait_for_desktop_window 90
assert_desktop_count 1
DESKTOP_PID="$(desktop_pid)"
info "quiverdesktop is pid $DESKTOP_PID"

# `pkill -x quiverdesktop` is what the update and uninstall lifecycles rely
# on. It matches on the process NAME, so this is the first place that
# assumption has ever been checked against a real running build.
assert_eq "quiverdesktop" "$(ps -p "$DESKTOP_PID" -o comm=)" \
	"the running app's process name (what pkill -x must match)"

screenshot "01-desktop-window-over-adopted-daemon"

# --- assert: it ADOPTED, it did not spawn ----------------------------------

say "The sideload claim: one daemon, and it is the one that was already there"
# Give the app a generous window to do the wrong thing. ensure_running probes
# and then would spawn; wait_for_ready allows 5s. If a second daemon were
# ever going to appear, it would be here.
sleep 10
assert_daemon_count 1
assert_eq "$HEADLESS_PID" "$(daemon_pid)" \
	"the daemon pid while the app is connected (unchanged = adopted, not replaced)"
list_core_daemons >"$SCENARIO_DIR/daemons-while-connected.txt"

# The app says so itself. This log line comes from the early return in
# SidecarManager::ensure_running, the branch that skips the spawn entirely.
if grep -q "a daemon is already listening" "$SCENARIO_DIR/desktop.log"; then
	ok "the app logged the adoption: $(grep -m1 'a daemon is already listening' "$SCENARIO_DIR/desktop.log")"
else
	fail "the app never logged the adoption branch; it may have spawned instead. Log tail:
$(tail -40 "$SCENARIO_DIR/desktop.log")"
fi
grep -q "spawned quiver.core" "$SCENARIO_DIR/desktop.log" \
	&& fail "the app spawned a sidecar despite a daemon already listening" \
	|| ok "the app never logged a sidecar spawn"

# It is genuinely talking to that daemon, not merely coexisting with it: the
# app announces its own arrow on every successful connection, so the row
# appearing in THIS daemon's catalog is end-to-end proof of the connection.
say "Confirming the app is actually talking to the adopted daemon"
# The app's own webview has to have loaded, connected and run announceSelf for
# this row to carry the app's build tag -- so this is an end-to-end check of
# the whole stack, not just of the Rust transport.
wait_for_catalogued "$DESK_ARROW" 90 "the app is served by the adopted daemon"
grep -q "POST /v0/arrow" "$SCENARIO_DIR/daemon.log" \
	|| fail "the adopted daemon never received a POST /v0/arrow from the app"
ok "the adopted daemon logged the app's own self-announce POST"
api_body GET /v0/arrow | jq '.' >"$SCENARIO_DIR/catalog-while-connected.json"

# --- assert: quitting the app leaves the daemon alone ----------------------

say "Quitting the app must not take the daemon with it"
# reap() is documented as a no-op for an adopted daemon -- this is the half of
# the guarantee that keeps a headless service alive when a user closes a GUI
# they happened to open.
pkill -x quiverdesktop
wait_for_no_desktop_window 30
assert_desktop_count 0
sleep 5
assert_daemon_count 1
assert_eq "$HEADLESS_PID" "$(daemon_pid)" "the daemon pid after the app exited"
assert_eq "200" "$(api_status GET /v0/health)" "the daemon still answers after the app exited"
screenshot "02-app-closed-daemon-survives"

# --- and the whole thing comes back off ------------------------------------

say "Uninstalling through the API with NO variables, exactly as the UI does"
# src/features/arrow-details/components/hero.tsx calls uninstall with no
# variables form at all. quiver.desktop's uninstall steps expand only
# ${QUIVER_DESKTOP_APPIMAGE_PATH}, which has a default -- but the engine used
# to demand every declared no-default variable on every method, so this call
# was refused with "required variable not provided" and the uninstall button
# could never work. Sent bare here on purpose.
api_ok POST "/v0/runtime/$(ns_enc "$DESK_ARROW")/uninstall" '{"variables":{}}' 202 >/dev/null
ok "POST /v0/runtime/$DESK_ARROW/uninstall accepted with no variables (202)"
wait_for_state "$DESK_ARROW" absent 120
assert_no_file "$INSTALL_PATH" "the installed app after uninstall"
assert_daemon_count 1
assert_daemon_alive "after the uninstall"

scenario_end

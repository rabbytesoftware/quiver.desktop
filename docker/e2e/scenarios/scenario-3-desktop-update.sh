#!/usr/bin/env bash
# SCENARIO 3
#
#   "Desktop seeds and installs core -> core has an update upstream, desktop
#    is untouched (no updates to it). Then later desktop receives an update,
#    quiver.core must show that to the user, then if the user agrees to
#    update, core executes its update, and then opens it up again."
#
# Three separate claims, asserted separately:
#
#   A. Launching the app on a machine with no daemon brings one up, and that
#      daemon registers quiver.core's own arrow by itself.
#   B. Core can replace itself while the app is running, and the app's own
#      arrow stays un-outdated through it -- one moving without the other.
#   C. When a NEW desktop release appears, core reports the app's own arrow
#      as outdated, and running that arrow's update lifecycle stops the old
#      process, replaces the file, and brings a new one back up.
#
# The "user agrees" click is a REAL click: a pointer moved onto the button in
# the running app and a mouse button pressed. Everything it sets off is the
# app's own code, including resolving its own release asset from the release
# page, which is the half that used to be missing and made the button 422
# before a single step ran. It does so without api.github.com, whose
# anonymous quota a shared IP exhausts.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin "3-desktop-update" \
	"desktop seeds core; core updates alone; then desktop updates and reopens"

# One row each, for their whole life: a catalog identity is namespace@channel,
# and an update moves what that row has installed (resolved_ref) in place.
CORE_ARROW="$CORE_NS@stable"
DESK_ARROW="$DESK_NS@stable"
INSTALL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/Quiver"
INSTALL_PATH="$INSTALL_DIR/Quiver.AppImage"

# --- preconditions ---------------------------------------------------------

say "Preconditions: no daemon anywhere, no quiver on PATH, one release of each"
reset_state
reset_upstream
assert_daemon_count 0
assert_desktop_count 0
assert_no_file /usr/local/bin/quiver "quiver.core on PATH"
assert_no_file "$HOME/.quiver/self/quiver" "a previously self-installed core"

publish_manifest rabbytesoftware/quiver.core "$CORE_V1" "/workspace/build/src/quiver.core/ARROW.md"
publish_release rabbytesoftware/quiver.core "$CORE_V1" "$BUILD_BIN/quiver-$CORE_V1"
mark_latest rabbytesoftware/quiver.core "$CORE_V1"
git_tag rabbytesoftware/quiver.core "$CORE_V1"

publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" "/workspace/build/src/quiver.desktop/ARROW.md"
cp "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1" "/tmp/$DESKTOP_ASSET"
publish_release rabbytesoftware/quiver.desktop "$DESK_V1" "/tmp/$DESKTOP_ASSET"
mark_latest rabbytesoftware/quiver.desktop "$DESK_V1"
git_tag rabbytesoftware/quiver.desktop "$DESK_V1"

# The user got the app the way install.sh gets it -- one file, at the exact
# path ARROW.md's own install lifecycle and install.sh both place it. Quiver
# is not involved: there is no Quiver on this machine yet. This is what makes
# the app the SEED rather than the installed thing.
say "Placing the desktop app the way install.sh would, with no Quiver involved"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1" "$INSTALL_PATH"
assert_file "$INSTALL_PATH" "the desktop app before anything else exists"

# ===========================================================================
# A. The app seeds core
# ===========================================================================

say "A. Launching the desktop app on a machine with no daemon"
launch_desktop "$INSTALL_PATH" desktop-v1
wait_for_desktop_window 120
assert_desktop_count 1
DESKTOP_PID_V1="$(desktop_pid)"
info "quiverdesktop (v1) is pid $DESKTOP_PID_V1"
assert_eq "quiverdesktop" "$(ps -p "$DESKTOP_PID_V1" -o comm=)" \
	"the running app's process name"

wait_for_daemon 60
assert_daemon_count 1
SEEDED_DAEMON_PID="$(daemon_pid)"
ok "the app brought a quiver.core daemon up (pid $SEEDED_DAEMON_PID)"
list_core_daemons >"$SCENARIO_DIR/seeded-daemon.txt"

# The daemon it started is its own bundled sidecar, not something that was
# already on the machine: its argv[0] is inside the app's extracted bundle.
DAEMON_CMD="$(tr '\0' ' ' <"/proc/$SEEDED_DAEMON_PID/cmdline")"
info "seeded daemon cmdline: $DAEMON_CMD"
assert_contains "$DAEMON_CMD" "quiver-appimage" "the seeded daemon's command line"
screenshot "01-app-seeded-core"

say "A. The seeded core registered its own arrow, unprompted"
assert_eq "200" "$(api_status GET "/v0/arrow/$(ns_enc "$CORE_ARROW")")" \
	"GET /v0/arrow/$CORE_ARROW"
assert_eq "Quiver Core" "$(arrow_field "$CORE_ARROW" '.data.name')" \
	"the seeded core's self-registered arrow"

say "A. And the app announced ITSELF to it"
# announceSelf fires on every successful daemon connection: a build stamped
# with a stable-* tag by build.rs registers quiver.desktop@stable and adopts
# that tag as what is installed. Core's Add-time preinstalled probe then finds
# the app already present at the manifest's own install path and lands the
# arrow at Ready with no install ever running -- which is exactly the
# situation: the user installed it outside Quiver.
wait_for_catalogued "$DESK_ARROW" 120 "the app announced itself"
assert_eq "stable" "$(catalogued_refs "$DESK_NS")" \
	"the selector the running build filed itself under"
wait_for_resolved "$DESK_ARROW" "$DESK_V1" 60 "the build the app declared installed"
wait_for_state "$DESK_ARROW" ready 60
ok "core's preinstalled probe found the app already installed and marked it ready"
arrow_detail "$DESK_ARROW" | jq '.' >"$SCENARIO_DIR/desktop-arrow-before.json"

# ===========================================================================
# B. Core updates alone; the desktop app is untouched
# ===========================================================================

say "B. Publishing a new quiver.core, and nothing new for quiver.desktop"
publish_manifest rabbytesoftware/quiver.core "$CORE_V2" "/workspace/build/src/quiver.core/ARROW.md"
publish_core_asset "$CORE_V2"
git_tag rabbytesoftware/quiver.core "$CORE_V2"
mark_latest rabbytesoftware/quiver.core "$CORE_V2"

say "B. Core notices its own drift; the desktop arrow must not"
core_outdated=0
for _ in $(seq 1 40); do
	arrow_detail "$CORE_ARROW" >/dev/null
	arrow_detail "$DESK_ARROW" >/dev/null
	sleep 1
	[ "$(arrow_field "$CORE_ARROW" '.data.outdated')" = "true" ] && { core_outdated=1; break; }
done
[ "$core_outdated" = "1" ] || fail "core never noticed its own new release"
ok "quiver.core's own arrow is outdated (available $(arrow_field "$CORE_ARROW" '.data.available.ref'))"

assert_eq "false" "$(arrow_field "$DESK_ARROW" '.data.outdated')" \
	"the desktop arrow's outdated flag while only core has a new release"
assert_eq "ready" "$(runtime_state "$DESK_ARROW")" \
	"the desktop arrow's state while only core has a new release"

say "B. Running core's own update"
# quiver.core settled its own runtime on boot; ready and outdated are both a
# legal starting point for the update.
for _ in $(seq 1 60); do
	case "$(runtime_state "$CORE_ARROW")" in ready|outdated) break ;; esac
	sleep 1
done
info "core self-arrow state before update: $(runtime_state "$CORE_ARROW")"

api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW")/update" '{}' 202 >/dev/null

became_v2=0
for _ in $(seq 1 90); do
	if [ "$(api_status GET /versions)" = "200" ] && [ "$(daemon_version)" = "$CORE_V2" ]; then
		became_v2=1; break
	fi
	sleep 1
done
[ "$became_v2" = "1" ] || fail "the seeded daemon never came back as $CORE_V2 (now: $(daemon_version))"
assert_eq "$CORE_V2" "$(daemon_version)" "the version the seeded daemon now reports"
assert_daemon_count 1

say "B. The desktop app must have survived core replacing itself"
# The ledger flags an unmitigated SIGPIPE hazard in the other direction (a
# spawned sidecar dying when its reader goes away). This is the same pipe
# from the other end: core was replaced while the app still held its
# stdout/stderr. The app must still be the same process, still on screen.
assert_desktop_count 1
assert_eq "$DESKTOP_PID_V1" "$(desktop_pid)" "the desktop pid across core's self-update"
process_alive "$DESKTOP_PID_V1" || fail "the desktop process died during core's self-update"
wmctrl -l | grep -q Quiver || fail "the desktop window vanished during core's self-update"
ok "the desktop app is untouched: same pid, window still mapped"
screenshot "02-desktop-alive-after-core-self-update"

# ===========================================================================
# C. A new desktop release appears; core shows it, then applies it
# ===========================================================================

say "C. Publishing quiver.desktop $DESK_V2 upstream"
publish_manifest rabbytesoftware/quiver.desktop "$DESK_V2" "/workspace/build/src/quiver.desktop/ARROW.md"
cp "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V2" "/tmp/$DESKTOP_ASSET"
publish_release rabbytesoftware/quiver.desktop "$DESK_V2" "/tmp/$DESKTOP_ASSET"
git_tag rabbytesoftware/quiver.desktop "$DESK_V2"
mark_latest rabbytesoftware/quiver.desktop "$DESK_V2"

DESK_URL="https://github.com/rabbytesoftware/quiver.desktop/releases/download/$DESK_V2/$DESKTOP_ASSET"
DESK_SUM_V2="$(sha256_of "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V2")"
DESK_SUM_V1="$(sha256_of "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1")"
assert_eq "$DESK_SUM_V1" "$(sha256_of "$INSTALL_PATH")" "the installed app is still $DESK_V1"

say "C. quiver.core must show the user that quiver.desktop is behind"
desk_outdated=0
for _ in $(seq 1 60); do
	arrow_detail "$DESK_ARROW" >/dev/null
	sleep 1
	[ "$(arrow_field "$DESK_ARROW" '.data.outdated')" = "true" ] && { desk_outdated=1; break; }
done
[ "$desk_outdated" = "1" ] || fail "core never marked the desktop arrow outdated: $(arrow_detail "$DESK_ARROW" | jq -c '.data | {state,resolved_ref,outdated,available}')"
ok "the desktop arrow reports outdated=true"
assert_eq "$DESK_V2" "$(arrow_field "$DESK_ARROW" '.data.available.ref')" \
	"the desktop release core found ahead"
assert_eq "outdated" "$(runtime_state "$DESK_ARROW")" \
	"the desktop arrow's runtime state (what drives the badge in the UI)"
arrow_detail "$DESK_ARROW" | jq '.' >"$SCENARIO_DIR/desktop-arrow-outdated.json"
screenshot "03-core-reports-desktop-outdated"

say "C. The user agrees, by actually clicking Update"
# THE REAL CLICK. Not a request this harness composed: a pointer moved onto
# the button the person sees and a mouse button pressed, so everything after
# it is the app's own code -- hero.tsx's handler, releaseVariables(), the
# invoke into src-tauri/src/release/, a real HTTPS request to the releases
# API, and the mutation that posts the result to core.
#
# This is what the ledger recorded as the last thing NOT proven. The lifecycle
# mechanics were already green here, but they were reached by a POST carrying
# values the harness had resolved itself; the button, as shipped, sent no
# variables and 422'd. Driving the pointer is the only way to prove the wiring
# rather than assert it.
say "C. Opening Quiver's own page in the running app"
# The Library shelf's card for Quiver Desktop, which is a real router Link to
# /arrow/$ (collection-arrow-tile.tsx) -- the same thing a person clicks. The
# card, not the sidebar row: it is a ~450x220 target rather than a ~20px-tall
# one, so this does not depend on the sidebar's exact line height. The shelf
# lists quiver.core's own card first and this one second, so it is the right
# hand card; the screenshot taken immediately above shows the layout being
# clicked into.
click_in_app 952 190
sleep 3
screenshot "04-quiver-own-page"

# The app resolves the asset of the release the row is moving to (the
# detail's available.ref), not whatever is newest.
RELEASE_PAGE="/rabbytesoftware/quiver.desktop/releases/expanded_assets/$DESK_V2"
RELEASES_API="/repos/rabbytesoftware/quiver.desktop/releases/tags/$DESK_V2"
PAGE_HITS_BEFORE="$(upstream_hits "$RELEASE_PAGE")"
API_HITS_BEFORE="$(upstream_hits "$RELEASES_API")"
info "the release page has been asked $PAGE_HITS_BEFORE times before the click"

UPDATE_BUTTON="$(find_button_center)"
info "the hero's primary action is at window $UPDATE_BUTTON"
# shellcheck disable=SC2086  # deliberately two arguments
click_in_app $UPDATE_BUTTON
screenshot "05-update-clicked"

say "C. The click must have sent the app to the release page for its own asset"
# Read off the upstream fixture's own request log. This request exists only
# because the app resolved its own release at click time: no step of the
# manifest reads that page, and this harness never calls it.
#
# NOT asserted by polling the runtime state: the row keeps stable-1.0
# installed until the update execution ends and core advances it, near the
# bottom of this scenario, so a poll for a transient `updating` here would be
# racing a state that does not settle until well after this point.
wait_for_upstream_hit "$RELEASE_PAGE" "$PAGE_HITS_BEFORE" 30 \
	"the app read its own release's page when Update was clicked"
assert_eq "$API_HITS_BEFORE" "$(upstream_hits "$RELEASES_API")" \
	"api.github.com was not asked: its anonymous quota is not the app's to spend"

say "C. The old process must go away"
gone=0
for _ in $(seq 1 60); do
	process_alive "$DESKTOP_PID_V1" || { gone=1; break; }
	sleep 1
done
[ "$gone" = "1" ] || fail "the old desktop process ($DESKTOP_PID_V1) was never killed -- 'pkill -x quiverdesktop' did not match it"
ok "the old desktop process ($DESKTOP_PID_V1) is gone"

# THE SIGPIPE HAZARD, asserted directly. The daemon executing this update is
# the sidecar the app itself spawned, so its stdout and stderr are pipes whose
# only reader was the process the step above just killed. Go kills a process
# outright on SIGPIPE from a write to fd 1 or 2, so the daemon writing one
# more log line after the app dies would take the update down with it -- mid
# update, with the old app already gone and the new one not yet placed.
say "C. The daemon running the update must survive losing its log reader"
assert_daemon_count 1
assert_daemon_alive "right after the app it belongs to was killed"

assert_eq "$DESK_SUM_V2" "$(sha256_of "$INSTALL_PATH")" \
	"the installed file's sha256 after the update"

say "C. The new one must come back up"
back=0
for _ in $(seq 1 120); do
	if [ "$(count_desktop)" = "1" ]; then back=1; break; fi
	sleep 1
done
[ "$back" = "1" ] || fail "no new desktop process came up after the update (count=$(count_desktop))"
DESKTOP_PID_V2="$(desktop_pid)"
assert_ne "$DESKTOP_PID_V1" "$DESKTOP_PID_V2" "the desktop pid after the update"
# No single-instance guard exists on Linux, so "a window exists" is not
# enough -- two would also satisfy that, and would mean the kill step failed.
assert_desktop_count 1
wait_for_desktop_window 120

# The strongest possible statement about WHICH build is running: the path the
# new process is executing out of. Each packaged artifact extracts to a
# directory named after its own build tag, so this cannot be satisfied by the
# old binary under any circumstances.
RUNNING_EXE="$(readlink -f "/proc/$DESKTOP_PID_V2/exe")"
info "the relaunched app is running $RUNNING_EXE"
assert_contains "$RUNNING_EXE" "quiver-appimage/$DESK_V2/" \
	"the executable path of the relaunched app"
screenshot "06-desktop-reopened-after-update"

say "C. The same row now has the new build installed"
# Core advances the row in place once the update's steps succeed and the
# target tag still names the commit it staged; the relaunched app's own
# announce then finds its tag already recorded and sends nothing more.
wait_for_resolved "$DESK_ARROW" "$DESK_V2" 60 "the row advanced to the new build"
api_body GET /v0/arrow | jq '.' >"$SCENARIO_DIR/catalog-after-update.json"
assert_eq "stable" "$(catalogued_refs "$DESK_NS")" \
	"the desktop rows in the catalog after the update (one row, never a second one per version)"
assert_eq "false" "$(arrow_field "$DESK_ARROW" '.data.outdated')" \
	"the row's outdated flag once it has the newest build"

assert_daemon_count 1
assert_daemon_alive "at the end of the scenario"
assert_eq "$CORE_V2" "$(daemon_version)" "the daemon version at the end of the scenario"

say "C. The update lifecycle itself must have run to completion"
# The runtime aggregate carries the whole execution: every step, and the
# outcome. This is the real "core executed desktop's update" assertion --
# stronger than any state name, because it names the four steps that ran.
#
UPDATE_RETURN="$(api_body GET "/v0/runtime/$(ns_enc "$DESK_ARROW")" | jq -c '.data.last_return')"
printf '%s\n' "$UPDATE_RETURN" | jq '.' >"$SCENARIO_DIR/update-execution.json"
assert_eq "_update" "$(printf '%s' "$UPDATE_RETURN" | jq -r '.method')" \
	"the lifecycle method that ran"
assert_eq "success" "$(printf '%s' "$UPDATE_RETURN" | jq -r '.outcome')" \
	"the update execution's outcome"
assert_eq "completed,completed,completed,completed" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '[.steps[].status] | join(",")')" \
	"every update step's status (fetch, stop, install, relaunch)"

# WHAT THE BUTTON ACTUALLY SENT. The execution's own recorded variables, read
# back off the runtime aggregate: nothing in this harness put them there.
# They can only have come from the app resolving its own release from the
# release page at the moment the pointer went down, which is the claim the
# whole click is here to make.
assert_eq "$DESK_URL" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_ASSET_URL')" \
	"the asset URL the app resolved and sent"
assert_eq "$DESK_SUM_V2" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_CHECKSUM')" \
	"the checksum the app read off the release page's digest"
# And it is the NEW release, not the one the row sits at -- the distinction
# that makes a caller-side resolver necessary in the first place.
assert_ne "$DESK_SUM_V1" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_CHECKSUM')" \
	"the checksum the app sent (must not be the installed version's)"

scenario_end

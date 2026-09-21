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
# app's own code, including resolving its own release asset against the
# releases API, which is the half that used to be missing and made the button
# 422 before a single step ran.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

scenario_begin "3-desktop-update" \
	"desktop seeds core; core updates alone; then desktop updates and reopens"

CORE_ARROW_V1="$CORE_NS@$CORE_V1"
DESK_ARROW_V1="$DESK_NS@$DESK_V1"
DESK_ARROW_V2="$DESK_NS@$DESK_V2"
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
assert_eq "200" "$(api_status GET "/v0/arrow/$(ns_enc "$CORE_ARROW_V1")")" \
	"GET /v0/arrow/$CORE_ARROW_V1"
assert_eq "Quiver Core" "$(arrow_field "$CORE_ARROW_V1" '.data.name')" \
	"the seeded core's self-registered arrow"

say "A. And the app announced ITSELF to it"
# announceSelf fires on every successful daemon connection, at the tag
# build.rs baked in. Core's Add-time preinstalled probe then finds the app
# already present at the manifest's own install path and lands the arrow at
# Ready with no install ever running -- which is exactly the situation: the
# user installed it outside Quiver.
wait_for_catalogued "$DESK_ARROW_V1" 120 "the app announced itself"
assert_eq "$DESK_V1" "$(catalogued_refs "$DESK_NS")" \
	"the ref the running build announced itself under"
wait_for_state "$DESK_ARROW_V1" ready 60
ok "core's preinstalled probe found the app already installed and marked it ready"
arrow_detail "$DESK_ARROW_V1" | jq '.' >"$SCENARIO_DIR/desktop-arrow-before.json"

# ===========================================================================
# B. Core updates alone; the desktop app is untouched
# ===========================================================================

say "B. Publishing a new quiver.core, and nothing new for quiver.desktop"
publish_manifest rabbytesoftware/quiver.core "$CORE_V2" "/workspace/build/src/quiver.core/ARROW.md"
publish_release rabbytesoftware/quiver.core "$CORE_V2" "$BUILD_BIN/quiver-$CORE_V2"
git_tag rabbytesoftware/quiver.core "$CORE_V2"
mark_latest rabbytesoftware/quiver.core "$CORE_V2"

CORE_URL="https://github.com/rabbytesoftware/quiver.core/releases/download/$CORE_V2/quiver-$CORE_V2"
CORE_SUM="$(sha256_of "$BUILD_BIN/quiver-$CORE_V2")"

say "B. Core notices its own drift; the desktop arrow must not"
core_outdated=0
for _ in $(seq 1 40); do
	arrow_detail "$CORE_ARROW_V1" >/dev/null
	arrow_detail "$DESK_ARROW_V1" >/dev/null
	sleep 1
	[ "$(arrow_field "$CORE_ARROW_V1" '.data.outdated')" = "true" ] && { core_outdated=1; break; }
done
[ "$core_outdated" = "1" ] || fail "core never noticed its own new release"
ok "quiver.core's own arrow is outdated (recommended $(arrow_field "$CORE_ARROW_V1" '.data.recommended_ref'))"

assert_eq "false" "$(arrow_field "$DESK_ARROW_V1" '.data.outdated')" \
	"the desktop arrow's outdated flag while only core has a new release"
assert_eq "ready" "$(runtime_state "$DESK_ARROW_V1")" \
	"the desktop arrow's state while only core has a new release"

say "B. Bootstrapping and running core's own update"
api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW_V1")/install" \
	'{"variables":{}}' 202 >/dev/null
# install on an Outdated arrow settles it back to ready; either is a legal
# starting point for BeginUpdate.
for _ in $(seq 1 60); do
	case "$(runtime_state "$CORE_ARROW_V1")" in ready|outdated) break ;; esac
	sleep 1
done
info "core self-arrow state before update: $(runtime_state "$CORE_ARROW_V1")"

api_ok POST "/v0/runtime/$(ns_enc "$CORE_ARROW_V1")/update" \
	"$(jq -nc --arg u "$CORE_URL" --arg c "$CORE_SUM" \
		'{variables:{QUIVER_RELEASE_ASSET_URL:$u,QUIVER_RELEASE_CHECKSUM:$c}}')" \
	202 >/dev/null

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
# from the other end: core exec'd itself while the app still held its
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
	arrow_detail "$DESK_ARROW_V1" >/dev/null
	sleep 1
	[ "$(arrow_field "$DESK_ARROW_V1" '.data.outdated')" = "true" ] && { desk_outdated=1; break; }
done
[ "$desk_outdated" = "1" ] || fail "core never marked the desktop arrow outdated: $(arrow_detail "$DESK_ARROW_V1" | jq -c '.data | {state,outdated,recommended_ref}')"
ok "the desktop arrow reports outdated=true"
assert_eq "$DESK_V2" "$(arrow_field "$DESK_ARROW_V1" '.data.recommended_ref')" \
	"the desktop version core is recommending"
assert_eq "outdated" "$(runtime_state "$DESK_ARROW_V1")" \
	"the desktop arrow's runtime state (what drives the badge in the UI)"
arrow_detail "$DESK_ARROW_V1" | jq '.' >"$SCENARIO_DIR/desktop-arrow-outdated.json"
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
# The Library shelf's card for Quiver, which is a real router Link to
# /arrow/$ (collection-arrow-tile.tsx) -- the same thing a person clicks. The
# card, not the sidebar row: it is a ~450x220 target rather than a ~20px-tall
# one, so this does not depend on the sidebar's exact line height. The
# screenshot taken immediately above shows the layout being clicked into.
click_in_app 493 190
sleep 3
screenshot "04-quiver-own-page"

RELEASES_API="/repos/rabbytesoftware/quiver.desktop/releases/latest"
API_HITS_BEFORE="$(upstream_hits "$RELEASES_API")"
info "the releases API has been asked $API_HITS_BEFORE times before the click"

UPDATE_BUTTON="$(find_button_center)"
info "the hero's primary action is at window $UPDATE_BUTTON"
# shellcheck disable=SC2086  # deliberately two arguments
click_in_app $UPDATE_BUTTON
screenshot "05-update-clicked"

say "C. The click must have sent the app to the releases API for its own asset"
# Read off the upstream fixture's own request log, which is written by the
# stand-in for api.github.com and by nothing else. This request exists only
# because the app resolved its own release at click time: no step of the
# manifest talks to the releases API, and this harness never calls it.
#
# NOT asserted by polling the runtime state: this arrow reports `outdated`
# for the whole click-to-relaunch window, because the row stays at stable-1.0
# (with upstream's latest at stable-1.1) until the relaunched process
# self-announces and retires it, near the bottom of this scenario. A poll for
# a transient `updating` here would be racing a state that does not clear
# until well after this point.
wait_for_upstream_hit "$RELEASES_API" "$API_HITS_BEFORE" 30 \
	"the app asked api.github.com for its own latest release when Update was clicked"

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

say "C. The update lifecycle itself must have run to completion"
# The runtime aggregate carries the whole execution: every step, and the
# outcome. This is the real "core executed desktop's update" assertion --
# stronger than any state name, because it names the four steps that ran.
UPDATE_RETURN="$(api_body GET "/v0/runtime/$(ns_enc "$DESK_ARROW_V1")" | jq -c '.data.last_return')"
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
# They can only have come from the app resolving its own release against the
# releases API at the moment the pointer went down, which is the claim the
# whole click is here to make.
assert_eq "$DESK_URL" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_ASSET_URL')" \
	"the asset URL the app resolved and sent"
assert_eq "$DESK_SUM_V2" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_CHECKSUM')" \
	"the checksum the app read off the releases API's own per-asset digest"
# And it is the NEW release, not the one the row sits at -- the distinction
# that makes a caller-side resolver necessary in the first place.
assert_ne "$DESK_SUM_V1" \
	"$(printf '%s' "$UPDATE_RETURN" | jq -r '.variables.QUIVER_RELEASE_CHECKSUM')" \
	"the checksum the app sent (must not be the installed version's)"

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

say "C. And the thing that came back announces itself as the NEW build"
# Independent of the path check and end-to-end: the relaunched app's own
# frontend has to have loaded, connected to core and run announceSelf at the
# tag build.rs baked into it for this row to exist.
wait_for_catalogued "$DESK_ARROW_V2" 120 "the relaunched app announced itself as the NEW build"
api_body GET /v0/arrow | jq '.' >"$SCENARIO_DIR/catalog-after-update.json"

assert_daemon_count 1
assert_daemon_alive "at the end of the scenario"
assert_eq "$CORE_V2" "$(daemon_version)" "the daemon version at the end of the scenario"

# --- the stale catalog row is retired, not left behind ---------------------
#
# Running an `update` lifecycle replaces what the manifest's steps replace; it
# does not move the catalog row to the ref the version check recommended
# (${REF} inside those steps is always the ref being updated FROM, never the
# one being updated TO -- which is why the asset URL has to be supplied by the
# caller rather than templated from ${REF}). Left alone, that would leave the
# OLD row catalogued and outdated forever: nothing else ever revisits it once
# self-announce has created a new row for the version actually running.
#
# So self-announce retires every OTHER installed version of quiver.desktop's
# own namespace once it announces the new one -- the same "only one real
# install exists" invariant quiver.core's own selfarrow.RetireStale enforces
# for itself, applied here from the client side since quiver.desktop is a
# separate process with no access to call that directly. The relaunched
# stable-1.1 process's own self-announce (already waited for above, the same
# call that proved "C. And the thing that came back...") is what performs
# this, so by the time execution reaches here it should already be done --
# waited for explicitly anyway, since a slow DELETE racing this assertion is
# exactly the kind of flake worth ruling out rather than hoping past.
say "C. The stale stable-1.0 row is retired once the new one announces"
deadline=$(( SECONDS + 30 ))
while [ "$SECONDS" -lt "$deadline" ]; do
	[ "$(catalogued_refs "$DESK_NS")" = "$DESK_V2" ] && break
	sleep 1
done
assert_eq "$DESK_V2" "$(catalogued_refs "$DESK_NS")" \
	"the desktop refs in the catalog after the update (only the version actually running, not the one it replaced)"
# is_catalogued, never arrow_field/arrow_detail, for an existence check: GetDetail
# falls back to a live remote preview for an uncatalogued namespace and answers
# 200 with a full-looking body, which would make a removed row look present.
if is_catalogued "$DESK_ARROW_V1"; then
	fail "the old stable-1.0 row is still catalogued after the update"
fi
ok "the old stable-1.0 row is gone from the catalog"
assert_eq "false" "$(arrow_field "$DESK_ARROW_V2" '.data.outdated')" \
	"the current row's outdated flag now that it is the only installed version"

scenario_end

#!/usr/bin/env bash
# Shared constants, assertions and drivers for the three E2E scenarios.
#
# Every assertion here fails loudly and says what it actually saw. A scenario
# that cannot prove its claim must stop at the claim it cannot prove, not
# carry on to a green summary -- the whole point of this harness is that its
# "pass" means something.

# --- identities ------------------------------------------------------------
#
# Everything below is read by the scenario scripts that source this file, not
# by this file itself, which is the whole point of a shared library and also
# what SC2034 cannot see.
# shellcheck disable=SC2034

CORE_NS="github.com/rabbytesoftware/quiver.core"
DESK_NS="github.com/rabbytesoftware/quiver.desktop"

CORE_V1="stable-26.5.90"
CORE_V2="stable-26.5.91"
DESK_V1="stable-1.0"
DESK_V2="stable-1.1"

TARGET_TRIPLE="${TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/{print $2}')}"

# The Linux release asset quiver.desktop's bundler produces, and the name the
# scenarios publish it under. Tauri's appimage bundler names it
# {productName}_{version}_{arch}.AppImage, with version coming from
# tauri.conf.json (a static "0.1.0" here, deliberately -- see ARROW.md) and
# arch from the build target. The name is carried rather than derived by
# anything under test: the manifest never constructs it, the caller resolves
# the asset and hands the URL in, exactly as install.sh does against the real
# releases API.
case "$TARGET_TRIPLE" in
	aarch64-*) DESKTOP_ARCH=aarch64 ;;
	x86_64-*)  DESKTOP_ARCH=amd64 ;;
	*)         DESKTOP_ARCH="${TARGET_TRIPLE%%-*}" ;;
esac
DESKTOP_ASSET="Quiver_0.1.0_${DESKTOP_ARCH}.AppImage"

# --- locations -------------------------------------------------------------

BUILD_BIN=/workspace/build/bin
CARGO_TARGET=/workspace/build/cargo-target
RUN_DIR=/workspace/run
UPSTREAM_STATE="$RUN_DIR/upstream"
RESULTS=/workspace/results
E2E_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Where a scenario's own logs and screenshots go. Set by scenario_begin.
SCENARIO_DIR=""
SCENARIO_NAME=""
ASSERTIONS=0

export QUIVER_HOME="${QUIVER_HOME:-$HOME/.quiver}"
SOCKET="$HOME/.quiver/quiver.sock"

# --- output ----------------------------------------------------------------

say()  { printf '\n\033[1;36m» %s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }
ok()   { ASSERTIONS=$((ASSERTIONS + 1)); printf '  \033[1;32mPASS\033[0m %s\n' "$*"; }
fail() {
	printf '\n  \033[1;31mFAIL\033[0m %s\n' "$*" >&2
	if [ -n "$SCENARIO_DIR" ]; then
		printf '%s\n' "FAIL: $*" >>"$SCENARIO_DIR/result.txt"
		collect_diagnostics || true
	fi
	exit 1
}

scenario_begin() {
	SCENARIO_NAME="$1"
	SCENARIO_DIR="$RESULTS/$1"
	rm -rf "$SCENARIO_DIR"
	mkdir -p "$SCENARIO_DIR"
	ASSERTIONS=0
	say "SCENARIO $1: $2"
	printf '%s\n' "$2" >"$SCENARIO_DIR/description.txt"
}

scenario_end() {
	printf 'PASS (%d assertions)\n' "$ASSERTIONS" >>"$SCENARIO_DIR/result.txt"
	say "SCENARIO $SCENARIO_NAME PASSED -- $ASSERTIONS assertions"
}

collect_diagnostics() {
	local dest="$SCENARIO_DIR/diagnostics"
	mkdir -p "$dest"
	ps -ef >"$dest/ps.txt" 2>&1 || true
	ls -la "$HOME/.quiver" >"$dest/quiver-home.txt" 2>&1 || true
	# The daemon's OWN log files, not the copy that reaches a parent process's
	# pipe: when the app pumping that pipe has just been killed, this is the
	# only surviving record of what the daemon did next.
	cp -r "$HOME/.quiver/logs" "$dest/core-logs" 2>/dev/null || true
	list_core_daemons >"$dest/daemons.txt" 2>&1 || true
	pgrep -a quiverdesktop >"$dest/desktop-processes.txt" 2>&1 || true
	wmctrl -l >"$dest/windows.txt" 2>&1 || true
	screenshot "failure" 2>/dev/null || true
}

# --- assertions ------------------------------------------------------------

assert_eq() {
	local want="$1" got="$2" what="$3"
	[ "$want" = "$got" ] || fail "$what: expected [$want], got [$got]"
	ok "$what = $got"
}

assert_ne() {
	local unwanted="$1" got="$2" what="$3"
	[ "$unwanted" != "$got" ] || fail "$what: expected anything but [$unwanted], got exactly that"
	ok "$what = $got (changed, as required)"
}

assert_contains() {
	local haystack="$1" needle="$2" what="$3"
	case "$haystack" in
		*"$needle"*) ok "$what contains '$needle'" ;;
		*) fail "$what: expected to contain [$needle], got [$haystack]" ;;
	esac
}

assert_file() {
	[ -f "$1" ] || fail "$2: expected a file at $1, found none"
	ok "$2 exists ($1, $(stat -c%s "$1") bytes)"
}

assert_no_file() {
	[ ! -e "$1" ] || fail "$2: expected nothing at $1, found $(ls -la "$1")"
	ok "$2 is absent ($1)"
}

# --- the daemon, over its real REST API ------------------------------------

# api METHOD PATH [BODY] -- one call over the unix socket the daemon actually
# binds, i.e. the same transport quiver.desktop's own UnixTransport uses. The
# HTTP status is appended as the last line so callers can assert on it.
api() {
	local method="$1" path="$2" body="${3:-}"
	local args=(--silent --show-error --unix-socket "$SOCKET"
		-X "$method" -w '\n%{http_code}' "http://localhost$path")
	if [ -n "$body" ]; then
		args+=(-H 'Content-Type: application/json' -d "$body")
	fi
	# `|| true` matters more than it looks. Under `set -euo pipefail` a curl
	# that cannot connect kills the calling scenario outright, mid-poll, with
	# no message and no diagnostics -- which is exactly what a daemon dying
	# unexpectedly looks like, and exactly when the harness most needs to
	# survive to say so. A dead daemon must surface as an assertion that
	# reports an empty body, never as a script that vanishes.
	curl "${args[@]}" 2>/dev/null || true
}

api_status() { api "$@" | tail -n1; }
api_body()   { api "$@" | sed '$d'; }

# api_ok METHOD PATH [BODY] [EXPECTED_STATUS] -- like api_body, but fails the
# scenario if the status is not what was asked for.
api_ok() {
	local method="$1" path="$2" body="${3:-}" want="${4:-200}"
	local raw status
	raw="$(api "$method" "$path" "$body")"
	status="$(printf '%s' "$raw" | tail -n1)"
	if [ "$status" != "$want" ]; then
		fail "$method $path: expected HTTP $want, got $status: $(printf '%s' "$raw" | sed '$d')"
	fi
	printf '%s' "$raw" | sed '$d'
}

ns_enc() { printf '%s' "$1" | jq -sRr @uri; }

arrow_detail() { api_body GET "/v0/arrow/$(ns_enc "$1")"; }
arrow_field()  { arrow_detail "$1" | jq -r "${2}"; }
runtime_state() {
	api_body GET "/v0/runtime/$(ns_enc "$1")" \
		| jq -r '.data.state // "«none»"' 2>/dev/null \
		|| printf '«unreachable»'
}

# daemon_alive -- is anything answering the socket at all right now?
daemon_alive() { [ "$(api_status GET /v0/health)" = "200" ]; }

assert_daemon_alive() {
	daemon_alive || fail "$1: no daemon is answering $SOCKET. Daemons: [$(list_core_daemons)]. This is the SIGPIPE hazard the ledger flags if it happened right after a process holding the daemon's stdout pipe was killed."
	ok "$1: the daemon is still answering"
}

# catalogued_refs BARE_NS -- the refs of BARE_NS that are really IN the
# catalog, comma-separated and sorted, or "" for none.
#
# GET /v0/arrow/:ns is NOT a catalog membership test and must never be used as
# one. GetDetail falls back to a live preview for an uncatalogued namespace --
# it resolves the manifest from the remote and answers 200 with a full-looking
# body for an arrow the daemon has never heard of. An early version of these
# scenarios asserted "the app announced itself" on exactly that 200 and passed
# while the app's frontend was not running at all.
#
# The list endpoint has no such fallback. It groups by BARE namespace with the
# real installed refs underneath, which is also why the ref has to be read out
# of .versions[] rather than off the row itself -- the same shape that made
# quiver.core's own RetireStale a permanent no-op.
catalogued_refs() {
	api_body GET /v0/arrow | jq -r --arg ns "$1" \
		'[.data[] | select(.namespace == $ns) | .versions[].ref] | sort | join(",")'
}

is_catalogued() {
	local bare="${1%@*}" ref="${1##*@}"
	case ",$(catalogued_refs "$bare")," in
		*",$ref,"*) return 0 ;;
		*) return 1 ;;
	esac
}

# wait_for_catalogued NS TIMEOUT WHAT -- polls until NS is genuinely in the
# catalog, then reports it.
wait_for_catalogued() {
	local ns="$1" timeout="${2:-60}" what="${3:-$1}"
	local deadline=$(( SECONDS + timeout ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		if is_catalogued "$ns"; then
			ok "$what: $ns is in the catalog"
			return 0
		fi
		sleep 1
	done
	fail "$what: $ns never appeared in the catalog within ${timeout}s. Catalogued: $(api_body GET /v0/arrow | jq -c '[.data[] | {ns: .namespace, refs: [.versions[].ref]}]')"
}

# The version of the binary SERVING RIGHT NOW, straight out of the daemon's
# own ldflags stamp (internal/api/endpoints/versions). This is the assertion
# that makes "core actually self-updated" mean something: it is answered by
# the process holding the socket, not by any file on disk.
daemon_version() {
	api_body GET /versions | jq -r '.data.version // "«unknown»"' 2>/dev/null \
		|| printf '«unreachable»'
}

wait_for_daemon() {
	local deadline=$(( SECONDS + ${1:-30} ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		if [ -S "$SOCKET" ] && [ "$(api_status GET /v0/health)" = "200" ]; then
			return 0
		fi
		sleep 0.3
	done
	fail "no daemon answering /v0/health on $SOCKET after ${1:-30}s"
}

# wait_for_state NS STATE TIMEOUT -- polls the real runtime aggregate.
wait_for_state() {
	local ns="$1" want="$2" timeout="${3:-120}"
	local deadline=$(( SECONDS + timeout )) seen=""
	while [ "$SECONDS" -lt "$deadline" ]; do
		seen="$(runtime_state "$ns")"
		[ "$seen" = "$want" ] && { ok "$ns reached state '$want'"; return 0; }
		sleep 0.5
	done
	local dump
	dump="$(api_body GET "/v0/runtime/$(ns_enc "$ns")" | jq -c '.data // .' 2>/dev/null)"
	fail "$ns never reached '$want' within ${timeout}s (last seen '$seen'); runtime: $dump"
}

# --- processes -------------------------------------------------------------

DAEMON_PATTERN='quiver[^ ]* daemon'

# count_core_daemons -- how many `quiver daemon` processes exist right now.
#
# Matched on the full command line rather than the process name, because a
# core binary is called `quiver-<version>` when the harness built it, `quiver`
# when it was installed onto PATH, and `quiver` again inside the desktop
# bundle -- while `daemon` is argv[1] in every one of those cases.
#
# `|| true` on its own is NOT enough here and was a real bug in this file:
# `pgrep -c` PRINTS "0" and EXITS 1 when nothing matches, so `$(pgrep -c ...
# || echo 0)` yields the two-line string "0\n0", and every comparison against
# it fails with the baffling "expected 0, found 0".
count_core_daemons() {
	local n
	n="$(pgrep -fc "$DAEMON_PATTERN" 2>/dev/null)" || n=0
	printf '%s' "${n:-0}"
}

list_core_daemons() { pgrep -fa "$DAEMON_PATTERN" 2>/dev/null || true; }

count_desktop() {
	local n
	n="$(pgrep -xc quiverdesktop 2>/dev/null)" || n=0
	printf '%s' "${n:-0}"
}

desktop_pid() { pgrep -xo quiverdesktop 2>/dev/null || true; }
daemon_pid()  { pgrep -fo "$DAEMON_PATTERN" 2>/dev/null || true; }

process_alive() { [ -n "$1" ] && kill -0 "$1" 2>/dev/null; }

assert_daemon_count() {
	local want="$1" got
	got="$(count_core_daemons)"
	if [ "$got" != "$want" ]; then
		fail "expected exactly $want quiver daemon process(es), found $got:
$(list_core_daemons)"
	fi
	ok "exactly $want quiver daemon process(es) running"
}

assert_desktop_count() {
	local want="$1" got
	got="$(count_desktop)"
	[ "$got" = "$want" ] || fail "expected $want quiverdesktop process(es), found $got:
$(pgrep -a quiverdesktop || true)"
	ok "exactly $want quiverdesktop process(es) running"
}

wait_for_desktop_window() {
	local timeout="${1:-60}"
	# Two statements, not one `local`: arithmetic in a `local` argument list
	# is expanded before any of that list has been assigned, so a `deadline`
	# built from `timeout` on the same line dies under `set -u`.
	local deadline=$(( SECONDS + timeout ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		if wmctrl -l 2>/dev/null | grep -q 'Quiver'; then
			# One more beat so the webview has actually painted before a
			# screenshot claims to show the app.
			sleep 3
			ok "a real Quiver window is mapped on $DISPLAY ($(wmctrl -l | grep Quiver | head -1))"
			return 0
		fi
		sleep 0.5
	done
	fail "no Quiver window appeared on $DISPLAY within ${timeout}s; windows: $(wmctrl -l 2>&1)"
}

wait_for_no_desktop_window() {
	local timeout="${1:-30}"
	local deadline=$(( SECONDS + timeout ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		wmctrl -l 2>/dev/null | grep -q 'Quiver' || { ok "no Quiver window is mapped any more"; return 0; }
		sleep 0.5
	done
	fail "a Quiver window was still mapped after ${timeout}s: $(wmctrl -l 2>&1)"
}

# --- screenshots -----------------------------------------------------------

screenshot() {
	local name="$1"
	local path="$SCENARIO_DIR/${name}.png"
	mkdir -p "$SCENARIO_DIR"
	if scrot -o "$path" 2>/dev/null; then
		info "screenshot: $path ($(stat -c%s "$path") bytes)"
		printf '%s\n' "$path" >>"$SCENARIO_DIR/screenshots.txt"
	else
		info "screenshot FAILED for $name"
	fi
}

# --- driving the real UI ---------------------------------------------------
#
# A scenario that wants to prove a BUTTON works has to press the button. These
# move a real pointer on the real X display and press a real mouse button, so
# what runs afterwards is the app's own click handler, its own mutation and
# its own IPC into Rust -- not a request the harness composed itself.

# desktop_window_id prints the X window id of the running app.
desktop_window_id() {
	xdotool search --name '^Quiver$' 2>/dev/null | tail -n 1
}

# focus_desktop raises and focuses the app window, and prints its CLIENT AREA
# geometry on screen as "X Y W H". Clicks are placed relative to that origin
# rather than to the screen, so a window manager that decorates or positions
# the window differently cannot silently move every target.
#
# xwininfo, and not `xdotool getwindowgeometry`. Under xfwm the latter reports
# the window's position within its own reparenting frame, which here is
# (10,85) while the client area actually starts at (5,56) -- a 29-pixel
# vertical lie, enough to turn a click on a sidebar row into a click on the
# empty space below it, which is exactly what it did. xwininfo's "Absolute
# upper-left" is the client area's real position on the root window.
focus_desktop() {
	local id info x y w h
	id="$(desktop_window_id)"
	[ -n "$id" ] || fail "no Quiver window to drive: $(wmctrl -l 2>&1)"

	wmctrl -i -a "$id" 2>/dev/null || true
	xdotool windowactivate --sync "$id" 2>/dev/null || true
	sleep 0.5

	info="$(xwininfo -id "$id")"
	x="$(printf '%s' "$info" | awk '/Absolute upper-left X:/ {print $4}')"
	y="$(printf '%s' "$info" | awk '/Absolute upper-left Y:/ {print $4}')"
	w="$(printf '%s' "$info" | awk '/^  Width:/ {print $2}')"
	h="$(printf '%s' "$info" | awk '/^  Height:/ {print $2}')"
	[ -n "$x" ] && [ -n "$y" ] && [ -n "$w" ] && [ -n "$h" ] ||
		fail "could not read the Quiver window's geometry: $info"
	printf '%s %s %s %s\n' "$x" "$y" "$w" "$h"
}

# click_in_app X Y -- clicks at a point given in the app window's own
# coordinates.
click_in_app() {
	local wx wy geom
	geom="$(focus_desktop)"
	wx=$(( $(printf '%s' "$geom" | cut -d' ' -f1) + $1 ))
	wy=$(( $(printf '%s' "$geom" | cut -d' ' -f2) + $2 ))
	info "clicking at window-relative ($1,$2) -> screen ($wx,$wy)"
	xdotool mousemove --sync "$wx" "$wy" sleep 0.2 click 1
	sleep 1
}

# find_button_center LABEL -- prints "X Y" in the app window's own coordinates
# for the primary action button, located by reading the pixels rather than by
# a coordinate written down once and left to rot.
#
# The hero's primary action is the one solid near-black pill in the content
# area (everything right of the sidebar). Nothing else in that area is a
# filled dark block: the status badge is an outline, the tags are outlines,
# and the banner is light grey. Locating it this way means a layout change
# moves the click with it, and a layout change that removes the button fails
# here with the screenshot attached instead of clicking empty space.
find_button_center() {
	local shot="$SCENARIO_DIR/.locate.png"
	scrot -o "$shot" 2>/dev/null || fail "could not screenshot to locate a button"
	local geom
	geom="$(focus_desktop)"
	python3 "$E2E_DIR/fixtures/find-button.py" \
		"$shot" \
		"$(printf '%s' "$geom" | cut -d' ' -f1)" \
		"$(printf '%s' "$geom" | cut -d' ' -f2)" \
		"$(printf '%s' "$geom" | cut -d' ' -f3)" \
		"$(printf '%s' "$geom" | cut -d' ' -f4)" ||
		fail "could not find the primary action button on screen (see $shot)"
}

# upstream_hits PATH -- how many times the upstream fixture has served PATH.
#
# The fixture records every request it serves as one line of JSON, which makes
# it the only witness to a request made by code this harness did not call: it
# can say that the APP went to the releases API, which no assertion against
# core's own state ever could.
upstream_hits() {
	grep -cF "\"path\": \"$1\"" "$RESULTS/upstream.log" 2>/dev/null || true
}

# wait_for_upstream_hit PATH SINCE TIMEOUT WHAT -- waits for a NEW request for
# PATH beyond the SINCE count taken before whatever was supposed to cause it,
# so an earlier phase's request cannot be mistaken for this one's.
wait_for_upstream_hit() {
	local path="$1" since="$2" timeout="${3:-30}" what="${4:-$1}"
	local deadline=$(( SECONDS + timeout ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		if [ "$(upstream_hits "$path")" -gt "$since" ]; then
			ok "$what"
			return 0
		fi
		sleep 0.5
	done
	fail "$what -- nothing new requested $path from the upstream fixture within ${timeout}s (count stayed at $since)"
}

# --- upstream fixture control ----------------------------------------------

# publish_release USER/REPO TAG FILE... -- puts assets under a tag.
publish_release() {
	local repo="$1" tag="$2"; shift 2
	local dir="$UPSTREAM_STATE/releases/$repo/$tag"
	mkdir -p "$dir"
	local f
	for f in "$@"; do cp "$f" "$dir/"; done
	info "published $repo@$tag: $*"
}

# mark_latest USER/REPO TAG -- what /releases/latest redirects to, i.e. what
# quiver.core's drift check will call the newest version.
mark_latest() {
	local repo="$1" tag="$2"
	mkdir -p "$UPSTREAM_STATE/releases/$repo"
	printf '%s\n' "$tag" >"$UPSTREAM_STATE/releases/$repo/LATEST"
	info "upstream: $repo latest release is now $tag"
}

# publish_manifest USER/REPO REF FILE -- serves FILE as ARROW.md at REF.
publish_manifest() {
	local repo="$1" ref="$2" file="$3"
	local dir="$UPSTREAM_STATE/raw/$repo/$ref"
	mkdir -p "$dir"
	cp "$file" "$dir/ARROW.md"
	info "upstream: $repo@$ref serves $(basename "$file") as ARROW.md"
}

# git_tag USER/REPO TAG -- adds a tag to the bare repo ls-remote reads.
git_tag() {
	local repo="$1" tag="$2"
	local dir="$UPSTREAM_STATE/git/${repo}.git"
	git --git-dir="$dir" tag -f "$tag" HEAD >/dev/null 2>&1 \
		|| fail "could not tag $repo with $tag"
	info "upstream: $repo now advertises tag $tag"
}

sha256_of() { sha256sum "$1" | cut -d' ' -f1; }

# reset_upstream -- an upstream with no releases and no tags at all, so each
# scenario declares the exact release history it means to test against and
# nothing leaks in from the one before it.
reset_upstream() {
	rm -rf "$UPSTREAM_STATE/raw" "$UPSTREAM_STATE/releases"
	mkdir -p "$UPSTREAM_STATE/raw" "$UPSTREAM_STATE/releases"
	local bare
	for bare in "$UPSTREAM_STATE"/git/*/*.git; do
		[ -d "$bare" ] || continue
		git --git-dir="$bare" tag -l | while read -r tag; do
			[ -n "$tag" ] && git --git-dir="$bare" tag -d "$tag" >/dev/null 2>&1
		done
	done
	info "upstream reset: no releases, no tags"
}

# --- state reset -----------------------------------------------------------

kill_everything() {
	pkill -x quiverdesktop 2>/dev/null || true
	pkill -f 'quiver[^ ]* daemon' 2>/dev/null || true
	local deadline=$(( SECONDS + 15 ))
	while [ "$SECONDS" -lt "$deadline" ]; do
		[ "$(count_core_daemons)" = "0" ] && [ "$(count_desktop)" = "0" ] && break
		sleep 0.3
	done
	pkill -9 -x quiverdesktop 2>/dev/null || true
	pkill -9 -f 'quiver[^ ]* daemon' 2>/dev/null || true
	sleep 0.5
}

# reset_state -- a genuinely fresh machine for the next scenario: no daemon,
# no desktop, no ~/.quiver, no installed app.
reset_state() {
	kill_everything
	rm -rf "$HOME/.quiver"
	rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/Quiver"
	rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/quiver-appimage"
	rm -f /usr/local/bin/quiver
	mkdir -p "$HOME/.quiver"
	write_core_config
}

# write_core_config -- the one config value the scenarios need to differ from
# the shipped default. version_check_ttl throttles the drift check to one
# remote call per arrow per hour, which is right for a product and impossible
# for a test that publishes a new release thirty seconds in.
write_core_config() {
	mkdir -p "$HOME/.quiver"
	cat >"$HOME/.quiver/config.yaml" <<'EOF'
config:
  arrows:
    version_check_ttl: 1s
EOF
}

# --- launching -------------------------------------------------------------

# start_headless_daemon BINARY -- "core installed headless": the binary on
# PATH, `quiver daemon` in the background, nothing else.
start_headless_daemon() {
	local bin="$1"
	install -m 0755 "$bin" /usr/local/bin/quiver
	nohup /usr/local/bin/quiver daemon >"$SCENARIO_DIR/daemon.log" 2>&1 &
	wait_for_daemon 45
	ok "headless quiver daemon is up (pid $(daemon_pid), $(daemon_version))"
}

# launch_desktop PATH -- starts the GUI on the real X display, detached from
# this script's pipes.
#
# setsid + full redirection is not cosmetic. A child holding this script's
# stdout keeps the harness's own readers open, and the SIGPIPE hazard the
# ledger flags (a spawned sidecar dying when its reader goes away) is exactly
# what this scenario is meant to observe rather than cause.
launch_desktop() {
	local bin="$1" logname="${2:-desktop}"
	setsid env DISPLAY="$DISPLAY" \
		nohup "$bin" >"$SCENARIO_DIR/${logname}.log" 2>&1 </dev/null &
	info "launched $bin on $DISPLAY"
}

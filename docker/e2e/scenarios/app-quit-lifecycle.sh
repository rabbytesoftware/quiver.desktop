#!/usr/bin/env bash
# What happens to running arrows when the Quiver app quits or dies, and
# whether a relaunch recovers cleanly. Three arrows run each time: quiver.chat
# (a `listen` ui), E2E Static App 1 (a `static` ui kept open by `sleep 3000`)
# and E2E Supervised (a plain `sleep 3000` run with no ui), the comparison.
#
#   docker compose -f docker/e2e/docker-compose.yml run -d e2e bash scenarios/app-quit-lifecycle.sh [CASE...]
#
# Cases: close-window, sigterm-app, sigkill-app, sigkill-daemon,
# sigkill-daemon-arrows-die (the daemon and every arrow process killed while
# the app stays up). Default: all.
# QUIT_SETTLE=N waits N seconds after every arrow is ready before acting.
# Every case starts from a clean home. Observations go to
# results/app-quit/<case>.txt; nothing here asserts, it records.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

OUT="$RESULTS/app-quit"
mkdir -p "$OUT"
SCENARIO_DIR="$OUT"
APP_DIR="$RUN_DIR/quit-app"
RUN_SOCKETS="$HOME/.quiver/run"
CLI="$HOME/.quiver/self/quiver"
CASES=("$@")
[ "${#CASES[@]}" -gt 0 ] || CASES=(close-window sigterm-app sigkill-app sigkill-daemon sigkill-daemon-arrows-die)

declare -A NS=(
	[chat]=github.com/rabbytesoftware/quiver.chat
	[static]=github.com/rabbytesoftware/e2e-static-app-1
	[plain]=github.com/rabbytesoftware/e2e-supervised
)
declare -A IDENT=()
declare -A TREE=()

kill_arrow_processes() {
	pkill -f 'quiver-chat-linux-|server\.py |^sleep 3000$' 2>/dev/null || true
}

seed_upstream() {
	bash "$E2E_DIR/fixtures/upstream-up.sh" >/dev/null || fail "upstream did not come up"
	reset_upstream >/dev/null
	publish_core_asset "$CORE_V1" >/dev/null
	publish_manifest rabbytesoftware/quiver.core "$CORE_V1" /workspace/build/src/quiver.core/ARROW.md >/dev/null
	mark_latest rabbytesoftware/quiver.core "$CORE_V1" >/dev/null
	git_tag rabbytesoftware/quiver.core "$CORE_V1" >/dev/null
	publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" /workspace/build/src/quiver.desktop/ARROW.md >/dev/null
	git_tag rabbytesoftware/quiver.desktop "$DESK_V1" >/dev/null
	publish_manifest rabbytesoftware/quiver.chat develop "$CHAT_CHECKOUT/ARROW.md" >/dev/null
	publish_release rabbytesoftware/quiver.chat nightly \
		"$CHAT_DIST/quiver-chat-linux-arm64.tar.gz" "$CHAT_DIST/quiver-chat-linux-amd64.tar.gz" >/dev/null
	publish_manifest rabbytesoftware/e2e-static-app-1 develop "$UPSTREAM_STATE/fixtures/e2e-static-app-1/ARROW.md" >/dev/null
	publish_manifest rabbytesoftware/e2e-supervised develop "$E2E_DIR/fixtures/arrows/e2e-supervised/ARROW.md" >/dev/null
}

app_pid() { pgrep -xo quiverdesktop 2>/dev/null || true; }

launch_app() {
	launch_desktop "$APP_DIR/quiverdesktop" "desktop-$1" >/dev/null
	wait_for_desktop_window 90 >/dev/null
	wait_for_daemon 90
	for _ in $(seq 1 120); do [ -x "$CLI" ] && break; sleep 0.5; done
}

# descendants PID -- PID and every process below it.
descendants() {
	local pid="$1" kid
	printf '%s\n' "$pid"
	for kid in $(pgrep -P "$pid" 2>/dev/null); do descendants "$kid"; done
}

runtime_json() { api_body GET "/v0/runtime/$(ns_enc "$1")" | jq -c '.data | {state, pid: .active_run.pid, surface: .active_run.surface, last: .last_return.method}' 2>/dev/null || echo '«unreachable»'; }

ui_status() { api_status GET "/v0/ui/$(ns_enc "$1")/"; }

snapshot() {
	local label="$1" key pid alive
	echo "--- $label ($(date -u +%T))"
	echo "app: $(app_pid || true)   daemons: $(list_core_daemons | tr '\n' ' ')"
	for key in chat static plain; do
		alive=""
		for pid in ${TREE[$key]:-}; do
			if kill -0 "$pid" 2>/dev/null; then alive+="$pid "; fi
		done
		echo "$key: tree [${TREE[$key]:-}] alive [${alive% }]"
		if daemon_alive; then
			echo "  runtime $(runtime_json "${IDENT[$key]}")"
			[ "$key" = plain ] || echo "  /v0/ui -> $(ui_status "${IDENT[$key]}")"
		fi
	done
	echo "run dir: $(ls -la "$RUN_SOCKETS" 2>/dev/null | awk 'NR>1 {print $1, $NF}' | tr '\n' ';')"
	echo "arrow processes: $(pgrep -af 'quiver-chat-linux-|^sleep 3000$' | tr '\n' ';')"
}

setup_case() {
	kill_everything
	kill_arrow_processes
	rm -rf "$HOME/.quiver"
	mkdir -p "$HOME/.quiver"
	seed_upstream
	launch_app "$1-first"
	local key
	for key in chat static plain; do
		"$CLI" arrow add "${NS[$key]}" >/dev/null 2>&1 || fail "add ${NS[$key]}"
		IDENT[$key]="$(arrow_field "${NS[$key]}" '.data.namespace')"
		"$CLI" install "${IDENT[$key]}" >/dev/null 2>&1 || fail "install ${IDENT[$key]}"
		"$CLI" run "${IDENT[$key]}" --detach >/dev/null 2>&1 || fail "run ${IDENT[$key]}"
	done
	for key in chat static; do
		local deadline=$(( SECONDS + 60 ))
		until [ "$(arrow_field "${IDENT[$key]}" '.data.active_run.surface.ready')" = "true" ]; do
			[ "$SECONDS" -lt "$deadline" ] || fail "${IDENT[$key]} never ready"
			sleep 0.5
		done
	done
	wait_for_state "${IDENT[plain]}" running 60 >/dev/null
	for key in chat static plain; do
		TREE[$key]="$(descendants "$(arrow_field "${IDENT[$key]}" '.data.active_run.pid')" | tr '\n' ' ')"
	done
}

act() {
	case "$1" in
		close-window) wmctrl -c Quiver ;;
		sigterm-app) kill -TERM "$(app_pid)" ;;
		sigkill-app) kill -KILL "$(app_pid)" ;;
		sigkill-daemon) pkill -KILL -f "$DAEMON_PATTERN" ;;
		sigkill-daemon-arrows-die)
			pkill -KILL -f "$DAEMON_PATTERN"
			sleep 1
			local key pid
			for key in chat static plain; do
				for pid in ${TREE[$key]}; do kill -KILL "$pid" 2>/dev/null || true; done
			done
			;;
	esac
}

# restart_check KEY -- stops the arrow if the runtime still holds it, starts
# it again and reports what it gets.
restart_check() {
	local key="$1" id="${IDENT[$1]}" state
	state="$(runtime_state "$id")"
	if [ "$state" = running ] || [ "$state" = detached ]; then
		echo "  stop from $state -> HTTP $(api_status POST "/v0/runtime/$(ns_enc "$id")/stop" '{}')"
		wait_for_state "$id" ready 30 >/dev/null 2>&1 || echo "  did not settle to ready: $(runtime_json "$id")"
	fi
	echo "  execute -> HTTP $(api_status POST "/v0/runtime/$(ns_enc "$id")/execute" '{}')"
	local deadline=$(( SECONDS + 30 ))
	until [ "$(runtime_state "$id")" = running ]; do [ "$SECONDS" -lt "$deadline" ] || break; sleep 0.5; done
	if [ "$key" != plain ]; then
		deadline=$(( SECONDS + 30 ))
		until [ "$(arrow_field "$id" '.data.active_run.surface.ready')" = "true" ]; do [ "$SECONDS" -lt "$deadline" ] || break; sleep 0.5; done
		echo "  after restart: $(runtime_json "$id") /v0/ui -> $(ui_status "$id")"
	else
		echo "  after restart: $(runtime_json "$id")"
	fi
}

run_case() {
	local name="$1"
	say "CASE $name"
	{
		echo "case: $name"
		setup_case "$name"
		sleep "${QUIT_SETTLE:-0}"
		snapshot "before"
		act "$name"
		sleep 10
		snapshot "10 s after $name"
		scrot -o "$OUT/$name-after.png" 2>/dev/null || true
		if [ -z "$(app_pid)" ]; then
			launch_app "$name-relaunch"
			echo "relaunched the app"
		elif ! daemon_alive; then
			local deadline=$(( SECONDS + 30 ))
			until daemon_alive || [ "$SECONDS" -ge "$deadline" ]; do sleep 1; done
			if daemon_alive; then
				echo "the running app brought a daemon back"
			else
				echo "no daemon came back within 30 s while the app stayed up; closing the window and relaunching"
				wmctrl -c Quiver
				deadline=$(( SECONDS + 30 ))
				while [ -n "$(app_pid)" ] && [ "$SECONDS" -lt "$deadline" ]; do sleep 1; done
				echo "app after close: $(app_pid || true)"
				launch_app "$name-relaunch"
				echo "relaunched the app"
			fi
		fi
		sleep 8
		snapshot "after relaunch/recovery"
		scrot -o "$OUT/$name-recovered.png" 2>/dev/null || true
		echo "--- restart each arrow"
		local key
		for key in chat static plain; do
			echo "$key:"
			restart_check "$key"
		done
		echo "run dir after restarts: $(ls -la "$RUN_SOCKETS" 2>/dev/null | awk 'NR>1 {print $1, $NF}' | tr '\n' ';')"
		echo "arrow processes after restarts: $(pgrep -af 'quiver-chat-linux-|^sleep 3000$' | tr '\n' ';')"
		cp "$HOME/.quiver/logs/Quiver.log" "$OUT/$name-daemon.log" 2>/dev/null || true
	} >"$OUT/$name.txt" 2>&1
	cp "$OUT/$name.txt" "$OUT/$name.${QUIT_SETTLE:-0}s.$(date +%s).txt"
	cat "$OUT/$name.txt"
}

[ -f "$CHAT_CHECKOUT/ARROW.md" ] || fail "no quiver.chat checkout at $CHAT_CHECKOUT"
mkdir -p "$APP_DIR"
install -m 0755 "$BUILD_BIN/quiverdesktop-$DESK_V1" "$APP_DIR/quiverdesktop"
install -m 0755 "$BUILD_BIN/quiver-$CORE_V1" "$APP_DIR/quiver"
for c in "${CASES[@]}"; do
	for _ in $(seq 1 "${QUIT_REPEAT:-1}"); do run_case "$c"; done
done
kill_everything
kill_arrow_processes

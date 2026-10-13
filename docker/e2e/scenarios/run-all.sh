#!/usr/bin/env bash
# Builds both apps once, stands up the local upstream, then runs the three
# scenarios in order, each from a genuinely clean machine.
#
#   docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh
#   docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh 2 3
#   SKIP_BUILD=1 ... scenarios/run-all.sh
#
# Everything a run produces -- logs, screenshots, API snapshots, the upstream
# request log -- lands in docker/e2e/results/ on the host.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

WANTED=("$@")
run_wanted() {
	[ "${#WANTED[@]}" -eq 0 ] && return 0
	local want
	for want in "${WANTED[@]}"; do
		[ "$want" = "$1" ] && return 0
	done
	return 1
}

mkdir -p "$RESULTS"
: >"$RESULTS/upstream.log"
SUMMARY="$RESULTS/SUMMARY.txt"
: >"$SUMMARY"

record() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }

record "quiver.core <-> quiver.desktop E2E"
record "run started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
record "host:        $(uname -sm) in a container, display $DISPLAY"
record "target:      $TARGET_TRIPLE"
record ""

# --- build -----------------------------------------------------------------

if [ "${SKIP_BUILD:-0}" = "1" ] && [ -d "$BUILD_BIN" ]; then
	record "build: skipped (SKIP_BUILD=1)"
else
	say "Building both applications from source"
	bash "$HERE/build.sh" 2>&1 | tee "$RESULTS/build.log" | tail -5
	# shellcheck disable=SC2181
	[ "${PIPESTATUS[0]}" = "0" ] || { record "build: FAILED (see results/build.log)"; exit 1; }
	record "build: ok"
fi

for artifact in \
	"$BUILD_BIN/quiver-$CORE_V1" "$BUILD_BIN/quiver-$CORE_V2" \
	"$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1" "$BUILD_BIN/$DESKTOP_ASSET.$DESK_V2"
do
	[ -f "$artifact" ] || { record "missing build artifact: $artifact"; exit 1; }
	record "artifact: $(basename "$artifact")  $(stat -c%s "$artifact") bytes  sha256=$(sha256_of "$artifact")"
done
record ""

# --- upstream --------------------------------------------------------------

say "Standing up the local GitHub stand-in"
bash "$E2E_DIR/fixtures/upstream-up.sh" 2>&1 | tee "$RESULTS/upstream-up.log"
[ "${PIPESTATUS[0]}" = "0" ] || { record "upstream: FAILED"; exit 1; }
record "upstream: up (https://github.com resolves to the local fixture)"
record ""

# --- scenarios -------------------------------------------------------------

declare -a FAILED=()
declare -a PASSED=()

run_scenario() {
	local number="$1" script="$2"
	run_wanted "$number" || { record "scenario $number: skipped"; return 0; }
	if bash "$HERE/$script" 2>&1 | tee "$RESULTS/scenario-$number.log"; then
		PASSED+=("$number")
		record "scenario $number: PASS"
	else
		FAILED+=("$number")
		record "scenario $number: FAIL (see results/scenario-$number.log)"
	fi
}

run_scenario 1 scenario-1-core-self-update.sh
run_scenario 2 scenario-2-sideload.sh
run_scenario 3 scenario-3-desktop-update.sh

# --- summary ---------------------------------------------------------------

kill_everything

record ""
record "passed: ${PASSED[*]:-none}"
record "failed: ${FAILED[*]:-none}"
record "screenshots:"
find "$RESULTS" -name '*.png' -printf '  %p (%s bytes)\n' 2>/dev/null | sort | tee -a "$SUMMARY"
record "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo
cat "$SUMMARY"

[ "${#FAILED[@]}" -eq 0 ]

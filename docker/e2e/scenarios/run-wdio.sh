#!/usr/bin/env bash
# Runs the tauri-driver + WebdriverIO specs under e2e/ in this box: the real
# release-built app on the box's X display, driven through WebKitWebDriver,
# against a real quiver.core daemon and the GitHub stand-in the shell
# scenarios use.
#
#   docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-wdio.sh
#   docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-wdio.sh update-and-restart
#   SKIP_BUILD=1 ... scenarios/run-wdio.sh update-and-restart
#
# Arguments are spec names (the file name without .spec.ts); none runs them all.
# Every spec starts from a wiped home and a freshly seeded upstream, so they
# cannot lean on one another.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

CORE_SRC=/workspace/build/src/quiver.core
DESK_SRC=/workspace/build/src/quiver.desktop
APP_DIR="$RUN_DIR/wdio-app"
WDIO_RESULTS="$RESULTS/wdio"
SPEC_HOMES=/tmp/quiver-e2e

ALL_SPECS=(bootstrap generic-outdated-badge self-update-while-running update-and-restart)
if [ "$#" -gt 0 ]; then SPECS=("$@"); else SPECS=("${ALL_SPECS[@]}"); fi

case "$(uname -m)" in
	aarch64) CORE_ARCH=arm64 ;;
	x86_64)  CORE_ARCH=amd64 ;;
	*)       CORE_ARCH="$(uname -m)" ;;
esac
# The name a quiver.core release asset carries for this machine, which the
# daemon's own asset picker has to recognise.
CORE_ASSET="quiver-linux-$CORE_ARCH"

mkdir -p "$RESULTS" "$WDIO_RESULTS"
SUMMARY="$WDIO_RESULTS/SUMMARY.txt"
: >"$SUMMARY"
record() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }

record "quiver.desktop WebDriver E2E"
record "run started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
record "host:        $(uname -sm) in a container, display $DISPLAY"
record "specs:       ${SPECS[*]}"
record ""

# --- build -----------------------------------------------------------------

needed=("$BUILD_BIN/quiver-$CORE_V1" "$BUILD_BIN/quiver-$CORE_V2" "$BUILD_BIN/quiver-$CORE_V3" "$BUILD_BIN/quiver-$CORE_V4" "$BUILD_BIN/quiverdesktop-$DESK_V1")
missing=0
for artifact in "${needed[@]}"; do [ -f "$artifact" ] || missing=1; done

if [ "${SKIP_BUILD:-0}" = "1" ] && [ "$missing" = "0" ]; then
	record "build: skipped (SKIP_BUILD=1)"
else
	say "Building quiver.core (four versions) and quiver.desktop from source"
	WDIO_ONLY=1 bash "$HERE/build.sh" 2>&1 | tee "$WDIO_RESULTS/build.log" | tail -5
	[ "${PIPESTATUS[0]}" = "0" ] || { record "build: FAILED (see results/wdio/build.log)"; exit 1; }
	record "build: ok"
fi
for artifact in "${needed[@]}"; do
	[ -f "$artifact" ] || { record "missing build artifact: $artifact"; exit 1; }
	record "artifact: $(basename "$artifact")  $(stat -c%s "$artifact") bytes  sha256=$(sha256_of "$artifact")"
done
record ""

# --- the harness -----------------------------------------------------------

# Only e2e/ is refreshed from the bind mount: the sources build.sh synced are
# what the app under test was built from, and node_modules stays in the build
# volume rather than in the host checkout.
say "Syncing e2e/ into the build volume and installing its dependencies"
mkdir -p "$DESK_SRC/e2e"
tar -C /workspace/quiver.desktop/e2e -cf - --exclude=./node_modules --exclude=./logs . | tar -C "$DESK_SRC/e2e" -xf -
( cd "$DESK_SRC/e2e" && { bun install --frozen-lockfile || bun install; } ) >"$WDIO_RESULTS/e2e-install.log" 2>&1 \
	|| { record "e2e dependencies: FAILED (see results/wdio/e2e-install.log)"; exit 1; }
if ( cd "$DESK_SRC/e2e" && bun run typecheck ) >"$WDIO_RESULTS/e2e-typecheck.log" 2>&1; then
	record "harness typecheck: ok"
else
	record "harness typecheck: FAILED (see results/wdio/e2e-typecheck.log)"
	cat "$WDIO_RESULTS/e2e-typecheck.log"
	exit 1
fi

# --- the app under test ----------------------------------------------------

# The desktop binary with the oldest quiver.core beside it as its sidecar, the
# way a fresh install ships: the app starts the sidecar, which then installs
# itself under <QUIVER_HOME>/self and is the build every later update replaces.
mkdir -p "$APP_DIR"
install -m 0755 "$BUILD_BIN/quiverdesktop-$DESK_V1" "$APP_DIR/quiverdesktop"
install -m 0755 "$BUILD_BIN/quiver-$CORE_V1" "$APP_DIR/quiver"

# --- upstream --------------------------------------------------------------

say "Standing up the local GitHub stand-in"
: >"$RESULTS/upstream.log"
bash "$E2E_DIR/fixtures/upstream-up.sh" 2>&1 | tee "$RESULTS/upstream-up.log"
[ "${PIPESTATUS[0]}" = "0" ] || { record "upstream: FAILED"; exit 1; }

ASSETS="$RUN_DIR/wdio-assets"
for tag in "$CORE_V1" "$CORE_V2" "$CORE_V3" "$CORE_V4"; do
	mkdir -p "$ASSETS/$tag"
	install -m 0755 "$BUILD_BIN/quiver-$tag" "$ASSETS/$tag/$CORE_ASSET"
done

publish_core_release() {
	local tag="$1"
	publish_manifest rabbytesoftware/quiver.core "$tag" "$CORE_SRC/ARROW.md"
	publish_release rabbytesoftware/quiver.core "$tag" "$ASSETS/$tag/$CORE_ASSET"
	mark_latest rabbytesoftware/quiver.core "$tag"
	git_tag rabbytesoftware/quiver.core "$tag"
}

# What every spec starts from: quiver.core and quiver.desktop each with one
# release, and the two small arrows the specs add to the library.
seed_baseline() {
	reset_upstream
	publish_core_release "$CORE_V1"

	publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" "$DESK_SRC/ARROW.md"
	git_tag rabbytesoftware/quiver.desktop "$DESK_V1"

	local fixture
	for fixture in e2e-supervised e2e-required; do
		publish_manifest "rabbytesoftware/$fixture" develop "$E2E_DIR/fixtures/arrows/$fixture/ARROW.md"
	done
	publish_manifest rabbytesoftware/e2e-required stable-1.0 "$E2E_DIR/fixtures/arrows/e2e-required/ARROW.md"
	git_tag rabbytesoftware/e2e-required stable-1.0
}

# --- run -------------------------------------------------------------------

export QUIVER_E2E_APP_BINARY="$APP_DIR/quiverdesktop"
export QUIVER_E2E_TMP="$SPEC_HOMES"
export QUIVER_E2E_BUILD_CHANNEL=stable
export QUIVER_E2E_FIXTURE_NS="github.com/rabbytesoftware/e2e-supervised"
export QUIVER_E2E_REQUIRED_NS="github.com/rabbytesoftware/e2e-required"
export QUIVER_E2E_VERSION_CHECK_TTL=1s
export QUIVER_E2E_RESULTS="$WDIO_RESULTS"
export QUIVER_E2E_UPSTREAM_STATE="$UPSTREAM_STATE"
export QUIVER_E2E_UPSTREAM_LOG="$RESULTS/upstream.log"
export QUIVER_E2E_CORE_MANIFEST="$CORE_SRC/ARROW.md"
export QUIVER_E2E_CORE_TAGS="$CORE_V1 $CORE_V2 $CORE_V3 $CORE_V4 $CORE_V5"
export QUIVER_E2E_CORE_ASSETS="$ASSETS"
export QUIVER_E2E_CORE_ASSET="$CORE_ASSET"

declare -a PASSED=() FAILED=()

run_spec() {
	local name="$1"
	say "SPEC $name"
	kill_everything
	rm -rf "$SPEC_HOMES/$name" "$WDIO_RESULTS/$name"
	mkdir -p "$WDIO_RESULTS/$name"
	seed_baseline
	[ "$name" = "generic-outdated-badge" ] && publish_core_release "$CORE_V2"
	: >"$RESULTS/upstream.log"

	( cd "$DESK_SRC/e2e" && ./node_modules/.bin/wdio run ./wdio.conf.ts --spec "./scenarios/$name.spec.ts" ) \
		2>&1 | tee "$WDIO_RESULTS/$name.log"
	local code="${PIPESTATUS[0]}"

	cp -r "$SPEC_HOMES/$name/.quiver/logs" "$WDIO_RESULTS/$name/daemon-logs" 2>/dev/null || true
	cp "$RESULTS/upstream.log" "$WDIO_RESULTS/$name/upstream.log" 2>/dev/null || true
	scrot -o "$WDIO_RESULTS/$name/final-screen.png" 2>/dev/null || true
	kill_everything

	if [ "$code" = "0" ]; then
		PASSED+=("$name"); record "spec $name: PASS"
	else
		FAILED+=("$name"); record "spec $name: FAIL (exit $code, see results/wdio/$name.log)"
	fi
}

for spec in "${SPECS[@]}"; do
	[ -f "$DESK_SRC/e2e/scenarios/$spec.spec.ts" ] || { record "spec $spec: no such file"; FAILED+=("$spec"); continue; }
	run_spec "$spec"
done

record ""
record "passed: ${PASSED[*]:-none}"
record "failed: ${FAILED[*]:-none}"
record "screenshots:"
find "$WDIO_RESULTS" -name '*.png' -printf '  %p (%s bytes)\n' 2>/dev/null | sort | tee -a "$SUMMARY"
record "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

[ "${#FAILED[@]}" -eq 0 ]

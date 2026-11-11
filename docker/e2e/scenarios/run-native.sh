#!/usr/bin/env bash
# Runs the native end-to-end scenarios (`cargo test -p quiver-e2e --test scenarios`) in this box: the release-profile
# app built WITH the e2e bridge, on the box's X display (Xvfb + openbox, Mesa lavapipe), against a real quiver.core
# daemon built from source and the GitHub stand-in the shell scenarios use.
#
#   make test-e2e-box                         # all scenarios
#   make test-e2e-box TESTS="update_core qol" # only tests whose name contains one of these
#   SKIP_BUILD=1 docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-native.sh
#
# Every test starts from a wiped home and a freshly seeded upstream, so they cannot lean on one another: the list
# of tests comes from `--list` and each runs on its own (`--exact`).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

CORE_SRC=/workspace/build/src/quiver.core
DESK_SRC=/workspace/quiver.desktop
APP_DIR="$RUN_DIR/native-app"
NATIVE_RESULTS="$RESULTS/native"
SPEC_HOMES=/tmp/quiver-e2e
FILTERS=("$@")

case "$(uname -m)" in
	aarch64) CORE_ARCH=arm64 ;;
	x86_64)  CORE_ARCH=amd64 ;;
	*)       CORE_ARCH="$(uname -m)" ;;
esac
# The name a quiver.core release asset carries for this machine, which the daemon's own asset picker has to recognise.
CORE_ASSET="quiver-linux-$CORE_ARCH"

mkdir -p "$RESULTS" "$NATIVE_RESULTS"
SUMMARY="$NATIVE_RESULTS/SUMMARY.txt"
: >"$SUMMARY"
record() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }

record "quiver.desktop native E2E"
record "run started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
record "host:        $(uname -sm) in a container, display $DISPLAY"
record ""

# --- build -----------------------------------------------------------------

needed=("$BUILD_BIN/quiver-$CORE_V1" "$BUILD_BIN/quiver-$CORE_V2" "$BUILD_BIN/quiver-$CORE_V3" "$BUILD_BIN/quiver-$CORE_V4"
	"$BUILD_BIN/quiverdesktop-$DESK_V1")
missing=0
for artifact in "${needed[@]}"; do [ -f "$artifact" ] || missing=1; done

if [ "${SKIP_BUILD:-0}" = "1" ] && [ "$missing" = "0" ]; then
	record "build: skipped (SKIP_BUILD=1)"
else
	say "Building quiver.core (four versions) and the app (with the e2e bridge) from source"
	bash "$HERE/build.sh" 2>&1 | tee "$NATIVE_RESULTS/build.log" | tail -5
	[ "${PIPESTATUS[0]}" = "0" ] || { record "build: FAILED (see results/native/build.log)"; exit 1; }
	record "build: ok"
fi
for artifact in "${needed[@]}"; do
	[ -f "$artifact" ] || { record "missing build artifact: $artifact"; exit 1; }
	record "artifact: $(basename "$artifact")  $(stat -c%s "$artifact") bytes  sha256=$(sha256_of "$artifact")"
done
record ""

# --- the app under test ----------------------------------------------------

# The app with the oldest quiver.core beside it as its sidecar, laid out the way a fresh install ships (the names
# `cargo xtask package` gives a bundle): the app starts the sidecar, which then installs itself under
# <QUIVER_HOME>/self and is the build every later update replaces.
mkdir -p "$APP_DIR"
install -m 0755 "$BUILD_BIN/quiverdesktop-$DESK_V1" "$APP_DIR/quiverdesktop"
install -m 0755 "$BUILD_BIN/quiver-$CORE_V1" "$APP_DIR/quiver"

# --- upstream --------------------------------------------------------------

# `make play-box-online` keeps the real github.com: no stand-in, no hosts entries, no local CA.
if [ "${PLAYGROUND:-0}" != "online" ]; then
	say "Standing up the local GitHub stand-in"
	: >"$RESULTS/upstream.log"
	bash "$E2E_DIR/fixtures/upstream-up.sh" 2>&1 | tee "$RESULTS/upstream-up.log"
	[ "${PIPESTATUS[0]}" = "0" ] || { record "upstream: FAILED"; exit 1; }
fi

ASSETS="$RUN_DIR/native-assets"
for tag in "$CORE_V1" "$CORE_V2" "$CORE_V3" "$CORE_V4"; do
	mkdir -p "$ASSETS/$tag"
	install -m 0755 "$BUILD_BIN/quiver-$tag" "$ASSETS/$tag/$CORE_ASSET"
done

publish_core_release() {
	local tag="$1"
	publish_manifest rabbytesoftware/quiver.core "$tag" "$CORE_SRC/ARROW.md"
	publish_release rabbytesoftware/quiver.core "$tag" "$ASSETS/$tag/$CORE_ASSET"
	printf '%s  ./%s\n' "$(sha256_of "$ASSETS/$tag/$CORE_ASSET")" "$CORE_ASSET" \
		>"$UPSTREAM_STATE/releases/rabbytesoftware/quiver.core/$tag/checksums.txt"
	mark_latest rabbytesoftware/quiver.core "$tag"
	git_tag rabbytesoftware/quiver.core "$tag"
}

# What every test starts from: quiver.core and quiver.desktop each with one release, and the two small arrows the
# tests add to the library.
seed_baseline() {
	reset_upstream
	publish_core_release "$CORE_V1"

	publish_manifest rabbytesoftware/quiver.desktop "$DESK_V1" "$DESK_SRC/ARROW.md"
	git_tag rabbytesoftware/quiver.desktop "$DESK_V1"

	local fixture
	for fixture in e2e-supervised e2e-required e2e-slow e2e-form; do
		publish_manifest "rabbytesoftware/$fixture" develop "$E2E_DIR/fixtures/arrows/$fixture/ARROW.md"
	done
	publish_manifest rabbytesoftware/e2e-updatable develop "$E2E_DIR/fixtures/arrows/e2e-updatable/ARROW.md"
	publish_manifest rabbytesoftware/e2e-updatable stable-1.0 "$E2E_DIR/fixtures/arrows/e2e-updatable/ARROW.md"
	git_tag rabbytesoftware/e2e-updatable stable-1.0
	publish_manifest rabbytesoftware/e2e-required stable-1.0 "$E2E_DIR/fixtures/arrows/e2e-required/ARROW.md"
	git_tag rabbytesoftware/e2e-required stable-1.0
}

# What the arrow-apps tests add: quiver.chat's own ARROW.md, unmodified, with the two linux archives it fetches from
# the `nightly` release; the echo app and its server; the short app; and the three rendered static apps
# (fixtures/upstream-up.sh).
seed_arrow_apps() {
	# quiver.chat is optional: without its checkout the chat test skips itself (QUIVER_E2E_CHAT_MANIFEST names a
	# file that does not exist) and only the repository's own fixture apps run.
	if [ -f "$CHAT_CHECKOUT/ARROW.md" ] && [ -d "$CHAT_DIST" ]; then
		publish_manifest rabbytesoftware/quiver.chat develop "$CHAT_CHECKOUT/ARROW.md"
		# The ARROW.md fetches releases/download/${REF}/..., so the archives sit under every ref a test installs from
		# (develop) as well as the rolling nightly.
		local ref
		for ref in nightly develop; do
			publish_release rabbytesoftware/quiver.chat "$ref" \
				"$CHAT_DIST/quiver-chat-linux-arm64.tar.gz" "$CHAT_DIST/quiver-chat-linux-amd64.tar.gz"
		done
	else
		info "quiver.chat: no checkout at $CHAT_CHECKOUT, its test will skip"
	fi
	publish_manifest rabbytesoftware/e2e-echo-app develop "$E2E_DIR/fixtures/arrows/e2e-echo-app/ARROW.md"
	publish_release rabbytesoftware/e2e-echo-app v1 "$E2E_DIR/fixtures/arrows/e2e-echo-app/server.py"
	publish_manifest rabbytesoftware/e2e-short-app develop "$E2E_DIR/fixtures/arrows/e2e-short-app/ARROW.md"
	local n
	for n in 1 2 3 4; do
		publish_manifest "rabbytesoftware/e2e-static-app-$n" develop "$UPSTREAM_STATE/fixtures/e2e-static-app-$n/ARROW.md"
	done
}

# --- the environment the tests read ------------------------------------------
# The SAME names run-wdio.sh exported to the WebDriver specs.

export QUIVER_E2E_APP_BINARY="$APP_DIR/quiverdesktop"
export QUIVER_E2E_TMP="$SPEC_HOMES"
export QUIVER_E2E_BUILD_CHANNEL=stable
export QUIVER_E2E_FIXTURE_NS="github.com/rabbytesoftware/e2e-supervised"
export QUIVER_E2E_REQUIRED_NS="github.com/rabbytesoftware/e2e-required"
export QUIVER_E2E_UPDATABLE_NS="github.com/rabbytesoftware/e2e-updatable"
export QUIVER_E2E_VERSION_CHECK_TTL=1s
export QUIVER_E2E_RESULTS="$NATIVE_RESULTS"
export QUIVER_E2E_UPSTREAM_STATE="$UPSTREAM_STATE"
export QUIVER_E2E_UPSTREAM_LOG="$RESULTS/upstream.log"
export QUIVER_E2E_CORE_MANIFEST="$CORE_SRC/ARROW.md"
export QUIVER_E2E_CORE_TAGS="$CORE_V1 $CORE_V2 $CORE_V3 $CORE_V4 $CORE_V5"
export QUIVER_E2E_CORE_ASSETS="$ASSETS"
export QUIVER_E2E_CORE_ASSET="$CORE_ASSET"
export QUIVER_E2E_CHAT_NS="github.com/rabbytesoftware/quiver.chat"
export QUIVER_E2E_CHAT_MANIFEST="$CHAT_CHECKOUT/ARROW.md"
export QUIVER_E2E_ECHO_NS="github.com/rabbytesoftware/e2e-echo-app"
export QUIVER_E2E_SHORT_NS="github.com/rabbytesoftware/e2e-short-app"
export QUIVER_E2E_STATIC_NS="github.com/rabbytesoftware/e2e-static-app-1 github.com/rabbytesoftware/e2e-static-app-2 github.com/rabbytesoftware/e2e-static-app-3 github.com/rabbytesoftware/e2e-static-app-4"
export QUIVER_E2E_TCP_PORT=40299
export CARGO_TARGET_DIR="$CARGO_TARGET"

# A desktop session has a Secret Service (the OS keyring the app keeps remote pairing tokens in: pairing with a
# remote fails without one, "Secret Service: no result found"). The box's bare Xvfb has none, so give the app's
# environment a session bus and an unlocked default keyring, the way a logged-in desktop would. The default
# collection is a plain-text (unencrypted) `login` keyring written ahead of time: creating one through the
# Secret Service API asks for a password in a GUI prompt, which nobody is there to answer.
start_secret_service() {
	command -v gnome-keyring-daemon >/dev/null || return 0
	mkdir -p "$HOME/.local/share/keyrings"
	printf '[keyring]\ndisplay-name=Login\nctime=0\nmtime=0\nlock-on-idle=false\nlock-after=false\n' \
		>"$HOME/.local/share/keyrings/login.keyring"
	printf login >"$HOME/.local/share/keyrings/default"
	if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
		eval "$(dbus-launch --sh-syntax)"
		export DBUS_SESSION_BUS_ADDRESS
	fi
	eval "$(printf '' | gnome-keyring-daemon --unlock --components=secrets 2>/dev/null)"
	export GNOME_KEYRING_CONTROL SSH_AUTH_SOCK
}
start_secret_service

# `make play-box`: no tests. Seeds the stand-in with the baseline and every fixture arrow, starts the app the way a
# user would (an installed bundle, a fresh home, the real daemon), and keeps the box up so it can be used by hand
# through noVNC (http://localhost:6080/vnc.html). `relaunch` restarts the app on a fresh home.
play() {
	local play_home=/tmp/quiver-play
	seed_baseline
	seed_arrow_apps
	cat >/usr/local/bin/relaunch <<RELAUNCH
#!/usr/bin/env bash
pkill -x quiverdesktop 2>/dev/null; pkill -f 'quiver[^ ]* daemon' 2>/dev/null; sleep 2
[ "\$1" = "--fresh" ] && rm -rf $play_home
mkdir -p $play_home
cd $play_home
HOME=$play_home QUIVER_HOME=$play_home/.quiver nohup $APP_DIR/quiverdesktop >$play_home.log 2>&1 &
echo "started; log: $play_home.log"
RELAUNCH
	chmod +x /usr/local/bin/relaunch
	relaunch --fresh
	record "playground up: open http://localhost:${E2E_NOVNC_PORT:-6080}/vnc.html"
	record "  relaunch [--fresh]   restart the app (fresh = wipe its home)"
	record "  upstream stand-in serves: quiver.core, quiver.desktop, e2e-* arrows (echo app, static apps, supervised, form...)"
	exec sleep infinity
}
[ "${PLAYGROUND:-0}" = "1" ] && play

# `make play-box-online`: the same hand-driven box, but on the real internet. Nothing is seeded: the app talks to
# the real github.com, so any arrow published there (and runnable on linux) can be searched for and installed.
play_online() {
	local play_home=/tmp/quiver-play
	if ! curl -fsS -o /dev/null --max-time 10 https://api.github.com/zen; then
		record "WARNING: github.com is not reachable from the box; searching and installing will fail"
	fi
	# The sidecar is the latest published quiver.core nightly, not the checkout built for the tests (which can be
	# weeks old): the release asset, checked against the release's own checksums.txt.
	local base="https://github.com/rabbytesoftware/quiver.core/releases/download/${CORE_PLAY_TAG:-nightly-latest}"
	local fetched=/tmp/quiver-core-nightly
	mkdir -p "$fetched"
	if curl -fsSL --max-time 120 -o "$fetched/$CORE_ASSET" "$base/$CORE_ASSET" \
		&& curl -fsSL --max-time 30 -o "$fetched/checksums.txt" "$base/checksums.txt" \
		&& (cd "$fetched" && grep " \./$CORE_ASSET\$\| $CORE_ASSET\$" checksums.txt | sed 's# \./# #' | sha256sum -c -); then
		install -m 0755 "$fetched/$CORE_ASSET" "$APP_DIR/quiver"
		record "quiver.core: $("$APP_DIR/quiver" version --short 2>/dev/null || "$APP_DIR/quiver" --version 2>/dev/null | head -1) (${CORE_PLAY_TAG:-nightly-latest}, sha256 checked)"
	else
		record "WARNING: could not fetch quiver.core ${CORE_PLAY_TAG:-nightly-latest}; keeping the locally built $CORE_V1"
	fi
	cat >/usr/local/bin/relaunch <<RELAUNCH
#!/usr/bin/env bash
pkill -x quiverdesktop 2>/dev/null; pkill -f 'quiver[^ ]* daemon' 2>/dev/null; sleep 2
[ "\$1" = "--fresh" ] && rm -rf $play_home
mkdir -p $play_home
cd $play_home
HOME=$play_home QUIVER_HOME=$play_home/.quiver nohup $APP_DIR/quiverdesktop >$play_home.log 2>&1 &
echo "started; log: $play_home.log"
RELAUNCH
	chmod +x /usr/local/bin/relaunch
	relaunch --fresh
	record "online playground up: open http://<this host>:${E2E_NOVNC_PORT:-6080}/vnc.html"
	record "  real internet, real github.com: any linux-compatible arrow can be installed"
	record "  relaunch [--fresh]   restart the app (fresh = wipe its home)"
	exec sleep infinity
}
[ "${PLAYGROUND:-0}" = "online" ] && play_online

declare -a PASSED=() FAILED=()

# Arrow processes a killed daemon leaves behind: the arrow-apps fixtures and the supervised fixture's sleep.
kill_arrow_processes() {
	pkill -f 'quiver-chat-linux-|server\.py |^sleep 3000$' 2>/dev/null || true
}

# Test names, from the test binary itself. A name containing a filter passed on the command line selects it.
list_tests() {
	( cd "$DESK_SRC" && cargo test -p quiver-e2e --test scenarios -- --list --format terse 2>/dev/null ) \
		| sed -n 's/: test$//p'
}

selected() {
	[ "${#FILTERS[@]}" -eq 0 ] && return 0
	local f
	for f in "${FILTERS[@]}"; do
		case "$1" in *"$f"*) return 0 ;; esac
	done
	return 1
}

run_test() {
	local name="$1" safe="${1//::/__}"
	say "TEST $name"
	kill_everything
	kill_arrow_processes
	rm -rf "$SPEC_HOMES" "${NATIVE_RESULTS:?}/$safe"
	mkdir -p "$NATIVE_RESULTS/$safe" "$SPEC_HOMES"
	seed_baseline
	case "$name" in *generic_outdated_badge*) publish_core_release "$CORE_V2" ;; esac
	case "$name" in *arrow_apps*) seed_arrow_apps ;; esac
	: >"$RESULTS/upstream.log"

	QUIVER_E2E_TEST_NAME="$name" \
		bash -c 'cd "$1" && exec cargo test -p quiver-e2e --test scenarios -- --exact --nocapture "$2"' _ "$DESK_SRC" "$name" \
		2>&1 | tee "$NATIVE_RESULTS/$safe.log"
	local code="${PIPESTATUS[0]}"

	cp -r "$SPEC_HOMES"/*/.quiver/logs "$NATIVE_RESULTS/$safe/daemon-logs" 2>/dev/null || true
	cp "$RESULTS/upstream.log" "$NATIVE_RESULTS/$safe/upstream.log" 2>/dev/null || true
	scrot -o "$NATIVE_RESULTS/$safe/final-screen.png" 2>/dev/null || true
	kill_everything
	kill_arrow_processes

	if [ "$code" = "0" ]; then
		PASSED+=("$name"); record "test $name: PASS"
	else
		FAILED+=("$name"); record "test $name: FAIL (exit $code, see results/native/$safe.log)"
	fi
}

mapfile -t ALL_TESTS < <(list_tests)
[ "${#ALL_TESTS[@]}" -gt 0 ] || { record "no tests listed by quiver-e2e's scenarios target"; exit 1; }
record "tests: ${#ALL_TESTS[@]} (filters: ${FILTERS[*]:-none})"
for t in "${ALL_TESTS[@]}"; do
	selected "$t" && run_test "$t"
done

record ""
record "passed: ${PASSED[*]:-none}"
record "failed: ${FAILED[*]:-none}"
record "screenshots:"
find "$NATIVE_RESULTS" -name '*.png' -printf '  %p (%s bytes)\n' 2>/dev/null | sort | tee -a "$SUMMARY"
record "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

[ "${#FAILED[@]}" -eq 0 ] && [ "${#PASSED[@]}" -gt 0 ]

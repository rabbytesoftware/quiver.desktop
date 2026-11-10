#!/usr/bin/env bash
# Builds every artifact the scenarios drive, once, into the container's own build volume.
#
# WHAT GETS BUILT, AND WHY SEVERAL CORES:
#
#   quiver.core   stable-26.5.90 to .93: v1 is what a scenario starts with, v2 is what "upstream" publishes
#                 mid-scenario so the self-update has somewhere real to go, v3/v4 are the releases after that,
#                 for scenarios that update more than once. Version, commit and channel are -ldflags stamps,
#                 exactly as the release workflows stamp them, because selfarrow.EnsureRegistered refuses to
#                 register an unstamped build at all and files a stamped one as quiver.core@stable.
#
#   quiver.desktop  the native app WITH the e2e bridge (`--features e2e-bridge`, the debug-only JSON-RPC control
#                 surface, see docs/e2e.md), stamped DESK_V1 through
#                 QUIVER_DESKTOP_RELEASE_TAG (what the running app announces itself under). It is installed under
#                 the names a real bundle has (quiverdesktop, quiver-arrow-window, sidecar quiver), see run-native.sh.
#                 The bridge binary is for this box only; releases are built without it (cargo xtask
#                 check-release-artifact fails on one that has it).
#
# RELEASE PROFILE, NOT DEBUG, and this is not an optimisation choice: a debug build takes the checkout-scoped
# QUIVER_HOME and a /tmp socket (cfg!(debug_assertions)), while the scenarios are about the app finding a daemon at
# the PRODUCTION address. LTO is switched off through the environment (not the manifest) because a fat-LTO link of
# a GPUI binary is minutes of CPU that buys the scenarios nothing.
#
# NOTHING IS WRITTEN INTO THE BIND-MOUNTED CHECKOUTS except Cargo.lock if cargo must touch it: the cargo target
# directory, the Go caches and the synced quiver.core sources live in named volumes.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

CORE_SRC=/workspace/build/src/quiver.core
DESK_SRC=/workspace/quiver.desktop

step() { echo; echo "=== $* ==="; }

# --- sources ---------------------------------------------------------------

sync_sources() {
	step "syncing quiver.core into the build volume"
	mkdir -p "$CORE_SRC"

	# Extracting over the previous run's copy keeps every file a newer checkout deleted or renamed, and a stale Go
	# file beside the current ones fails the build with errors in code nobody touched.
	find "$CORE_SRC" -mindepth 1 -delete

	tar -C /workspace/quiver.core -cf - \
		--exclude=./bin --exclude=./coverage --exclude=./.git \
		--exclude=./.quiver \
		. | tar -C "$CORE_SRC" -xf -
	echo "core: $(du -sh "$CORE_SRC" | cut -f1)"
}

# --- quiver.core -----------------------------------------------------------

build_core_at() {
	local version="$1" out="$2"
	echo "[core] building $version -> $out"
	# Stamped the way stable-release.yml stamps a release: the channel names
	# the row quiver.core files itself under (quiver.core@stable), the commit
	# is the one the stand-in repository's tags name (fixtures/seed-commit.sh
	# gives upstream-up.sh the same one). buildID is cosmetic here (it only
	# reaches `quiver version`), so it is pinned rather than derived from the
	# clock, to keep two runs of this script byte-comparable.
	local commit channel=stable
	case "$version" in nightly*) channel=nightly ;; esac
	commit="$("$E2E_DIR/fixtures/seed-commit.sh" "$(mktemp -d)" "$CORE_SRC/ARROW.md" develop)"
	( cd "$CORE_SRC" && CGO_ENABLED=0 go build \
		-ldflags "-X main.version=${version} -X main.commit=${commit} -X main.channel=${channel} -X main.buildID=9000" \
		-o "$out" ./cmd/quiver )
	"$out" version
}

build_core() {
	step "building quiver.core"
	mkdir -p "$BUILD_BIN"
	build_core_at "$CORE_V1" "$BUILD_BIN/quiver-$CORE_V1"
	build_core_at "$CORE_V2" "$BUILD_BIN/quiver-$CORE_V2"
	build_core_at "$CORE_V3" "$BUILD_BIN/quiver-$CORE_V3"
	build_core_at "$CORE_V4" "$BUILD_BIN/quiver-$CORE_V4"
}

# --- quiver.desktop --------------------------------------------------------

# Where cargo finds the app binary: `quiver` today, `quiverdesktop` once the bin is renamed to the installed name.
built_app_binary() {
	local dir="$CARGO_TARGET/release" name
	for name in quiverdesktop quiver; do
		[ -x "$dir/$name" ] && { printf '%s' "$dir/$name"; return 0; }
	done
	return 1
}

build_desktop_at() {
	local tag="$1" out="$2"
	echo "[desktop] building $tag -> $out"
	( cd "$DESK_SRC" && \
		CARGO_TARGET_DIR="$CARGO_TARGET" \
		CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 CARGO_PROFILE_RELEASE_STRIP=false \
		QUIVER_DESKTOP_RELEASE_TAG="$tag" \
		cargo build --release -p quiver-app --features e2e-bridge )
	local built
	built="$(built_app_binary)" || fail "cargo produced neither quiverdesktop nor quiver in $CARGO_TARGET/release"
	cp "$built" "$out"
	chmod +x "$out"
	grep -aq QUIVER_E2E_PORT "$out" || fail "$out was built without the e2e bridge: the scenarios cannot drive it"
}

build_desktop() {
	step "building quiver.desktop (with the e2e bridge)"
	mkdir -p "$BUILD_BIN"
	build_desktop_at "$DESK_V1" "$BUILD_BIN/quiverdesktop-$DESK_V1"
}

# The scenarios test binary, compiled here so `run-native.sh` only has to run it.
build_scenarios() {
	step "building the scenarios test target (quiver-e2e)"
	( cd "$DESK_SRC" && CARGO_TARGET_DIR="$CARGO_TARGET" cargo test -p quiver-e2e --test scenarios --no-run ) \
		|| fail "crates/quiver-e2e has no scenarios test target yet (crates/quiver-e2e/tests/scenarios.rs)"
}

# --- quiver.chat -----------------------------------------------------------

# The first arrow app, built from the checkout mounted at /workspace/quiver.chat
# (read-only) into the two release archives its ARROW.md fetches from the
# `nightly` release: quiver-chat-linux-<arm64|amd64>.tar.gz, each holding one
# binary named like the archive. These are the linux half of the chat
# Makefile's `release-archives`, which also cross-builds macOS and Windows and
# needs `zip`. The Go server embeds frontend/out, so a checkout that has not
# built its frontend gets it built here.
build_chat() {
	[ -d "$CHAT_CHECKOUT" ] || { echo "[chat] no checkout at $CHAT_CHECKOUT, skipped"; return 0; }
	step "building quiver.chat release archives"
	mkdir -p "$CHAT_SRC"
	find "$CHAT_SRC" -mindepth 1 -maxdepth 1 ! -name node_modules -exec rm -rf {} +
	tar -C "$CHAT_CHECKOUT" -cf - \
		--exclude=./.git --exclude=./dist --exclude=./frontend/node_modules \
		--exclude=./frontend/.next --exclude=./quiver-chat \
		. | tar -C "$CHAT_SRC" -xf -
	if [ ! -f "$CHAT_SRC/frontend/out/index.html" ]; then
		( cd "$CHAT_SRC/frontend" && npm ci && npm run build )
	fi
	mkdir -p "$CHAT_DIST"
	local arch
	for arch in arm64 amd64; do
		( cd "$CHAT_SRC" && CGO_ENABLED=0 GOOS=linux GOARCH="$arch" \
			go build -ldflags="-s -w" -o "$CHAT_DIST/quiver-chat-linux-$arch" . )
		tar -C "$CHAT_DIST" -czf "$CHAT_DIST/quiver-chat-linux-$arch.tar.gz" "quiver-chat-linux-$arch"
	done
	ls -la "$CHAT_DIST"
}

# `build.sh chat` builds only the quiver.chat archives.
main() {
	if [ "${1:-}" = chat ]; then
		build_chat
		return
	fi
	sync_sources
	build_core
	[ "${CORE_ONLY:-0}" = "1" ] && { step "build complete (core only)"; return; }
	build_chat
	build_desktop
	build_scenarios
	step "build complete"
}

main "$@"

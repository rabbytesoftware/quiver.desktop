#!/usr/bin/env bash
# Builds every artifact the three scenarios drive, once, into the container's
# own build volume.
#
# NOTHING IS WRITTEN INTO EITHER BIND-MOUNTED CHECKOUT. Both are macOS
# checkouts with their own node_modules, target/ and bin/ already populated
# for darwin; a Linux `bun install` or `cargo build` landing in them would
# replace darwin-native packages with linux ones and break the host's own dev
# loop. Sources are copied into /workspace/build first and everything happens
# there.
#
# WHAT GETS BUILT, AND WHY TWO OF EACH:
#
#   quiver.core   stable-26.5.90 to .93: v1 is what a scenario
#                 starts with, v2 is what "upstream" publishes mid-scenario
#                 so the self-update has somewhere real to go, and v3 is the
#                 releases after that, for scenarios that update more than once. Version,
#                 commit and channel are -ldflags stamps, exactly as the
#                 release workflows stamp them, because
#                 selfarrow.EnsureRegistered refuses to register an unstamped
#                 build at all and files a stamped one as quiver.core@stable.
#
#   quiver.desktop stable-1.0 and stable-1.1 -- same idea. The tag is baked in
#                 by src-tauri/build.rs from QUIVER_DESKTOP_RELEASE_TAG, and
#                 it is the thing the running app announces itself under, so
#                 two builds is the only honest way to have "the user is
#                 running 1.0 and 1.1 now exists".
#
# RELEASE PROFILE, NOT DEBUG, and this is not an optimisation choice. A debug
# build takes `dev_quiver_home()`/`dev_socket_override()`
# (src-tauri/src/connection/local/mod.rs, both gated on
# cfg!(debug_assertions)), which point the app at a checkout-scoped
# QUIVER_HOME and a /tmp socket. Scenario 2 is entirely about the app finding
# a headless daemon at the PRODUCTION address, so a debug build would prove
# nothing. `cargo build --release` gives that without paying for the AppImage
# bundler.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$HERE/lib.sh"

CORE_SRC=/workspace/build/src/quiver.core
DESK_SRC=/workspace/build/src/quiver.desktop

step() { echo; echo "=== $* ==="; }

# --- sources ---------------------------------------------------------------

sync_sources() {
	step "syncing sources into the build volume"
	mkdir -p "$CORE_SRC" "$DESK_SRC"

	# Extracting over the previous run's copy keeps every file a newer checkout
	# deleted or renamed, and a stale Go file beside the current ones fails the
	# build with errors in code nobody touched. Only node_modules survives: it
	# is what makes a rebuild fast, and it is keyed by the lockfile, not the tree.
	find "$CORE_SRC" -mindepth 1 -delete
	find "$DESK_SRC" -mindepth 1 -maxdepth 1 ! -name node_modules -exec rm -rf {} +

	tar -C /workspace/quiver.core -cf - \
		--exclude=./bin --exclude=./coverage --exclude=./.git \
		--exclude=./.quiver \
		. | tar -C "$CORE_SRC" -xf -

	tar -C /workspace/quiver.desktop -cf - \
		--exclude=./node_modules --exclude=./src-tauri/target \
		--exclude=./dist --exclude=./.git --exclude=./coverage \
		--exclude=./docker/e2e/results --exclude=./src-tauri/.quiver \
		. | tar -C "$DESK_SRC" -xf -

	echo "core:    $(du -sh "$CORE_SRC" | cut -f1)"
	echo "desktop: $(du -sh "$DESK_SRC" | cut -f1)"
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
	local commit
	commit="$("$E2E_DIR/fixtures/seed-commit.sh" "$(mktemp -d)" "$CORE_SRC/ARROW.md" develop)"
	( cd "$CORE_SRC" && CGO_ENABLED=0 go build \
		-ldflags "-X main.version=${version} -X main.commit=${commit} -X main.channel=stable -X main.buildID=9000" \
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

build_frontend() {
	step "building the quiver.desktop frontend"
	( cd "$DESK_SRC" && bun install --frozen-lockfile >/dev/null 2>&1 || bun install )
	( cd "$DESK_SRC" && bun run build )
	test -d "$DESK_SRC/dist" || fail "frontend build produced no dist/"
}

build_desktop_at() {
	local tag="$1" out="$2"
	echo "[desktop] building $tag -> $out"
	# The sidecar has to exist before tauri-build runs: it validates every
	# externalBin for the current target triple and fails the build if one is
	# missing. v1 of core is the bootstrap seed a real bundle would ship.
	mkdir -p "$DESK_SRC/src-tauri/binaries"
	cp "$BUILD_BIN/quiver-$CORE_V1" "$DESK_SRC/src-tauri/binaries/quiver-${TARGET_TRIPLE}"
	chmod +x "$DESK_SRC/src-tauri/binaries/quiver-${TARGET_TRIPLE}"

	# `tauri build --no-bundle`, NOT a bare `cargo build --release`, and this
	# is a correctness requirement rather than a preference.
	#
	# Whether a Tauri binary serves its embedded frontend or tries to load
	# tauri.conf.json's devUrl is decided by the `tauri` crate's
	# `custom-protocol` feature -- `dev = !has_feature("custom-protocol")` in
	# tauri's own build.rs, exported as DEP_TAURI_DEV and read by
	# tauri-build's is_dev(). The cargo PROFILE has nothing to do with it, so
	# `cargo build --release` produces a RELEASE binary that still points at
	# http://localhost:1420. This box caught that the loud way: the window
	# came up, wmctrl saw it, a screenshot was taken, and the screenshot said
	# "Could not connect to localhost: Connection refused". Every Rust-side
	# assertion still passed, because the Rust side was fine -- it was the
	# webview, and so the entire frontend including announceSelf, that never
	# ran.
	#
	# --no-bundle keeps this to one compile: a real production binary, no
	# AppImage/deb bundling, no appimagetool download, no FUSE.
	( cd "$DESK_SRC" && \
		CARGO_TARGET_DIR="$CARGO_TARGET" \
		QUIVER_DESKTOP_RELEASE_TAG="$tag" \
		bun run tauri build --no-bundle \
			--config '{"bundle":{"macOS":{"files":{}}}}' )

	cp "$CARGO_TARGET/release/quiverdesktop" "$out"
	chmod +x "$out"
}

build_desktop() {
	step "building quiver.desktop"
	build_desktop_at "$DESK_V1" "$BUILD_BIN/quiverdesktop-$DESK_V1"
	build_desktop_at "$DESK_V2" "$BUILD_BIN/quiverdesktop-$DESK_V2"
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

# --- artifacts -------------------------------------------------------------

package_artifacts() {
	step "packaging the release artifacts"
	"$HERE/../fixtures/pack-appimage.sh" \
		"$BUILD_BIN/quiverdesktop-$DESK_V1" \
		"$BUILD_BIN/quiver-$CORE_V1" \
		"$DESK_V1" \
		"$BUILD_BIN/$DESKTOP_ASSET.$DESK_V1"

	"$HERE/../fixtures/pack-appimage.sh" \
		"$BUILD_BIN/quiverdesktop-$DESK_V2" \
		"$BUILD_BIN/quiver-$CORE_V1" \
		"$DESK_V2" \
		"$BUILD_BIN/$DESKTOP_ASSET.$DESK_V2"

	ls -la "$BUILD_BIN"
}

# `build.sh chat` builds only the quiver.chat archives.
# WDIO_ONLY=1 builds what scenarios/run-wdio.sh drives: the three core builds
# and one desktop build, without the second desktop build and the AppImages the
# shell scenarios publish.
main() {
	if [ "${1:-}" = chat ]; then
		build_chat
		return
	fi
	sync_sources
	build_core
	build_chat
	build_frontend
	if [ "${WDIO_ONLY:-0}" = "1" ]; then
		step "building quiver.desktop (one build, for the WebDriver scenarios)"
		build_desktop_at "$DESK_V1" "$BUILD_BIN/quiverdesktop-$DESK_V1"
	else
		build_desktop
		package_artifacts
	fi
	step "build complete"
}

main "$@"

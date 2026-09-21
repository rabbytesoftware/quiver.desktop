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
#   quiver.core   stable-26.5.90 and stable-26.5.91 -- v1 is what a scenario
#                 starts with, v2 is what "upstream" publishes mid-scenario
#                 so the self-update has somewhere real to go. The version is
#                 an -ldflags stamp, exactly as the repo's own `make build`
#                 does it, because selfarrow.EnsureRegistered refuses to
#                 register an unstamped build at all.
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
	# buildID is cosmetic here (it only reaches `quiver version`), so it is
	# pinned rather than derived from the clock, to keep two runs of this
	# script byte-comparable.
	( cd "$CORE_SRC" && CGO_ENABLED=0 go build \
		-ldflags "-X main.version=${version} -X main.buildID=9000" \
		-o "$out" ./cmd/quiver )
	"$out" version
}

build_core() {
	step "building quiver.core"
	mkdir -p "$BUILD_BIN"
	build_core_at "$CORE_V1" "$BUILD_BIN/quiver-$CORE_V1"
	build_core_at "$CORE_V2" "$BUILD_BIN/quiver-$CORE_V2"
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

main() {
	sync_sources
	build_core
	build_frontend
	build_desktop
	package_artifacts
	step "build complete"
}

main "$@"

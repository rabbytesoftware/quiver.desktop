#!/usr/bin/env bash
# Packs a compiled quiver.desktop binary and its quiver.core sidecar into a
# single executable file, the way a real AppImage is a single executable file.
#
# WHAT THIS IS, PRECISELY, AND WHAT IT IS NOT.
#
# It is NOT a real AppImage. A real AppImage is a squashfs image behind a
# small ELF runtime that mounts itself through FUSE, and building one needs
# appimagetool and linuxdeploy (network downloads at bundle time) while
# RUNNING one needs /dev/fuse, which this container does not have.
#
# It IS faithful in the two respects the lifecycle under test depends on, and
# those are not accidents of this format -- they are what the manifest's steps
# actually touch:
#
#   1. ONE FILE, at the path the manifest places it. install/update fetch a
#      single asset, chmod +x it and mv it to
#      ${XDG_DATA_HOME:-$HOME/.local/share}/Quiver/Quiver.AppImage. A
#      self-extracting archive is a single chmod-able file exactly as an
#      AppImage is, so fetch -> checksum -> chmod -> place is byte-for-byte
#      the same code path.
#
#   2. THE PROCESS IT LEAVES BEHIND IS NAMED `quiverdesktop`. This is the one
#      that matters and the one a naive "just rename the binary
#      Quiver.AppImage" fixture gets WRONG: a process exec'd from a file named
#      Quiver.AppImage is called Quiver.AppImage, and `pkill -x quiverdesktop`
#      -- which the update and uninstall lifecycles both rely on -- would
#      never match it. A real AppImage does not behave that way either: Tauri's
#      generated AppRun ends in `exec "$APPDIR/usr/bin/quiverdesktop"`, so the
#      process a user sees carries the cargo binary name. This script
#      reproduces that with `exec`, so the pkill assumption is tested against
#      the process name a real AppImage would actually present.
#
# The sidecar sits next to the main binary in usr/bin for the same reason a
# real bundle puts it there: tauri-plugin-shell resolves a sidecar as
# dirname(current_exe)/<name>.
#
# Usage: pack-appimage.sh <desktop-binary> <core-binary> <build-id> <output>
set -euo pipefail

DESKTOP_BIN="$1"
CORE_BIN="$2"
BUILD_ID="$3"
OUT="$4"

[ -x "$DESKTOP_BIN" ] || { echo "not executable: $DESKTOP_BIN" >&2; exit 1; }
[ -x "$CORE_BIN" ] || { echo "not executable: $CORE_BIN" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/appdir/usr/bin"
cp "$DESKTOP_BIN" "$WORK/appdir/usr/bin/quiverdesktop"
cp "$CORE_BIN" "$WORK/appdir/usr/bin/quiver"
chmod +x "$WORK/appdir/usr/bin/quiverdesktop" "$WORK/appdir/usr/bin/quiver"

tar -C "$WORK/appdir" -czf "$WORK/payload.tar.gz" .

# The header is fixed-length by construction: __PAYLOAD_AT__ is replaced with a
# same-width, zero-padded number, so substituting it cannot move the offset it
# describes.
cat >"$WORK/header.sh" <<EOF
#!/bin/sh
# Quiver Desktop ${BUILD_ID} -- E2E stand-in for the release AppImage.
# See docker/e2e/fixtures/pack-appimage.sh for exactly how this differs from
# a real one (and, more importantly, how it does not).
set -e
PAYLOAD_AT=__PAYLOAD_AT__
ROOT="\${XDG_CACHE_HOME:-\$HOME/.cache}/quiver-appimage/${BUILD_ID}"
if [ ! -x "\$ROOT/usr/bin/quiverdesktop" ]; then
	rm -rf "\$ROOT"
	mkdir -p "\$ROOT"
	tail -c +\$PAYLOAD_AT "\$0" | tar -xzf - -C "\$ROOT"
fi
# exec, not a plain call: this is what makes the running process carry the
# name \`quiverdesktop\`, as it does under a real AppImage's AppRun.
exec "\$ROOT/usr/bin/quiverdesktop" "\$@"
EOF

HEADER_BYTES="$(stat -c%s "$WORK/header.sh")"
PAYLOAD_AT=$(( HEADER_BYTES + 1 ))
# Width 14 exactly, matching len("__PAYLOAD_AT__"), so the substitution is
# length-preserving and the offset stays the one that was measured.
PADDED="$(printf '%014d' "$PAYLOAD_AT")"
sed -i "s/__PAYLOAD_AT__/$PADDED/" "$WORK/header.sh"

NEW_BYTES="$(stat -c%s "$WORK/header.sh")"
if [ "$NEW_BYTES" != "$HEADER_BYTES" ]; then
	echo "header length changed during substitution ($HEADER_BYTES -> $NEW_BYTES)" >&2
	exit 1
fi

mkdir -p "$(dirname "$OUT")"
cat "$WORK/header.sh" "$WORK/payload.tar.gz" >"$OUT"
chmod +x "$OUT"

echo "[pack] $OUT ($(stat -c%s "$OUT") bytes, payload at byte $PAYLOAD_AT, build ${BUILD_ID})"

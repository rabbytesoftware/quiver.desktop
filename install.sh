#!/usr/bin/env sh
#
# Quiver Desktop installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/install.sh | bash
#
# No package manager is involved, deliberately. There is no one package manager
# across Debian, Fedora, Arch and everything else, and the .deb this project
# publishes only installs on the Debian family and only with root. The AppImage
# is a single executable file that runs the same everywhere and needs neither,
# so that is what this installs on Linux; macOS gets the .dmg.
#
# Where things land is the same as ARROW.md's own install lifecycle, so that
# quiver.core's preinstalled probe recognises a script install and an arrow
# install identically:
#
#   Linux   ${XDG_DATA_HOME:-$HOME/.local/share}/Quiver/Quiver.AppImage
#   macOS   /Applications/Quiver.app, or $HOME/Applications/Quiver.app when
#           /Applications is not writable
#
# POSIX sh with one documented exception: `local`, which is not in POSIX but is
# implemented by every shell this can realistically run under (dash, bash, ash,
# ksh, zsh). rustup's installer makes the same assumption for the same reason,
# hence the file-wide SC3043 suppression below.

# shellcheck disable=SC3043

set -eu

QUIVER_REPO="${QUIVER_REPO:-rabbytesoftware/quiver.desktop}"
QUIVER_API="${QUIVER_API:-https://api.github.com}"

# The release tag to install. Empty means "whatever /releases/latest points at".
QUIVER_TAG="${QUIVER_TAG:-}"

# Paths the exit trap cleans up. Set by main and install_macos rather than by a
# trap of their own: a second `trap ... EXIT` replaces the first, so nesting
# them would silently drop the download directory on macOS.
QUIVER_TMPDIR=""
QUIVER_MOUNTPOINT=""

cleanup() {
	if [ -n "$QUIVER_MOUNTPOINT" ]; then
		hdiutil detach "$QUIVER_MOUNTPOINT" -quiet -force >/dev/null 2>&1 || true
	fi
	if [ -n "$QUIVER_TMPDIR" ]; then
		rm -rf "$QUIVER_TMPDIR"
	fi
}

# --- output -----------------------------------------------------------------
# Everything goes to stderr: stdout is reserved for the values functions
# return, which is what makes them testable by capturing their output.

say() {
	printf '%s\n' "$1" >&2
}

warn() {
	printf 'warning: %s\n' "$1" >&2
}

err() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

need_cmd() {
	if ! command -v "$1" >/dev/null 2>&1; then
		err "$1 is required but was not found on PATH"
	fi
}

usage() {
	cat >&2 <<'EOF'
Install Quiver Desktop.

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --tag <TAG>     Install this release tag instead of the latest one
    -h, --help      Print this message

ENVIRONMENT:
    QUIVER_REPO     owner/name of the repository to install from
    QUIVER_API      Base URL of the GitHub API to query
    QUIVER_TAG      Same as --tag
EOF
}

# --- platform ---------------------------------------------------------------

# detect_platform prints "linux" or "macos". Anything else is a hard error:
# this script has no correct behaviour on a platform whose assets it cannot
# name. Windows users want install.ps1.
detect_platform() {
	local kernel
	kernel=$(uname -s)
	case "$kernel" in
	Linux) printf 'linux\n' ;;
	Darwin) printf 'macos\n' ;;
	MINGW* | MSYS* | CYGWIN* | Windows_NT)
		err "this script does not install on Windows. Use install.ps1 instead:
    irm https://raw.githubusercontent.com/${QUIVER_REPO}/develop/install.ps1 | iex"
		;;
	*)
		err "unsupported operating system: ${kernel}"
		;;
	esac
}

# detect_arch prints "amd64" or "arm64". An unrecognised machine name is not
# fatal on its own: it prints "unknown", which asset selection reads as "do not
# narrow by architecture", so a release carrying exactly one asset of the right
# kind still installs.
detect_arch() {
	local machine
	machine=$(uname -m)
	case "$machine" in
	x86_64 | amd64) printf 'amd64\n' ;;
	aarch64 | arm64) printf 'arm64\n' ;;
	*) printf 'unknown\n' ;;
	esac
}

# --- http -------------------------------------------------------------------

# http_get prints the body of $1 on stdout. curl first, wget as the fallback:
# minimal images ship one or the other, rarely neither, and insisting on curl
# would fail on a wget-only image for no reason this script cares about.
http_get() {
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL --proto '=https' --tlsv1.2 "$1"
	elif command -v wget >/dev/null 2>&1; then
		wget -qO- "$1"
	else
		err "neither curl nor wget is available; cannot read ${1}"
	fi
}

# http_download writes $1 to the file at $2.
http_download() {
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1"
	elif command -v wget >/dev/null 2>&1; then
		wget -qO "$2" "$1"
	else
		err "neither curl nor wget is available; cannot download ${1}"
	fi
}

# --- release metadata -------------------------------------------------------

release_url() {
	if [ -n "$QUIVER_TAG" ]; then
		printf '%s/repos/%s/releases/tags/%s\n' "$QUIVER_API" "$QUIVER_REPO" "$QUIVER_TAG"
	else
		printf '%s/repos/%s/releases/latest\n' "$QUIVER_API" "$QUIVER_REPO"
	fi
}

# list_asset_urls prints one asset download URL per line, read from the release
# JSON on stdin.
#
# Parsed with grep and sed rather than jq, because jq is not installed by
# default on any of the distributions this has to work on. The API returns the
# document on a single line and every asset URL appears exactly once as a
# "browser_download_url" value, so a match-only grep over the whole blob is
# enough. The one property this relies on is that a URL cannot contain an
# unescaped double quote.
list_asset_urls() {
	grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' |
		sed -e 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//' -e 's/"$//'
}

# arch_pattern prints the extended-regex alternation matching $1 in an asset
# filename. Tauri names the Linux bundle with the Debian architecture (amd64)
# and macOS ones with the Rust triple's (x86_64); both spellings, plus the
# x64/aarch64 variants other tooling emits, are accepted.
arch_pattern() {
	case "$1" in
	amd64) printf '(amd64|x86_64|x64)\n' ;;
	arm64) printf '(arm64|aarch64)\n' ;;
	*) printf '.\n' ;;
	esac
}

# select_asset_url prints the download URL to install, given the release JSON
# on stdin plus a platform ($1) and an architecture ($2).
#
# Selection is by extension, never by filename: quiver.desktop's release
# filenames carry tauri.conf.json's static "0.1.0", which does not track the
# git tag, so no name can be predicted from the tag being installed.
#
# The .deb and the .msi are published too and are deliberately never chosen.
# The .deb needs root and a Debian-family distro; the .msi installs per-machine
# and needs elevation. ARROW.md's install lifecycle makes the same two choices,
# so both routes put the same thing in the same place.
select_asset_url() {
	local platform arch urls pattern narrowed
	platform=$1
	arch=$2

	case "$platform" in
	linux) pattern='\.AppImage$' ;;
	macos) pattern='\.dmg$' ;;
	*) err "cannot select an asset for platform ${platform}" ;;
	esac

	urls=$(list_asset_urls | grep -E "$pattern" || true)
	if [ -z "$urls" ]; then
		err "this release publishes no ${pattern} asset for ${platform}"
	fi

	narrowed=$(printf '%s\n' "$urls" | grep -Ei "$(arch_pattern "$arch")" || true)
	if [ -n "$narrowed" ]; then
		urls=$narrowed
	fi

	printf '%s\n' "$urls" | head -n 1
}

# select_checksums_url prints the URL of the release's checksum manifest, or
# nothing when there is none.
#
# HONEST GAP: .github/workflows/stable-release.yml uploads the bundle files and
# nothing else, so no release publishes a checksum manifest today and this
# always prints nothing. The lookup is here so that publishing one later turns
# verification on with no change to this script, and verify_checksum says out
# loud when it found nothing rather than implying a check happened.
select_checksums_url() {
	list_asset_urls | grep -Ei '/(checksums(\.txt)?|SHA256SUMS(\.txt)?)$' | head -n 1 || true
}

# --- verification -----------------------------------------------------------

# sha256_of prints the SHA-256 of file $1, or nothing when the machine has
# neither tool that computes one.
sha256_of() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | cut -d' ' -f1
	elif command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$1" | cut -d' ' -f1
	else
		printf '\n'
	fi
}

# expected_sum prints the digest recorded for file name $2 in the
# sha256sum-format manifest $1, or nothing when that name is absent. Handles
# both the text (two spaces) and binary ("*name") forms sha256sum writes.
expected_sum() {
	printf '%s\n' "$1" | awk -v want="$2" '
		{ name = $2; sub(/^\*/, "", name); if (name == want) { print $1; exit } }
	'
}

# verify_checksum checks the downloaded file $1 against the manifest at URL $2.
# A missing manifest, or a manifest with no line for this file, is reported and
# accepted: the download still came from GitHub over TLS. A digest that is
# present and does not match is fatal, and takes the file with it.
verify_checksum() {
	local file sums_url name sums want got
	file=$1
	sums_url=$2
	name=$(basename "$file")

	if [ -z "$sums_url" ]; then
		warn "this release publishes no checksum manifest, so the download could not be verified beyond TLS"
		return 0
	fi

	sums=$(http_get "$sums_url")
	want=$(expected_sum "$sums" "$name")
	if [ -z "$want" ]; then
		warn "the checksum manifest has no entry for ${name}; skipping verification"
		return 0
	fi

	got=$(sha256_of "$file")
	if [ -z "$got" ]; then
		warn "neither sha256sum nor shasum is available; skipping verification"
		return 0
	fi

	if [ "$want" != "$got" ]; then
		rm -f "$file"
		err "checksum mismatch for ${name}: expected ${want}, got ${got}"
	fi
	say "Checksum verified."
}

# --- installation -----------------------------------------------------------

# appimage_path prints where the AppImage is installed. This is the same path
# ARROW.md's QUIVER_DESKTOP_APPIMAGE_PATH variable defaults to. The two must
# not drift: if they do, quiver.core's preinstalled probe stops recognising
# what this script installed.
appimage_path() {
	printf '%s/Quiver/Quiver.AppImage\n' "${XDG_DATA_HOME:-$HOME/.local/share}"
}

install_linux() {
	local src target dir
	src=$1
	target=$(appimage_path)
	dir=$(dirname "$target")

	mkdir -p "$dir" || err "cannot create ${dir}: no write permission"
	chmod +x "$src"
	mv -f "$src" "$target" || err "cannot write ${target}: no write permission"

	say "Installed to ${target}"
	say "Run it with: ${target}"
}

install_macos() {
	local dmg dest
	dmg=$1

	QUIVER_MOUNTPOINT=$(dirname "$dmg")/mnt
	mkdir -p "$QUIVER_MOUNTPOINT"
	hdiutil attach "$dmg" -mountpoint "$QUIVER_MOUNTPOINT" -nobrowse -quiet ||
		err "could not mount ${dmg}"

	if [ ! -d "$QUIVER_MOUNTPOINT/Quiver.app" ]; then
		err "the disk image does not contain Quiver.app"
	fi

	dest=/Applications
	[ -w "$dest" ] || dest="$HOME/Applications"
	mkdir -p "$dest" || err "cannot create ${dest}: no write permission"

	rm -rf "$dest/Quiver.app"
	# ditto, not cp -R: it is the copy that preserves a bundle's extended
	# attributes, and so its code signature.
	ditto "$QUIVER_MOUNTPOINT/Quiver.app" "$dest/Quiver.app" ||
		err "cannot write ${dest}/Quiver.app: no write permission"

	say "Installed to ${dest}/Quiver.app"
	say "Open it with: open -a \"${dest}/Quiver.app\""
}

# --- entry point ------------------------------------------------------------

main() {
	while [ $# -gt 0 ]; do
		case "$1" in
		-h | --help)
			usage
			return 0
			;;
		--tag)
			if [ $# -lt 2 ]; then
				err "--tag requires a value"
			fi
			QUIVER_TAG=$2
			shift 2
			;;
		--tag=*)
			QUIVER_TAG=${1#--tag=}
			shift
			;;
		*)
			err "unknown option: $1 (try --help)"
			;;
		esac
	done

	need_cmd uname
	need_cmd mktemp

	local platform arch json url sums_url file
	platform=$(detect_platform)
	arch=$(detect_arch)

	say "Installing Quiver Desktop for ${platform}/${arch}..."

	json=$(http_get "$(release_url)") ||
		err "could not read the release list from GitHub. Check your connection and try again."
	if [ -z "$json" ]; then
		err "GitHub returned an empty release document. ${QUIVER_REPO} may have no published release yet."
	fi

	url=$(printf '%s' "$json" | select_asset_url "$platform" "$arch")
	sums_url=$(printf '%s' "$json" | select_checksums_url)

	QUIVER_TMPDIR=$(mktemp -d)
	trap cleanup EXIT
	file="$QUIVER_TMPDIR/$(basename "$url")"

	say "Downloading $(basename "$url")..."
	http_download "$url" "$file" || err "download failed: ${url}"

	verify_checksum "$file" "$sums_url"

	case "$platform" in
	linux) install_linux "$file" ;;
	macos) install_macos "$file" ;;
	esac

	say "Done."
}

# Sourcing this file with QUIVER_INSTALL_SOURCE_ONLY=1 defines every function
# and runs nothing, which is how the unit tests exercise platform detection,
# asset selection and checksum matching without touching the network.
if [ "${QUIVER_INSTALL_SOURCE_ONLY:-0}" != "1" ]; then
	main "$@"
fi

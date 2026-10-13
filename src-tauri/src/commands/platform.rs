//! This machine's real `"os/arch"` platform key, the way quiver.core's
//! manifest `targets` are keyed (`internal/domain/os.go`'s `domain.OS`,
//! quiver.core side).
//!
//! The webview cannot answer this honestly: `src/lib/platform.ts`'s
//! `currentPlatform()` guesses from `navigator.userAgent`, and its own doc
//! comment already admits Apple Silicon Macs commonly omit or misreport the
//! architecture hint there. `std::env::consts::OS`/`ARCH` are the compiler's
//! own answer for the binary actually running, the same source
//! `resolve_release_asset` (`src-tauri/src/commands/release.rs`) already
//! trusts for the identical reason.

/// Maps Rust's `std::env::consts` spelling to the Go-style spelling
/// quiver.core's manifests use, for the 6 platforms it supports:
/// `linux/amd64`, `linux/arm64`, `windows/amd64`, `windows/arm64`,
/// `darwin/amd64`, `darwin/arm64`.
///
/// An input outside that set is passed through raw rather than rejected --
/// there is no sensible error to hand back here, and a future Rust target
/// triple this hasn't been taught about yet should degrade to an unrecognised
/// (and thus never-matching) platform string, not crash the app.
fn go_platform(os: &str, arch: &str) -> String {
	let os = match os {
		"macos" => "darwin",
		other => other,
	};
	let arch = match arch {
		"x86_64" => "amd64",
		"aarch64" => "arm64",
		other => other,
	};
	format!("{os}/{arch}")
}

/// This machine's real platform key, straight from the compiler's own
/// `std::env::consts` -- not a runtime probe, so a universal macOS bundle
/// running under Rosetta still reports the architecture it was BUILT for.
#[tauri::command]
pub fn get_platform() -> String {
	go_platform(std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn maps_linux_amd64() {
		assert_eq!(go_platform("linux", "x86_64"), "linux/amd64");
	}

	#[test]
	fn maps_linux_arm64() {
		assert_eq!(go_platform("linux", "aarch64"), "linux/arm64");
	}

	#[test]
	fn maps_windows_amd64() {
		assert_eq!(go_platform("windows", "x86_64"), "windows/amd64");
	}

	#[test]
	fn maps_windows_arm64() {
		assert_eq!(go_platform("windows", "aarch64"), "windows/arm64");
	}

	#[test]
	fn maps_macos_to_darwin_amd64() {
		assert_eq!(go_platform("macos", "x86_64"), "darwin/amd64");
	}

	#[test]
	fn maps_macos_to_darwin_arm64() {
		assert_eq!(go_platform("macos", "aarch64"), "darwin/arm64");
	}

	/// An unrecognised triple (a future Rust target this hasn't been taught
	/// about) passes through raw rather than panicking or silently guessing --
	/// it will simply never match any manifest target, which is the safe
	/// failure mode.
	#[test]
	fn passes_through_an_unrecognised_os_or_arch_raw() {
		assert_eq!(go_platform("freebsd", "x86_64"), "freebsd/amd64");
		assert_eq!(go_platform("linux", "riscv64"), "linux/riscv64");
	}

	/// The command is the frontend's only view of this, so it has to agree
	/// with the function it wraps rather than re-deriving anything.
	#[test]
	fn the_command_reports_this_binary_s_own_real_platform() {
		assert_eq!(
			get_platform(),
			go_platform(std::env::consts::OS, std::env::consts::ARCH)
		);
	}
}

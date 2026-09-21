//! The frontend's one view of [`crate::release`].
//!
//! Thin on purpose, the same way `build_info`'s command is: the native side
//! answers a question only it can answer honestly (which asset belongs to
//! THIS machine), and the frontend decides what to do about the answer. In
//! particular, whether an asset with no published checksum may be installed
//! is not decided here -- `checksum` comes back as `null` and
//! `src/features/arrow-details/lib/release-variables.ts` rules on it, next to
//! the message the user would read.
//!
//! Supplying the defaults is also this layer's job rather than
//! `crate::release`'s. A function that reaches for `api.github.com` with no
//! way to be pointed anywhere else cannot be tested without the network, so
//! it belongs here, where nothing else is either, and `resolve` stays a
//! function that can be driven against a local server.

use crate::release::{resolve, ResolveError, ResolvedAsset, DEFAULT_API, DEFAULT_REPO};

/// This app's own newest release asset for the platform it is running on.
///
/// Called at the moment the user clicks Update (or Install) on Quiver's own
/// tile, to fill in `QUIVER_RELEASE_ASSET_URL` and `QUIVER_RELEASE_CHECKSUM`
/// -- the two variables `ARROW.md` declares without defaults precisely
/// because the caller is the only party that can resolve them.
///
/// `std::env::consts` and not a runtime probe: these are the compiler's own
/// answer for the binary being executed, so a universal macOS bundle running
/// under Rosetta still reports the architecture it was BUILT for, which is
/// the one whose bundle it should be updated with. It is also the one thing
/// the webview could not have answered: `navigator.platform` says `MacIntel`
/// on Apple Silicon.
#[tauri::command]
pub async fn resolve_release_asset() -> Result<ResolvedAsset, ResolveError> {
	resolve(
		DEFAULT_API,
		DEFAULT_REPO,
		None,
		std::env::consts::OS,
		std::env::consts::ARCH,
	)
	.await
}

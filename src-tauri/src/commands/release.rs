//! The frontend's one view of [`crate::release`].
//!
//! Thin on purpose, the same way `build_info`'s command is: the native side
//! answers a question only it can answer honestly (which asset belongs to
//! THIS machine), and the frontend decides what to do about the answer. In
//! particular, whether an asset with no published checksum may be installed
//! is not decided here -- `checksum` comes back as `null` and
//! `src/features/arrow-details/lib/release-variables.ts` rules on it, next to
//! the message the user would read.

use crate::release::{resolve_latest, ResolveError, ResolvedAsset};

/// This app's own newest release asset for the platform it is running on.
///
/// Called at the moment the user clicks Update (or Install) on Quiver's own
/// tile, to fill in `QUIVER_RELEASE_ASSET_URL` and `QUIVER_RELEASE_CHECKSUM`
/// -- the two variables `ARROW.md` declares without defaults precisely
/// because the caller is the only party that can resolve them.
#[tauri::command]
pub async fn resolve_release_asset() -> Result<ResolvedAsset, ResolveError> {
	resolve_latest().await
}

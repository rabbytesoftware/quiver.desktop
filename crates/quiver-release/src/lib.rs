//! Resolving this app's own release asset from its GitHub release page, and
//! the build stamp baked into the binary. No GPUI, no Tauri.
//!
//! Nothing in the Tauri app updated itself in Rust beyond this: the self-update
//! is an ARROW lifecycle run by quiver.core, which needs the asset URL and
//! checksum this crate resolves (see [`resolve`]). Core updates and rollback
//! are quiver.core's job and reach the app through its HTTP API.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod asset;
pub mod build_info;
mod checksum;
mod error;
mod http;
mod page;
mod resolve;
mod select;

pub use asset::{ReleaseAsset, ResolvedAsset};
pub use checksum::{checksum_manifest_url, digest_hex, expected_sum};
pub use error::{kind, Error, Result};
pub use resolve::{resolve, resolve_own_release, DEFAULT_ORIGIN, DEFAULT_REPO};
pub use select::select_asset;

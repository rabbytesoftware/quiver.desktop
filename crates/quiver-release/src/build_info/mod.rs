//! The release tag this binary was built from -- when it was built from one.
//!
//! `build.rs` runs at COMPILE time and, if that build is a genuine release
//! build, emits `cargo:rustc-env=QUIVER_DESKTOP_BUILD_TAG=stable-<series>`.
//! `option_env!` then bakes it in as a compile-time constant: `Some(tag)` in a
//! release build, `None` in every dev, PR-CI and untagged build. Nothing here
//! reads the environment at RUNTIME -- a tag on the user's machine, or an
//! exported variable in whatever shell launched the app, cannot influence what
//! this app claims to be.
//!
//! This is what lets `announceSelf` (src/lib/core-store/listeners/
//! self-announce.ts) announce `github.com/rabbytesoftware/quiver.desktop@<tag>`
//! -- a concrete, resolvable ref naming the exact build the user is running --
//! instead of falling back to a refless announce that tracks whatever the
//! latest published release happens to be. See that file for the trade-off the
//! refless path still carries for untagged builds.
//!
//! `tauri.conf.json`'s `version` ("0.1.0") is deliberately NOT what this
//! reports. That is a productVersion, not a git ref: no ref named `0.1.0` will
//! ever exist, because this repo's `stable-release.yml` tags releases
//! `stable-<series>[.patch]`. Announcing it 404'd permanently, which is the
//! defect this module exists to close properly.

mod stamp;
mod tag;

pub use stamp::{build_stamp, BuildStamp};
pub use tag::release_tag;

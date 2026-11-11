//! The non-view glue of the native app: adapters that implement the traits of
//! the pure crates over each other, the arrow window supervisor and the
//! start/stop lifecycle. Views call [`AppServices`]; nothing here draws.
//!
//! * [`settings`] / [`secrets`]: `quiver-core-client`'s stores over `quiver-platform`.
//! * [`events`]: the connection layer's [`EventSink`](quiver_core_client::EventSink)
//!   fanned out to channels, which is what `quiver-state` listens on.
//! * [`core_api`]: `quiver-state`'s `CoreApi` over the ACTIVE transport.
//! * [`backend`]: `quiver-state`'s `Backend` over the connection manager.
//! * [`release_vars`]: this app's own release asset and build stamp.
//! * [`arrow_host`] / [`arrow_windows`]: arrow apps, one embedded webview each.
//! * [`native_menu`]: the OS's own context menu.
//! * [`lifecycle`]: build everything, tear everything down.

mod announce;
mod arrow_host;
mod arrow_windows;
mod backend;
mod core_api;
mod error;
mod events;
#[cfg(target_os = "linux")]
mod gtk_thread;
pub mod http;
mod lifecycle;
pub mod native_menu;
mod release_vars;
mod secrets;
mod settings;

pub use arrow_host::CoreUpstream;
pub use arrow_windows::{
    cover as page_cover, ArrowWindows, EmbeddedPages, HelperArgs, HelperLauncher, HelperLink,
    WindowEvent,
};
pub use backend::NativeBackend;
pub use core_api::TransportCoreApi;
pub use error::{Error, Result};
pub use events::ChannelEvents;
pub use lifecycle::{AppServices, ServicesConfig};
pub use release_vars::{resolve_release_variables, BuildInfo, NativeReleaseSource, ReleaseSource};
pub use secrets::KeyringSecrets;
pub use settings::{FileKeyValue, FileSettings};

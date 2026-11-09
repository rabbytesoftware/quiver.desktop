//! Everything the app needs from the operating system that is not UI: the
//! open-file limit, this machine's platform key, the per-OS directories, a
//! JSON settings store, keyring secrets and the macOS menu as plain data.
//! No GPUI, no Tauri: the app crate turns [`MenuSpec`] into native menus and
//! reads [`window::WindowSpec`] when it opens its window.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
pub mod fdlimit;
pub mod menu;
pub mod paths;
mod platform_key;
pub mod secrets;
pub mod settings;
pub mod window;

pub use error::{Error, Result};
pub use menu::{macos_menu, MenuAction, MenuItemSpec, MenuSpec, SubmenuSpec};
pub use paths::{AppPaths, PathEnv, APP_IDENTIFIER};
pub use platform_key::{go_platform, platform_key};
pub use secrets::{KeyringStore, MemorySecretStore, SecretStore, KEYRING_SERVICE};
pub use settings::SettingsStore;

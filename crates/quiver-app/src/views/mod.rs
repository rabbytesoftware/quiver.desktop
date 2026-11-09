//! Views: one folder per feature (`<feature>/mod.rs` + `parts/`). A view draws what `quiver-state`
//! decided; it holds no business logic. See `README.md` in this folder for the contract.

pub mod home;
pub mod sidebar;
pub mod tiles;

use gpui::{IntoElement, SharedString};
use quiver_ui::inspect::{probe, Kind};

/// Marks the element it is added to as a whole screen for the e2e bridge (`screen.<name>`).
/// Add it as the LAST child of the screen's root element.
pub fn screen_probe(name: &str) -> impl IntoElement {
    let id: SharedString = format!("screen.{name}").into();
    probe(id, Kind::Screen, name.to_owned(), true, "")
}

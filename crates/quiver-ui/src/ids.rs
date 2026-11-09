//! Element id composition for the sub-parts of a component.

use gpui::{ElementId, SharedString};

/// `parent` + a part name: the id of a sub-element (a tab, a menu item) derived from the component id.
pub fn child_id(parent: &ElementId, part: impl Into<SharedString>) -> ElementId {
    ElementId::from((parent.clone(), part))
}

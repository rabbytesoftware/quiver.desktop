//! Element registry for the e2e control bridge.
//!
//! Every interactive element and every screen reports itself here while it paints, under a stable id.
//! The bridge (a debug-only server in `quiver-app`) reads the registry to dump the view tree, to find
//! where an element is (so it can click or type into it) and to wait on named signals.
//!
//! The registry is a plain process-wide structure, not a GPUI global, so the bridge's I/O thread can
//! read it. Without the `inspect` feature every function is an empty inline stub: release builds carry
//! no registry and no bridge (CI checks the shipped artifact for the bridge symbols).

use gpui::SharedString;

/// What kind of thing an element is. The bridge exposes this in the view tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Screen,
    Button,
    Input,
    Switch,
    Tab,
    MenuItem,
    Select,
    Slider,
    Dialog,
    Text,
    Other,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Screen => "screen",
            Kind::Button => "button",
            Kind::Input => "input",
            Kind::Switch => "switch",
            Kind::Tab => "tab",
            Kind::MenuItem => "menu-item",
            Kind::Select => "select",
            Kind::Slider => "slider",
            Kind::Dialog => "dialog",
            Kind::Text => "text",
            Kind::Other => "other",
        }
    }
}

/// One painted element, in window coordinates (logical pixels).
#[derive(Debug, Clone)]
pub struct Node {
    pub id: SharedString,
    pub kind: Kind,
    pub label: SharedString,
    pub enabled: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Free-form state the element wants visible (`"checked"`, `"value=3"`, `"selected"`).
    pub state: SharedString,
    pub frame: u64,
}

#[cfg(feature = "inspect")]
mod live {
    use super::Node;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{LazyLock, Mutex};

    static FRAME: AtomicU64 = AtomicU64::new(0);
    static NODES: LazyLock<Mutex<BTreeMap<String, Node>>> =
        LazyLock::new(|| Mutex::new(BTreeMap::new()));

    pub fn begin_frame() -> u64 {
        FRAME.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn frame() -> u64 {
        FRAME.load(Ordering::SeqCst)
    }

    pub fn record(mut node: Node) {
        node.frame = frame();
        if let Ok(mut nodes) = NODES.lock() {
            nodes.insert(node.id.to_string(), node);
        }
    }

    /// Nodes painted in the most recent frame, in id order.
    pub fn snapshot() -> Vec<Node> {
        let current = frame();
        NODES
            .lock()
            .map(|nodes| {
                nodes
                    .values()
                    .filter(|n| n.frame + 1 >= current)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find(id: &str) -> Option<Node> {
        snapshot().into_iter().find(|n| n.id.as_ref() == id)
    }
}

#[cfg(feature = "inspect")]
pub use live::{begin_frame, find, frame, record, snapshot};

#[cfg(not(feature = "inspect"))]
mod stub {
    use super::Node;

    #[inline(always)]
    pub fn begin_frame() -> u64 {
        0
    }
    #[inline(always)]
    pub fn frame() -> u64 {
        0
    }
    #[inline(always)]
    pub fn record(_node: Node) {}
    #[inline(always)]
    pub fn snapshot() -> Vec<Node> {
        Vec::new()
    }
    #[inline(always)]
    pub fn find(_id: &str) -> Option<Node> {
        None
    }
}

#[cfg(not(feature = "inspect"))]
pub use stub::{begin_frame, find, frame, record, snapshot};

/// An invisible overlay that reports its parent's painted bounds under `id`.
///
/// Add it as the LAST child of the element it describes (absolute positioning is relative to the
/// parent's box in GPUI, so it covers exactly that element and does not affect layout):
///
/// ```ignore
/// div().id("save").child("Save").child(inspect::probe("settings.save", Kind::Button, "Save", true, ""))
/// ```
pub fn probe(
    id: impl Into<SharedString>,
    kind: Kind,
    label: impl Into<SharedString>,
    enabled: bool,
    state: impl Into<SharedString>,
) -> impl gpui::IntoElement {
    use gpui::{canvas, px, Styled as _};
    let (id, label, state) = (id.into(), label.into(), state.into());
    canvas(
        |bounds, _, _| bounds,
        move |bounds, _, _, _| {
            record(Node {
                id: id.clone(),
                kind,
                label: label.clone(),
                enabled,
                x: f32::from(bounds.origin.x),
                y: f32::from(bounds.origin.y),
                w: f32::from(bounds.size.width),
                h: f32::from(bounds.size.height),
                state: state.clone(),
                frame: 0,
            });
        },
    )
    .absolute()
    .top(px(0.0))
    .left(px(0.0))
    .size_full()
}

//! The status badge on a tile's art (top-right): a dotted glyph and a label.
//!
//! Plain props: the view layer maps `quiver_state::tile::TileBadgeKind` onto [`TileStatus`] and
//! supplies the translated label.

use gpui::prelude::*;
use gpui::{black, div, point, px, App, BoxShadow, ElementId, FontWeight, Hsla, SharedString};

use crate::status_glyph::{GlyphKind, StatusGlyph};
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileStatus {
    Running,
    /// "Needs attention": the daemon lost track of the process.
    Detached,
    Updating,
    Stopping,
    Draining,
    Uninstalling,
    Installing,
    /// A newer release exists.
    UpdateAvailable,
    Installed,
    /// Not in the library ("Get").
    NotInstalled,
    /// A label the caller owns ("12 arrows").
    Custom,
}

impl TileStatus {
    fn glyph(self) -> Option<GlyphKind> {
        match self {
            Self::Running => Some(GlyphKind::Running),
            Self::Detached => Some(GlyphKind::NeedsAttention),
            Self::UpdateAvailable => Some(GlyphKind::UpdateAvailable),
            Self::Updating
            | Self::Stopping
            | Self::Draining
            | Self::Uninstalling
            | Self::Installing => Some(GlyphKind::Busy),
            Self::Installed | Self::NotInstalled | Self::Custom => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBadge {
    pub status: TileStatus,
    pub label: SharedString,
}

impl TileBadge {
    pub fn new(
        status: TileStatus,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            status,
            label: label.into(),
        }
    }
}

fn shadow(
    spread: f32,
    y: f32,
    blur: f32,
    alpha: f32,
) -> BoxShadow {
    BoxShadow {
        color: black().opacity(alpha),
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
    }
}

/// The badge element, absolutely positioned at the top-right of the art.
pub(crate) fn badge_element(
    badge: &TileBadge,
    id: ElementId,
    cx: &App,
) -> gpui::Div {
    let p = cx.quiver().palette;
    let running = badge.status == TileStatus::Running;
    let (bg, fg): (Hsla, Hsla) = if running {
        (p.primary, p.primary_foreground)
    } else if badge.status == TileStatus::Detached {
        (p.popover, p.destructive)
    } else {
        (p.popover, p.popover_foreground)
    };
    let shadows = if running {
        vec![shadow(0.0, 1.0, 3.0, 0.25)]
    } else {
        vec![shadow(1.0, 0.0, 0.0, 0.08), shadow(0.0, 1.0, 3.0, 0.12)]
    };
    div()
        .absolute()
        .top(px(10.))
        .right(px(10.))
        .flex()
        .items_center()
        .gap(px(5.))
        .h(px(22.))
        .px(px(8.))
        .rounded(px(7.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .shadow(shadows)
        .when_some(badge.status.glyph(), |d, kind| {
            d.child(StatusGlyph::new(id, kind).size(12.).color(fg))
        })
        .child(badge.label.clone())
}

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ElementId, Entity, Hsla, IntoElement, Window};
use quiver_state::search::tile_width;
use quiver_ui::ScrollArea;

use crate::model::AppModel;

pub const GRID_GAP_X: f32 = 12.;
pub const GRID_GAP_Y: f32 = 18.;

/// The scrolling page body: 24 px at the left, 40 at the right, as on Home.
pub fn page(
    id: impl Into<ElementId>,
    surface: Hsla,
    body: impl IntoElement,
) -> impl IntoElement {
    ScrollArea::new(id).fade_color(surface).child(
        div()
            .pl(px(24.))
            .pr(px(40.))
            .pt(px(10.))
            .pb(px(32.))
            .child(body),
    )
}

/// Tiles wrap at the gap the boards use.
pub fn tile_grid(tiles: impl IntoIterator<Item = AnyElement>) -> gpui::Div {
    div()
        .flex()
        .flex_wrap()
        .gap_x(px(GRID_GAP_X))
        .gap_y(px(GRID_GAP_Y))
        .children(tiles)
}

/// The tile width the old `columnRule(total)` gives in the space left of the sidebar.
pub fn grid_tile_width(
    app: &Entity<AppModel>,
    window: &Window,
    total: usize,
    cx: &App,
) -> f32 {
    let sidebar = app.read(cx).shell.sidebar_width as f32;
    let available = f32::from(window.viewport_size().width) - sidebar - 24. - 40.;
    tile_width(total, available)
}

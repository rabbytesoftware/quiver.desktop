use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::home::HomeModel;
use quiver_state::i18n::en::home as keys;
use quiver_state::search::tile_width;
use quiver_ui::{SectionHeader, TileSize};

use crate::model::Route;
use crate::views::home::HomeView;
use crate::views::tiles::{library_input, tile};

pub fn library(
    view: &HomeView,
    model: &HomeModel,
    content: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let app = view.app.clone();
    // The tile width the old `columnRule(preview.len())` gives in this much room: as many columns as fit at
    // the 190 px minimum (capped by how many arrows there are), stretched to fill the row.
    let width = tile_width(model.library_preview.len(), content);
    let (title, link) = {
        let a = app.read(cx);
        (
            a.t(keys::LIBRARY),
            a.translator
                .t_count(keys::VIEW_ALL_ARROWS, model.library_total),
        )
    };
    let tiles = model.library_preview.iter().map(|entry| {
        tile(
            "home.library",
            &library_input(entry),
            TileSize::Custom(width),
            &app,
            cx,
        )
    });
    let go = app.clone();
    div()
        .w_full()
        .child(SectionHeader::new("home-library", title).link(
            link,
            "home.library.view-all",
            move |_, _, cx| {
                go.update(cx, |m, cx| m.navigate(Route::Library, cx));
            },
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(12.))
                .gap_y(px(19.))
                .children(tiles),
        )
}

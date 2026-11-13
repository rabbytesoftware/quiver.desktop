//! The followed collections, a preview of four with a link to the Collections page (the old Home's last
//! section). The tile width follows the room, as on every grid.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::collections::collection_badge_label;
use quiver_state::home::HomeModel;
use quiver_state::i18n::en::home as keys;
use quiver_state::i18n::Params;
use quiver_state::search::{tile_width, COLUMN_GAP};
use quiver_state::tile::TileInput;
use quiver_ui::{SectionHeader, TileSize};

use crate::model::Route;
use crate::views::home::HomeView;
use crate::views::tiles::tile;

pub fn collections(
    view: &HomeView,
    model: &HomeModel,
    content: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let app = view.app.clone();
    let (title, link) = {
        let a = app.read(cx);
        (
            a.t(keys::COLLECTIONS),
            a.translator.t_with(
                keys::VIEW_ALL_COLLECTIONS,
                &Params::count(model.collections_total),
            ),
        )
    };
    let width = tile_width(model.collections_preview.len(), content);
    let tiles = model.collections_preview.iter().map(|item| {
        // The list payload names no members, so how many are already in the library is unknown here: the
        // badge says the count only.
        let label = collection_badge_label(&app.read(cx).translator, item.arrow_count, None);
        let input = TileInput::from_collection_item(item, label);
        let go = app.clone();
        let id = item.namespace.clone();
        tile(
            "home.collections",
            &input,
            TileSize::Custom(width),
            &app,
            cx,
        )
        .on_click(move |_, _, cx| {
            let route = Route::Collection { id: id.clone() };
            go.update(cx, |m, cx| m.navigate(route, cx));
        })
    });
    let go = app.clone();
    div()
        .w_full()
        .child(SectionHeader::new("home-collections", title).link(
            link,
            "home.collections.view-all",
            move |_, _, cx| {
                go.update(cx, |m, cx| m.navigate(Route::Collections, cx));
            },
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(COLUMN_GAP as f32))
                .gap_y(px(19.))
                .children(tiles),
        )
}

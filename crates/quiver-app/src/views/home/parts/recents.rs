use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::home::HomeModel;
use quiver_state::i18n::en::home as keys;
use quiver_state::search::{equal_columns_width, COLUMN_GAP};
use quiver_state::tile::BadgePolicy;
use quiver_ui::{SectionHeader, TileSize};

/// Recents is a curated top 3, so its column count is fixed (see below), not derived from a result count.
const RECENTS_COLUMNS: usize = 3;

use crate::views::home::HomeView;
use crate::views::tiles::{library_input, tile};

pub fn recents(
    view: &HomeView,
    model: &HomeModel,
    content: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let app = view.app.clone();
    let title = app.read(cx).t(keys::RECENTS);
    // Exactly three equal columns that fill the row (the old `repeat(3, minmax(0,1fr))`, deliberately not the
    // column cap: Recents is a curated top 3, not an organic result count), so a wider window grows the tiles.
    let width = equal_columns_width(RECENTS_COLUMNS, content);
    let tiles = model.recents.iter().map(|entry| {
        let input = quiver_state::tile::TileInput {
            policy: BadgePolicy::ActivityOnly,
            ..library_input(entry)
        };
        tile("home.recents", &input, TileSize::Custom(width), &app, cx)
    });
    div()
        .child(SectionHeader::new("home-recents", title))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(COLUMN_GAP as f32))
                .gap_y(px(19.))
                .children(tiles),
        )
}

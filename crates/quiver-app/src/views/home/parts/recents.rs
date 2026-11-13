use gpui::prelude::*;
use gpui::{div, Context, IntoElement};
use quiver_state::home::HomeModel;
use quiver_state::i18n::en::home as keys;
use quiver_state::tile::BadgePolicy;
use quiver_ui::{SectionHeader, TileSize};

use crate::views::home::HomeView;
use crate::views::tiles::{library_input, tile};

pub fn recents(
    view: &HomeView,
    model: &HomeModel,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let app = view.app.clone();
    let title = app.read(cx).t(keys::RECENTS);
    let tiles = model.recents.iter().map(|entry| {
        let input = quiver_state::tile::TileInput {
            policy: BadgePolicy::ActivityOnly,
            ..library_input(entry)
        };
        tile("home.recents", &input, TileSize::Shelf, &app, cx)
    });
    div()
        .child(SectionHeader::new("home-recents", title))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x(gpui::px(12.))
                .gap_y(gpui::px(19.))
                .children(tiles),
        )
}

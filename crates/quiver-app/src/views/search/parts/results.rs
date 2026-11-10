//! The two shelves ("In your vault", "From the network") and the empty state.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::domain::SearchEntry;
use quiver_state::i18n::en::{app, search as keys};
use quiver_state::search::{EmptyState, ResultsModel};
use quiver_state::tile::TileInput;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant};

use crate::views::grid_page::{notice, skeleton, tile_grid};
use crate::views::search::SearchView;
use crate::views::tiles::tile;

/// Uppercase mono label, its count and a hairline to the edge.
fn shelf_label(
    id: &'static str,
    label: String,
    count: String,
    cx: &Context<SearchView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .flex()
        .items_center()
        .gap(px(10.))
        .mb(px(14.))
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(9.))
                .text_color(p.muted_foreground)
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(9.))
                .text_color(p.muted_foreground)
                .child(count.clone()),
        )
        .child(div().flex_1().h(px(1.)).bg(p.border))
        .child(probe(id, Kind::Other, label, true, count))
}

fn tiles(
    view: &SearchView,
    entries: &[SearchEntry],
    width: f32,
    cx: &Context<SearchView>,
) -> gpui::Div {
    tile_grid(entries.iter().map(|entry| {
        tile(
            "search",
            &TileInput::from_search_entry(entry),
            quiver_ui::TileSize::Custom(width),
            &view.app,
            cx,
        )
        .into_any_element()
    }))
}

pub fn shelves(
    view: &SearchView,
    model: &ResultsModel,
    width: f32,
    cx: &mut Context<SearchView>,
) -> impl IntoElement {
    let a = view.app.read(cx);
    let (vault, network, so_far) = (
        a.t(keys::SHELF_VAULT),
        a.t(keys::SHELF_NETWORK),
        a.translator.t_with(
            keys::SHELF_SO_FAR,
            &quiver_state::i18n::Params::count(model.network.len()),
        ),
    );
    let mut out = div().flex().flex_col().gap(px(22.));
    if !model.vault.is_empty() {
        out = out.child(
            div()
                .child(shelf_label(
                    "search.shelf.vault",
                    vault,
                    model.vault.len().to_string(),
                    cx,
                ))
                .child(tiles(view, &model.vault, width, cx)),
        );
    }
    if !model.network.is_empty() || model.discovering {
        let count = if model.discovering {
            so_far
        } else {
            model.network.len().to_string()
        };
        out = out.child(
            div()
                .child(shelf_label("search.shelf.network", network, count, cx))
                .child(tiles(view, &model.network, width, cx))
                .when(model.discovering, |d| {
                    d.child(div().mt(px(19.)).child(skeleton("search", 2, width, cx)))
                }),
        );
    }
    out
}

pub fn empty(
    view: &SearchView,
    state: &EmptyState,
    narrowed_to_nothing: bool,
    cx: &mut Context<SearchView>,
) -> impl IntoElement {
    let a = view.app.read(cx);
    if narrowed_to_nothing {
        return notice(
            "search.empty",
            a.t(keys::NARROW_EMPTY),
            String::new(),
            None,
            cx,
        )
        .into_any_element();
    }
    let text = a.t(state.text());
    let action = (*state == EmptyState::Unreachable).then(|| {
        let retry = a.t(app::CRASH_RETRY);
        let query = view.query.clone();
        let session = view.session.clone();
        Button::new("search.error.retry")
            .variant(ButtonVariant::Outline)
            .label(retry.clone())
            .child(probe("search.error.retry", Kind::Button, retry, true, ""))
            .on_click(move |_, _, _| session.restart(&query))
    });
    let id = if *state == EmptyState::Unreachable {
        "search.error"
    } else {
        "search.empty"
    };
    notice(id, text, String::new(), action, cx).into_any_element()
}

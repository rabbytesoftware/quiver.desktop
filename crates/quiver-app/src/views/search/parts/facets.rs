//! Narrowing chips: hosts and tags the answer actually holds, and a Clear when any is chosen.

use gpui::prelude::*;
use gpui::{div, Context, IntoElement};
use quiver_state::i18n::en::search as keys;
use quiver_state::search::{is_narrowed, offered_facets, toggle, FacetKind};

use crate::views::grid_page::chip;
use crate::views::search::SearchView;

pub fn facet_row(
    view: &SearchView,
    answer: &[quiver_state::domain::SearchEntry],
    cx: &mut Context<SearchView>,
) -> Option<impl IntoElement> {
    let offered = offered_facets(answer, &view.selection);
    if offered.is_empty() && !is_narrowed(&view.selection) {
        return None;
    }
    let mut row = div().flex().flex_wrap().gap_2().pb(px_gap());
    for (kind, facet) in offered {
        let chosen = match kind {
            FacetKind::Host => view.selection.host.contains(&facet.value),
            FacetKind::Tag => view.selection.tag.contains(&facet.value),
        };
        let kind_id = if kind == FacetKind::Host {
            "host"
        } else {
            "tag"
        };
        let value = facet.value.clone();
        row = row.child(chip(
            format!("search.facet.{kind_id}.{}", facet.value),
            format!("{} {}", facet.value, facet.count),
            chosen,
            cx,
            move |this: &mut SearchView, cx| {
                this.selection = toggle(&this.selection, kind, &value);
                cx.notify();
            },
        ));
    }
    if is_narrowed(&view.selection) {
        let clear = view.app.read(cx).t(keys::NARROW_CLEAR);
        row = row.child(chip(
            "search.facets.clear".into(),
            clear,
            false,
            cx,
            |this: &mut SearchView, cx| {
                this.selection = Default::default();
                cx.notify();
            },
        ));
    }
    Some(row)
}

fn px_gap() -> gpui::Pixels {
    gpui::px(16.)
}

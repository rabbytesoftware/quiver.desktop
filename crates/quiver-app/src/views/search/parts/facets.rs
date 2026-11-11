//! Narrowing chips: hosts and tags the answer actually holds, and a Clear when any is chosen.

use gpui::prelude::*;
use gpui::{div, Context, IntoElement};
use quiver_state::i18n::en::search as keys;
use quiver_state::search::{is_narrowed, offered_facets, toggle, FacetKind};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::grid_page::counted_chip;
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
    let mut row = div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap(gpui::px(6.))
        .overflow_hidden();
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
        row = row.child(counted_chip(
            format!("search.facet.{kind_id}.{}", facet.value),
            facet.value.clone(),
            Some(facet.count),
            chosen,
            cx,
            move |this: &mut SearchView, cx| {
                this.selection = toggle(&this.selection, kind, &value);
                cx.notify();
            },
        ));
    }
    let p = cx.quiver().palette;
    let clear = is_narrowed(&view.selection).then(|| {
        let text = view.app.read(cx).t(keys::NARROW_CLEAR);
        div()
            .id("search-facets-clear")
            .relative()
            .flex_none()
            .cursor_pointer()
            .text_size(gpui::px(11.5))
            .text_color(p.muted_foreground)
            .hover(|s| s.text_color(p.foreground))
            .underline()
            .on_click(cx.listener(|this, _, _, cx| {
                this.selection = Default::default();
                cx.notify();
            }))
            .child(text.clone())
            .child(probe("search.facets.clear", Kind::Button, text, true, ""))
    });
    Some(
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap(gpui::px(12.))
            .child(row)
            .children(clear),
    )
}

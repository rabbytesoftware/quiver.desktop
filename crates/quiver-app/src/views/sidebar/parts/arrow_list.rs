//! The scrolling arrow list, virtualised: only the visible rows are built.

use gpui::prelude::*;
use gpui::{div, px, uniform_list, Context, FontWeight, IntoElement, Window};
use quiver_state::sidebar::{build_list, ArrowListState, CatalogPhase};
use quiver_state::store::CatalogStatus;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, FlickerSpinner};

use super::super::Sidebar;
use super::row::arrow_row;

fn phase(status: CatalogStatus) -> CatalogPhase {
    match status {
        CatalogStatus::Loading => CatalogPhase::Loading,
        CatalogStatus::Ready => CatalogPhase::Ready,
        CatalogStatus::Error => CatalogPhase::Error,
    }
}

pub fn arrow_list(
    sidebar: &Sidebar,
    _: &mut Window,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let app = sidebar.app.read(cx);
    let (entries, catalog) = app
        .services
        .arrows
        .store()
        .read(|s| (s.arrows.values().cloned().collect::<Vec<_>>(), s.catalog));
    let list = build_list(&entries, phase(catalog), app.show_self_components);
    let body = match list {
        ArrowListState::Rows(rows) => {
            let selected = app.route().arrow_namespace().map(str::to_owned);
            uniform_list(
                "sidebar.rows",
                rows.len(),
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    range
                        .filter_map(|i| rows.get(i).cloned())
                        .map(|entry| {
                            let active = selected.as_deref()
                                == Some(quiver_state::namespace::bare_namespace(&entry.namespace));
                            arrow_row(this, entry, active, cx)
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(sidebar.list_scroll.clone())
            .flex_1()
            .into_any_element()
        }
        ArrowListState::Loading => div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .py(px(10.))
            .text_size(px(13.))
            .text_color(p.muted_foreground)
            .child(FlickerSpinner::new("sidebar-loading").size(16.))
            .child(app.t(quiver_state::i18n::en::sidebar::ARROWS_LOADING))
            .into_any_element(),
        ArrowListState::Notice(notice) => div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(4.))
            .px(px(10.))
            .py(px(10.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(app.t(notice.title)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.muted_foreground)
                    .child(app.t(notice.body)),
            )
            .child(probe("sidebar.notice", Kind::Text, "Notice", true, ""))
            .into_any_element(),
    };
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .px(px(6.))
        .pt(px(6.))
        .child(body)
        .child(probe("sidebar.list", Kind::Other, "Arrows", true, ""))
}

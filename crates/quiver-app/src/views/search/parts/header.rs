//! "2 results", the plain-words source line and the sort control.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, SharedString};
use quiver_state::i18n::en::search as keys;
use quiver_state::search::{ResultsModel, SORT_KEYS};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Select, SelectOption, SelectSize};

use crate::views::search::SearchView;

fn sort(view: &SearchView, model: &ResultsModel, cx: &mut Context<SearchView>) -> impl IntoElement {
    let a = view.app.read(cx);
    let options: Vec<SelectOption> = SORT_KEYS
        .iter()
        .map(|k| SelectOption::new(format!("{k:?}"), a.t(k.label())))
        .collect();
    let value = format!("{:?}", view.sort);
    let label = a.t(view.sort.label());
    let this = cx.entity();
    div()
        .relative()
        .w(px(144.))
        .child(
            Select::new("search-sort")
                .size(SelectSize::Default)
                .options(options)
                .value(Some(SharedString::from(value)))
                .disabled(!model.sortable)
                .on_change(move |chosen, _, cx| {
                    this.update(cx, |view, cx| {
                        view.sort = SORT_KEYS
                            .into_iter()
                            .find(|k| format!("{k:?}") == chosen.as_ref())
                            .unwrap_or_default();
                        cx.notify();
                    });
                }),
        )
        .child(probe(
            "search.sort",
            Kind::Select,
            label.clone(),
            model.sortable,
            format!("value={label}"),
        ))
}

pub fn header(
    view: &SearchView,
    model: &ResultsModel,
    cx: &mut Context<SearchView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let a = view.app.read(cx);
    let count = a.translator.t_count(keys::RESULTS_COUNT, model.count);
    let status = if model.pass_failed {
        Some(a.t(keys::RESULTS_PASS_FAILED))
    } else if model.searching {
        Some(a.t(keys::RESULTS_SEARCHING))
    } else {
        None
    };
    let source = model.source_line.as_ref().map(|s| s.text(&a.translator));
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.))
        .pb(px(18.))
        .child(
            div()
                .relative()
                .flex()
                .items_baseline()
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(p.muted_foreground)
                        .child(count.clone()),
                )
                .children(status.map(|text| {
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted_foreground)
                        .child(text)
                }))
                .child(probe("search.count", Kind::Text, count, true, "")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(14.))
                .children(source.map(|text| {
                    div()
                        .relative()
                        .h(px(21.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.border)
                        .text_size(px(11.))
                        .text_color(p.muted_foreground)
                        .child(text.clone())
                        .child(probe("search.sources", Kind::Text, text, true, ""))
                }))
                .child(sort(view, model, cx)),
        )
}

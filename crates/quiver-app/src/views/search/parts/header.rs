//! "2 results", the plain-words source line and the sort control.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement, SharedString};
use quiver_state::i18n::en::search as keys;
use quiver_state::search::{ResultsModel, SORT_KEYS};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Select, SelectOption, SelectSize};

use crate::views::search::SearchView;

fn sort(
    view: &SearchView,
    model: &ResultsModel,
    cx: &mut Context<SearchView>,
) -> impl IntoElement {
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

fn panel_glyph(color: gpui::Hsla) -> impl IntoElement {
    div()
        .relative()
        .flex_none()
        .w(px(11.))
        .h(px(9.2))
        .rounded(px(1.8))
        .border_1()
        .border_color(color.opacity(0.7))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(7.))
                .w(px(1.))
                .bg(color.opacity(0.7)),
        )
}

pub fn header(
    view: &SearchView,
    model: &ResultsModel,
    facets: Option<impl IntoElement>,
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
    let facets_present = facets.is_some();
    // The pill opens the inspector as soon as a pass exists: mid-pass, before the summary, it just says
    // "Inspect" (the old header did the same).
    let source = match &model.source_line {
        Some(line) => Some((
            line.sources_part(&a.translator),
            line.unavailable_part(&a.translator),
            line.text(&a.translator),
        )),
        None if view.session.store.snapshot().job.is_some() => {
            let inspect = a.t(keys::RESULTS_INSPECT);
            Some((inspect.clone(), None, inspect))
        }
        None => None,
    };
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
        .children(facets)
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(14.))
                .when(facets_present, |d| d.ml_auto())
                .children(source.map(|(lead, rest, text)| {
                    div()
                        .id("search-sources")
                        .relative()
                        .h(px(23.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(p.border)
                        .text_size(px(11.5))
                        .text_color(p.muted_foreground)
                        .cursor_pointer()
                        .hover(|s| s.bg(p.accent))
                        .on_click(cx.listener(|this, _, _, cx| this.open_inspector(cx)))
                        .child(panel_glyph(p.muted_foreground))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.foreground)
                                .child(lead),
                        )
                        .children(rest.map(|rest| div().child(format!("· {rest}"))))
                        .child(probe("search.sources", Kind::Button, text, true, ""))
                }))
                .child(sort(view, model, cx)),
        )
}

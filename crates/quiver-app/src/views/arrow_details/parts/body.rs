//! The left column: the README rendered as Markdown, or, when the manifest ships none, an
//! "At a glance" card built from what the manifest already knows (`ArrowNoBannerAfter`).

use gpui::prelude::*;
use gpui::{div, px, rems, Context, FontWeight, IntoElement, SharedString, Window};
use gpui_component::text::{TextView, TextViewStyle};
use quiver_state::arrow_details::glance::{BodyPlan, Glance};
use quiver_state::i18n::en::{arrow, design};
use quiver_state::i18n::Params;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

pub fn body(
    view: &mut ArrowDetailsView,
    page: &Page,
    window: &mut Window,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let column = div().relative().flex_1().min_w_0().max_w(px(560.));
    match &page.plan {
        BodyPlan::Readme => column
            .child(readme(view, page, window, cx))
            .into_any_element(),
        BodyPlan::Glance(glance) => column
            .child(glance_card(view, glance, cx))
            .into_any_element(),
    }
}

/// Large READMEs lay out lazily inside the page's scroll area; the parse runs off the UI thread.
fn readme(
    view: &ArrowDetailsView,
    page: &Page,
    window: &mut Window,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let source = page.detail.readme.clone().unwrap_or_default();
    let id = SharedString::from(format!("arrow-readme-{}", view.namespace));
    let style = TextViewStyle::default()
        .paragraph_gap(rems(1.1))
        .heading_font_size(|level, _| if level <= 1 { px(16.) } else { px(14.) });
    div()
        .relative()
        .child(
            TextView::markdown(id, source, window, cx)
                .style(style)
                .selectable(true)
                .text_size(px(14.))
                .line_height(px(22.))
                .text_color(p.foreground.opacity(0.78)),
        )
        .child(probe("arrow.readme", Kind::Text, "README", true, ""))
}

struct Line {
    id: &'static str,
    title: String,
    detail: String,
    link: Option<(String, Link)>,
}

#[derive(Clone, Copy)]
enum Link {
    Methods,
    Settings,
    Source,
}

fn lines(
    glance: &Glance,
    view: &ArrowDetailsView,
    cx: &gpui::App,
) -> Vec<Line> {
    let tr = view.app.read(cx).translator;
    let mut out = Vec::new();
    out.push(Line {
        id: "methods",
        title: tr.t(arrow::TAB_METHODS),
        detail: tr.t_with(
            design::GLANCE_METHODS,
            &Params::new()
                .with("count", glance.method_count)
                .with("names", glance.method_names.join(", ")),
        ),
        link: Some(("View methods".into(), Link::Methods)),
    });
    if glance.setting_count > 0 {
        out.push(Line {
            id: "settings",
            title: tr.t(arrow::TAB_SETTINGS),
            detail: tr.t_with(
                design::GLANCE_SETTINGS,
                &Params::new().with("count", glance.setting_count),
            ),
            link: Some((tr.t(arrow::PREVIEW_CONFIGURE), Link::Settings)),
        });
    }
    if !glance.runs_on.is_empty() {
        out.push(Line {
            id: "runs-on",
            title: tr
                .t_with(
                    design::PLATFORM_RUNS_ON,
                    &Params::new().with("platforms", ""),
                )
                .trim()
                .to_owned(),
            detail: glance.runs_on.clone(),
            link: None,
        });
    }
    if !glance.source.is_empty() {
        out.push(Line {
            id: "source",
            title: "Source".into(),
            detail: glance.source.clone(),
            link: Some(("Open".into(), Link::Source)),
        });
    }
    out
}

fn glance_card(
    view: &ArrowDetailsView,
    glance: &Glance,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let title = view.app.read(cx).t(design::GLANCE_TITLE);
    let source = glance.source.clone();
    let rows = lines(glance, view, cx);
    let last = rows.len().saturating_sub(1);
    let rows: Vec<_> = rows
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let id = format!("arrow.glance.{}", line.id);
            let source = source.clone();
            div()
                .relative()
                .h(px(56.))
                .px(px(20.))
                .flex()
                .items_center()
                .justify_between()
                .when(i != last, |d| d.border_b_1().border_color(p.border))
                .child(
                    div()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(line.title),
                        )
                        .child(
                            div()
                                .mt(px(3.))
                                .text_size(px(12.5))
                                .text_color(p.muted_foreground)
                                .child(line.detail),
                        ),
                )
                .children(line.link.map(|(label, link)| {
                    div()
                        .id(SharedString::from(format!("{id}.link")))
                        .cursor_pointer()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .underline()
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| match link {
                            Link::Methods => this.open_methods(cx),
                            Link::Settings => this.open_settings(cx),
                            Link::Source => cx.open_url(&format!("https://{source}")),
                        }))
                }))
                .child(probe(id, Kind::Other, line.id, true, ""))
        })
        .collect();
    div()
        .relative()
        .mt(px(-11.))
        .child(
            div()
                .text_size(px(16.))
                .line_height(px(22.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .mt(px(17.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.border)
                .children(rows),
        )
        .child(probe("arrow.glance", Kind::Other, "At a glance", true, ""))
}

//! The left column: the README rendered as Markdown, or, when the manifest ships none, an
//! "At a glance" card built from what the manifest already knows (`ArrowNoBannerAfter`).

use gpui::prelude::*;
use gpui::{div, px, rems, Context, FontWeight, IntoElement, SharedString, Window};
use gpui_component::text::{TextView, TextViewStyle};
use quiver_state::arrow_details::glance::{BodyPlan, Glance};
use quiver_state::i18n::en::{arrow, design};
use quiver_state::i18n::Params;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, TrackedText};

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

/// Above this a README stays one view: it lays out lazily inside the page's scroll area and the parse
/// runs off the UI thread, which a view per block would multiply.
const CHUNK_LIMIT: usize = 48 * 1024;

/// The README cut at its blank lines (never inside a fenced block): the Markdown view gives its
/// top-level blocks no gap of their own, so each block is a view with the margin the old page gave it.
fn blocks(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fence: Option<&str> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(m)) => fence = Some(m),
            (Some(open), Some(m)) if open == m => fence = None,
            _ => {}
        }
        if fence.is_none() && line.trim().is_empty() {
            if !current.is_empty() {
                out.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        out.push(current.join("\n"));
    }
    out
}

/// Markdown's soft line breaks as spaces. The Markdown view breaks the line at every newline inside a paragraph,
/// where a browser reflows the paragraph: a README wrapped at 80 columns would keep its source's line ends. Fenced
/// code, list items, quotes, tables, headings and explicit hard breaks (two trailing spaces, a backslash) keep
/// theirs.
fn soften(source: &str) -> String {
    let starts_block = |line: &str| {
        let t = line.trim_start();
        t.is_empty()
            || ["#", ">", "|", "```", "~~~", "- ", "* ", "+ ", "<"]
                .iter()
                .any(|m| t.starts_with(m))
            || t.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    let mut out = String::with_capacity(source.len());
    let mut fence: Option<&str> = None;
    let mut previous: Option<&str> = None;
    let mut first_done = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        let in_fence = fence.is_some();
        match (fence, marker) {
            (None, Some(m)) => fence = Some(m),
            (Some(open), Some(m)) if open == m => fence = None,
            _ => {}
        }
        let joins = !in_fence
            && marker.is_none()
            && !starts_block(line)
            && previous.is_some_and(|p| {
                let hard = p.ends_with("  ") || p.ends_with('\\');
                !hard
                    && !p.trim().is_empty()
                    && !p.trim_start().starts_with('|')
                    && !p.trim_start().starts_with('#')
            });
        if first_done {
            out.push(if joins { ' ' } else { '\n' });
        }
        first_done = true;
        out.push_str(if joins { trimmed } else { line });
        previous = if in_fence || marker.is_some() {
            None
        } else {
            Some(line)
        };
    }
    out
}

/// The margin above a block: headings open a section (`mt-6`, `mt-5`, `mt-4`), anything else is
/// separated by the paragraph gap (`mb-2.5`).
fn gap_above(block: &str) -> f32 {
    let level = block.chars().take_while(|c| *c == '#').count();
    match level {
        0 => 10.,
        1 | 2 => 24.,
        3 => 20.,
        _ => 16.,
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
    let source = soften(&page.detail.readme.clone().unwrap_or_default());
    let style = || {
        TextViewStyle::default()
            .paragraph_gap(rems(0.))
            .heading_font_size(|level, _| match level {
                1 => px(18.),
                2 => px(16.),
                _ => px(14.),
            })
    };
    let view_of =
        |id: String, text: String, window: &mut Window, cx: &mut Context<ArrowDetailsView>| {
            TextView::markdown(SharedString::from(id), text, window, cx)
                .style(style())
                .selectable(true)
                .text_size(px(14.))
                .line_height(px(22.))
                .text_color(p.foreground.opacity(0.78))
        };
    let column = if source.len() <= CHUNK_LIMIT {
        let mut column = div().flex().flex_col();
        for (i, block) in blocks(&source).into_iter().enumerate() {
            let gap = if i == 0 { 0. } else { gap_above(&block) };
            let id = format!("arrow-readme-{}-{i}", view.namespace);
            column = column.child(div().mt(px(gap)).child(view_of(id, block, window, cx)));
        }
        column
    } else {
        let id = format!("arrow-readme-{}", view.namespace);
        div().child(view_of(id, source, window, cx))
    };
    div()
        .relative()
        .child(column)
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
                    // The text gives way to the link: a long methods list ends in "…" inside the card.
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(line.title),
                        )
                        .child(div().mt(px(3.)).child(TrackedText::new(
                            line.detail,
                            12.5,
                            16.,
                            0.,
                            p.muted_foreground,
                        ))),
                )
                .children(line.link.map(|(label, link)| {
                    div()
                        .id(SharedString::from(format!("{id}.link")))
                        .flex_none()
                        .ml(px(16.))
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
        .child(TrackedText::new(title, 16., 22., -0.1, p.foreground).weight(FontWeight::SEMIBOLD))
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

#[cfg(test)]
mod tests {
    use super::soften;

    #[test]
    fn a_wrapped_paragraph_reflows() {
        assert_eq!(
            soften("An arrow with an update\nmethod, for the spec.\n\nNext."),
            "An arrow with an update method, for the spec.\n\nNext."
        );
    }

    #[test]
    fn lists_code_tables_headings_and_hard_breaks_keep_their_lines() {
        let source =
            "# Title\nText\n- one\n  more\n- two\n\n```\na\nb\n```\n| a |\n| b |\nhard  \nbreak";
        assert_eq!(
            soften(source),
            "# Title\nText\n- one more\n- two\n\n```\na\nb\n```\n| a |\n| b |\nhard  \nbreak"
        );
    }
}

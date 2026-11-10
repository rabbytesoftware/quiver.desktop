//! The scrolling output: one row per entry, coloured by span. A log record opens as JSON on click.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, IntoElement};
use quiver_state::console::entries::{ConsoleEntry, NewEntry};
use quiver_state::console::lines::{display_line, Line, SpanTone};
use quiver_state::console::Support;
use quiver_state::i18n::en::console;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Palette};

use crate::views::console::ConsoleDock;

fn tone_color(
    p: &Palette,
    tone: SpanTone,
) -> gpui::Hsla {
    match tone {
        SpanTone::Normal => p.foreground,
        SpanTone::Muted => p.muted_foreground,
        SpanTone::Warn => p.warning,
        SpanTone::Error => p.destructive,
        SpanTone::Ok => p.success,
        SpanTone::Number => p.info,
        SpanTone::Bool => p.success,
        SpanTone::Path => p.foreground,
        SpanTone::Prompt => p.primary,
    }
}

/// What kind of entry a row is, for the e2e driver (`console.line.<id>` state): `log:<level>`, `cmd`,
/// `out:stdout|stderr`, `note:<tone>` or `raw`.
fn kind_of(entry: &ConsoleEntry) -> String {
    match &entry.body {
        NewEntry::Log(record) => format!("log:{:?}", record.level).to_lowercase(),
        NewEntry::Raw(_) => "raw".into(),
        NewEntry::Cmd { .. } => "cmd".into(),
        NewEntry::Out { stream, .. } => format!("out:{stream:?}").to_lowercase(),
        NewEntry::Note { tone, .. } => format!("note:{tone:?}").to_lowercase(),
    }
}

fn row(
    p: &Palette,
    line: &Line,
    kind: String,
    this: &gpui::Entity<ConsoleDock>,
) -> impl IntoElement {
    let id = line.id;
    let this = this.clone();
    let text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
    div()
        .id(("console-line", id))
        .relative()
        .px(px(12.))
        .py(px(1.))
        .flex()
        .flex_wrap()
        .when(line.expandable, |d| {
            d.cursor_pointer().on_click(move |_, _, cx| {
                this.update(cx, |dock, cx| {
                    dock.controller.state.toggle_expanded(id);
                    cx.notify();
                })
            })
        })
        .children(line.spans.iter().map(|s| {
            div()
                .when(s.tone == SpanTone::Path, |d| d.underline())
                .when(matches!(s.tone, SpanTone::Warn | SpanTone::Error), |d| {
                    d.font_weight(FontWeight::MEDIUM)
                })
                .text_color(tone_color(p, s.tone))
                .child(s.text.clone())
        }))
        .child(probe(
            format!("console.line.{id}"),
            Kind::Text,
            text,
            true,
            kind,
        ))
}

pub fn log(
    dock: &mut ConsoleDock,
    cx: &mut Context<ConsoleDock>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let (unsupported, offset) = {
        let state = &dock.controller.state;
        (state.support == Support::Unsupported, 0)
    };
    let m = dock.app.read(cx);
    let lines: Vec<Line> = dock
        .controller
        .state
        .entries
        .iter()
        .map(|e| display_line(e, &m.translator, offset))
        .collect();
    let kinds: Vec<String> = dock.controller.state.entries.iter().map(kind_of).collect();
    let notice = unsupported.then(|| m.t(console::UNSUPPORTED));
    let count = lines.len();
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .child(
            div()
                .id("console-log")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&dock.scroll)
                .py(px(6.))
                .font_family(FontFamily::Mono.family())
                .text_size(px(12.))
                .line_height(px(17.))
                .children(notice.map(|n| div().px(px(12.)).text_color(p.muted_foreground).child(n)))
                .children(
                    lines
                        .iter()
                        .zip(&kinds)
                        .map(|(l, k)| row(&p, l, k.clone(), &this)),
                ),
        )
        .child(probe(
            "console.output",
            Kind::Text,
            "output",
            true,
            format!("{count} lines"),
        ))
}

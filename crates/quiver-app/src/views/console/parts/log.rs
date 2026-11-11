//! The scrolling output: one row per entry. A daemon log record reads `time  LEVEL  message key=value`
//! and opens as JSON on click; a typed command, its output and the console's own notes are plain lines.

use gpui::prelude::*;
use gpui::{div, px, Context, FontWeight, Hsla, IntoElement, Rgba};
use quiver_state::console::entries::{ConsoleEntry, NewEntry, Stream, Tone};
use quiver_state::console::frames::{format_log_time, json_lines, FieldKind, LogLevel, LogRecord};
use quiver_state::console::Support;
use quiver_state::i18n::en::console;
use quiver_theme::{Color, FontFamily, Tokens};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, TrackedText};

use crate::views::console::ConsoleDock;

fn hsla(c: Color) -> Hsla {
    Rgba {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
    .into()
}

pub struct Colors {
    pub surface: Hsla,
    pub foreground: Hsla,
    pub dim: Hsla,
    pub hover: Hsla,
    pub open: Hsla,
    debug: Hsla,
    info: Hsla,
    warn: Hsla,
    error: Hsla,
    warn_text: Hsla,
    error_text: Hsla,
    number: Hsla,
    bool: Hsla,
    path: Hsla,
    field_error: Hsla,
}

impl Colors {
    pub fn of(t: &Tokens) -> Self {
        Self {
            surface: hsla(t.console),
            foreground: hsla(t.console_foreground),
            dim: hsla(t.console_dim),
            hover: hsla(t.console_row_hover),
            open: hsla(t.console_row_open),
            debug: hsla(t.log_debug),
            info: hsla(t.log_info),
            warn: hsla(t.log_warn),
            error: hsla(t.log_error),
            warn_text: hsla(t.log_warn_text),
            error_text: hsla(t.log_error_text),
            number: hsla(t.log_number),
            bool: hsla(t.log_bool),
            path: hsla(t.log_path),
            field_error: hsla(t.log_field_error),
        }
    }

    fn field(
        &self,
        kind: FieldKind,
    ) -> Hsla {
        match kind {
            FieldKind::Text => self.foreground,
            FieldKind::Number | FieldKind::Duration => self.number,
            FieldKind::Bool => self.bool,
            FieldKind::Path => self.path,
            FieldKind::Error => self.field_error,
        }
    }

    fn level(
        &self,
        level: LogLevel,
    ) -> Hsla {
        match level {
            LogLevel::Debug => self.debug,
            LogLevel::Info => self.info,
            LogLevel::Warn => self.warn,
            LogLevel::Error => self.error,
        }
    }

    fn message(
        &self,
        level: LogLevel,
    ) -> Hsla {
        match level {
            LogLevel::Warn => self.warn_text,
            LogLevel::Error => self.error_text,
            _ => self.foreground,
        }
    }
}

fn kind_of(entry: &ConsoleEntry) -> String {
    match &entry.body {
        NewEntry::Log(record) => format!("log:{:?}", record.level).to_lowercase(),
        NewEntry::Raw(_) => "raw".into(),
        NewEntry::Cmd { .. } => "cmd".into(),
        NewEntry::Out { stream, .. } => format!("out:{stream:?}").to_lowercase(),
        NewEntry::Note { tone, .. } => format!("note:{tone:?}").to_lowercase(),
    }
}

fn clock(
    ms: i64,
    offset_minutes: i32,
) -> String {
    let day_ms = (ms + i64::from(offset_minutes) * 60_000).rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        day_ms / 3_600_000,
        day_ms % 3_600_000 / 60_000,
        day_ms % 60_000 / 1000,
        day_ms % 1000
    )
}

fn level_label(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

fn log_row(
    c: &Colors,
    id: u64,
    record: &LogRecord,
    expanded: bool,
    offset: i32,
    this: &gpui::Entity<ConsoleDock>,
) -> gpui::AnyElement {
    let this = this.clone();
    let hover = c.hover;
    let message = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_wrap()
        .child(
            div()
                .text_color(c.message(record.level))
                .child(record.msg.clone()),
        )
        .children(record.fields.iter().map(|f| {
            div()
                .ml(px(10.))
                .flex()
                .child(div().text_color(c.dim).child(format!("{}=", f.key)))
                .child(
                    div()
                        .text_color(c.field(f.kind))
                        .when(f.kind == FieldKind::Path, |d| d.underline())
                        .child(f.value.clone()),
                )
        }));
    let line = div()
        .id(("console-line", id))
        .relative()
        .w_full()
        .flex()
        .gap(px(10.))
        .px(px(16.))
        .py(px(3.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .when(expanded, |d| d.bg(c.open))
        .on_click(move |_, _, cx| {
            this.update(cx, |dock, cx| {
                dock.controller.state.toggle_expanded(id);
                cx.notify();
            })
        })
        .child(
            div()
                .flex_none()
                .w(px(88.))
                .text_color(c.dim)
                .child(format_log_time(&record.iso, offset)),
        )
        .child(
            div().flex_none().w(px(44.)).child(
                // Old log row: font-semibold tracking-[0.04em] on the 12px mono line.
                TrackedText::new(
                    level_label(record.level),
                    12.,
                    18.,
                    0.04 * 12.,
                    c.level(record.level),
                )
                .weight(FontWeight::SEMIBOLD)
                .family(FontFamily::Mono.family()),
            ),
        )
        .child(message);
    let mut block = div().flex().flex_col().child(line);
    if expanded {
        let lines = json_lines(record);
        let last = lines.len().saturating_sub(1);
        block = block.child(
            div()
                .bg(c.open)
                .py(px(6.))
                .pr(px(16.))
                .pl(px(168.))
                .text_color(c.dim)
                .child("{")
                .children(lines.into_iter().enumerate().map(|(i, l)| {
                    div()
                        .pl(px(16.))
                        .flex()
                        .child(format!("\"{}\": ", l.key))
                        .child(div().text_color(c.field(l.kind)).child(l.value))
                        .when(i < last, |d| d.child(","))
                }))
                .child("}"),
        );
    }
    block.into_any_element()
}

fn output_row(
    text: String,
    color: Hsla,
) -> gpui::AnyElement {
    div()
        .pr(px(16.))
        .pl(px(114.))
        .min_h(px(18.))
        .text_color(color)
        .child(text)
        .into_any_element()
}

fn entry_row(
    c: &Colors,
    entry: &ConsoleEntry,
    expanded: bool,
    offset: i32,
    this: &gpui::Entity<ConsoleDock>,
    tr: &quiver_state::i18n::Translator,
) -> gpui::AnyElement {
    match &entry.body {
        NewEntry::Log(record) => log_row(c, entry.id, record, expanded, offset, this),
        NewEntry::Raw(text) => output_row(text.clone(), c.dim),
        NewEntry::Cmd { text, at } => div()
            .flex()
            .gap(px(10.))
            .px(px(16.))
            .pt(px(8.))
            .pb(px(2.))
            .text_size(px(12.5))
            .child(
                div()
                    .flex_none()
                    .w(px(88.))
                    .text_size(px(12.))
                    .whitespace_nowrap()
                    .text_color(c.dim)
                    .child(clock(*at, offset)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.foreground)
                    .child(format!("› {text}")),
            )
            .into_any_element(),
        NewEntry::Out { stream, text } => output_row(
            text.clone(),
            match stream {
                Stream::Stdout => c.foreground,
                Stream::Stderr => c.field_error,
            },
        ),
        NewEntry::Note { tone, note } => output_row(
            quiver_state::console::note_text(note, tr),
            match tone {
                Tone::Info => c.dim,
                Tone::Ok => c.bool,
                Tone::Error => c.field_error,
            },
        ),
    }
}

pub fn log(
    dock: &mut ConsoleDock,
    cx: &mut Context<ConsoleDock>,
) -> impl IntoElement {
    let theme = cx.quiver();
    let c = Colors::of(&Tokens::for_mode(theme.mode));
    let this = cx.entity();
    let state = &dock.controller.state;
    let unsupported = state.support == Support::Unsupported;
    let m = dock.app.read(cx);
    let rows: Vec<gpui::AnyElement> = state
        .entries
        .iter()
        .map(|e| {
            let expanded = state.expanded_id == Some(e.id);
            let kind = kind_of(e);
            let text = div()
                .relative()
                .child(entry_row(&c, e, expanded, 0, &this, &m.translator))
                .child(probe(
                    format!("console.line.{}", e.id),
                    Kind::Text,
                    quiver_state::console::lines::display_line(e, &m.translator, 0)
                        .spans
                        .iter()
                        .map(|s| s.text.as_str())
                        .collect::<String>(),
                    true,
                    kind,
                ));
            text.into_any_element()
        })
        .collect();
    let notice = unsupported.then(|| m.t(console::UNSUPPORTED));
    let count = rows.len();
    let body = div()
        .id("console-log")
        .size_full()
        .overflow_y_scroll()
        .track_scroll(&dock.scroll)
        .py(px(8.))
        .font_family(FontFamily::Mono.family())
        .text_size(px(12.))
        .line_height(px(18.))
        .text_color(c.foreground);
    let body = match notice {
        Some(text) => body
            .flex()
            .items_center()
            .justify_center()
            .px(px(24.))
            .text_color(c.dim)
            .child(text),
        None => body.children(rows),
    };
    div().relative().flex_1().min_h_0().child(body).child(probe(
        "console.output",
        Kind::Text,
        "output",
        true,
        format!("{count} lines"),
    ))
}

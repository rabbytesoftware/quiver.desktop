//! The command palette (port of `src/features/remote/components/command-palette.tsx`): the global,
//! always-reachable connection switcher. Opened by the connection chip in the top row; mounted once by the
//! shell. Its open / query / highlighted-row state is `AppModel::remote` (`quiver_state::remote::RemoteOverlays`).
//! Up and Down move the highlight (wrapping), Enter switches, Escape or a backdrop press closes.

use gpui::prelude::*;
use gpui::{
    div, px, relative, AnyElement, App, Context, Entity, Focusable as _, FontWeight, KeyDownEvent,
    Render, SharedString, Subscription, Window,
};
use gpui_component::input::Input as Editor;
use quiver_state::domain::{ConnectionConfig, ConnectionKind, ConnectionStatus};
use quiver_state::i18n::en::remote as keys;
use quiver_state::i18n::Params;
use quiver_state::remote::{clamp_index, filter_connections};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Dialog, Glyph, InputEvent, InputState, Palette, TrackedText};

use super::actions::{connect_to, DIALOGS};
use crate::model::{app_model, close_modal, AppModel};

/// Opens the palette, closing the Remote dialogs first so none is left stacked underneath it.
pub fn open_palette(cx: &mut App) {
    if app_model(cx).read(cx).remote.cmd_open {
        return;
    }
    for id in DIALOGS {
        close_modal(cx, id);
    }
    app_model(cx).update(cx, |m, cx| m.open_cmd(cx));
}

pub struct CommandPalette {
    app: Entity<AppModel>,
    input: Entity<InputState>,
    /// The palette was open when the model last changed: an open is the false-to-true edge.
    was_open: bool,
    /// Focus the field on the next frame (once the popup exists, so the dialog does not take it back).
    focus_pending: bool,
    _subscriptions: Vec<Subscription>,
}

impl CommandPalette {
    pub fn new(
        app: Entity<AppModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = app.read(cx).t(keys::COMMAND_PLACEHOLDER);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let on_input =
            cx.subscribe(
                &input,
                |this: &mut Self, input, event: &InputEvent, cx| match event {
                    InputEvent::Change => {
                        let query = input.read(cx).value().to_string();
                        this.app.update(cx, |m, cx| {
                            if m.remote.cmd_query != query {
                                m.remote.set_cmd_query(&query);
                                cx.notify();
                            }
                        });
                    }
                    InputEvent::PressEnter { .. } => this.select_highlighted(cx),
                    _ => {}
                },
            );
        let on_model = cx.observe_in(&app, window, |this: &mut Self, app, window, cx| {
            let open = app.read(cx).remote.cmd_open;
            if open && !this.was_open {
                this.input.update(cx, |i, cx| i.set_value("", window, cx));
                this.focus_pending = true;
            }
            this.was_open = open;
            cx.notify();
        });
        Self {
            app,
            input,
            was_open: false,
            focus_pending: false,
            _subscriptions: vec![on_input, on_model],
        }
    }

    fn filtered(
        &self,
        cx: &App,
    ) -> Vec<ConnectionConfig> {
        let m = self.app.read(cx);
        filter_connections(
            &m.services.connections.snapshot().connections,
            &m.remote.cmd_query,
        )
    }

    fn close(
        &self,
        cx: &mut App,
    ) {
        self.app.update(cx, |m, cx| m.close_cmd(cx));
    }

    /// Closes, then switches unless `id` is already the live connection.
    fn select(
        &self,
        id: String,
        name: String,
        cx: &mut App,
    ) {
        self.close(cx);
        let (active, status) = {
            let services = &self.app.read(cx).services;
            (
                services.connections.snapshot().active_id,
                services.status.status(),
            )
        };
        if id == active && status != ConnectionStatus::Disconnected {
            return;
        }
        connect_to(&self.app, id, name, cx);
    }

    fn select_highlighted(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let index = self.app.read(cx).remote.cmd_index;
        if let Some(item) = self.filtered(cx).into_iter().nth(index) {
            self.select(item.id, item.name, cx);
        }
    }

    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step = match event.keystroke.key.as_str() {
            "down" => 1,
            "up" => -1,
            _ => return,
        };
        cx.stop_propagation();
        let len = self.filtered(cx).len();
        self.app.update(cx, |m, cx| {
            let index = i64::try_from(m.remote.cmd_index).unwrap_or(0) + step;
            m.remote.cmd_index = clamp_index(index, len);
            cx.notify();
        });
    }
}

/// `KBD`: `inline-flex h-4.5 min-w-4.5 items-center justify-center rounded border border-border px-1
/// font-mono text-[10px] text-muted-foreground`.
fn kbd(
    text: &'static str,
    p: &Palette,
) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(18.))
        .min_w(px(18.))
        .px(px(4.))
        .rounded(px(4.))
        .border_1()
        .border_color(p.border)
        .font_family(FontFamily::Mono.family())
        .text_size(px(10.))
        .line_height(px(15.))
        .text_color(p.muted_foreground)
        .child(text)
}

/// One footer hint: `flex items-center gap-1.5 text-[11.5px] text-muted-foreground`.
fn hint(
    keys: &[&'static str],
    label: String,
    p: &Palette,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(11.5))
        .line_height(px(17.))
        .text_color(p.muted_foreground)
        .children(keys.iter().map(|k| kbd(k, p)))
        .child(label)
}

// Line heights are the old 1.5 rounded down to whole pixels, as WebKit laid the old rows out
// (16.5 -> 16, 19.5 + 17.25 -> 19 + 17): measured against the old app, the list is 2px shorter otherwise.

impl CommandPalette {
    fn row(
        &self,
        connection: &ConnectionConfig,
        highlighted: bool,
        current: Option<&str>,
        local_subtitle: &str,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = cx.quiver().palette;
        let glyph = match connection.kind {
            ConnectionKind::Local => Glyph::Monitor,
            ConnectionKind::Remote => Glyph::Globe,
        };
        let subtitle = match connection.kind {
            ConnectionKind::Local => local_subtitle.to_owned(),
            ConnectionKind::Remote => connection.url.clone().unwrap_or_default(),
        };
        let this = cx.entity();
        let (id, name) = (connection.id.clone(), connection.name.clone());
        let state = match (highlighted, current.is_some()) {
            (true, true) => "[selected] current",
            (true, false) => "[selected]",
            (false, true) => "current",
            (false, false) => "",
        };
        div()
            .id(SharedString::from(format!(
                "remote-cmd-item-{}",
                connection.id
            )))
            .relative()
            .flex()
            .w_full()
            .items_center()
            .gap(px(10.))
            .rounded(px(quiver_theme::Radius::MD))
            .px(px(8.))
            .py(px(8.))
            .cursor_pointer()
            .when(highlighted, |d| d.bg(p.accent))
            .on_click(move |_, _, cx| {
                let (id, name) = (id.clone(), name.clone());
                this.update(cx, |v, cx| v.select(id, name, cx));
            })
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(26.))
                    .rounded(px(quiver_theme::Radius::SM))
                    .bg(p.muted)
                    .child(icon(glyph, 14., p.muted_foreground)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(connection.name.clone()),
                    )
                    .child(
                        TrackedText::new(subtitle, 11.5, 17., 0., p.muted_foreground)
                            .family(FontFamily::Mono.family()),
                    ),
            )
            .when_some(current, |d, current| {
                d.child(
                    div()
                        .flex_none()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .text_color(p.muted_foreground)
                        .child(current.to_owned()),
                )
            })
            .child(probe(
                format!("remote.cmd.item.{}", connection.id),
                Kind::Button,
                connection.name.clone(),
                true,
                state,
            ))
            .into_any_element()
    }
}

impl Render for CommandPalette {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let m = self.app.read(cx);
        if !m.remote.cmd_open {
            return div().into_any_element();
        }
        if std::mem::take(&mut self.focus_pending) {
            let input = self.input.clone();
            window.on_next_frame(move |window, cx| input.update(cx, |i, cx| i.focus(window, cx)));
        }
        let p = cx.quiver().palette;
        let (group_label, current, local_subtitle, navigate, select, close) = (
            m.t(keys::COMMAND_GROUP_LABEL),
            m.t(keys::COMMAND_CURRENT),
            m.t(keys::LOCAL_SUBTITLE),
            m.t(keys::COMMAND_NAVIGATE),
            m.t(keys::COMMAND_SELECT),
            m.t(keys::COMMAND_CLOSE),
        );
        let query = m.remote.cmd_query.clone();
        let empty = m.translator.t_with(
            keys::COMMAND_EMPTY,
            &Params::new().with("query", query.as_str()),
        );
        let index = m.remote.cmd_index;
        let active = m.services.connections.snapshot().active_id;
        let filtered = self.filtered(cx);
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let input_state = if focused {
            format!("{query} [focused]")
        } else {
            query
        };

        let header = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .border_b_1()
            .border_color(p.border)
            .px(px(16.))
            .py(px(12.))
            .child(icon(Glyph::Search, 16., p.muted_foreground))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h(px(24.))
                    .on_key_down(cx.listener(Self::key_down))
                    .child(
                        Editor::new(&self.input)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .px_0()
                            .h(px(24.))
                            .text_size(px(14.5)),
                    )
                    .child(probe(
                        "remote.cmd.input",
                        Kind::Input,
                        group_label.clone(),
                        true,
                        input_state,
                    )),
            )
            .child(kbd("esc", &p));

        let list = div()
            .id("remote-cmd-list")
            .max_h(px(320.))
            .overflow_y_scroll()
            .p(px(6.));
        let list = if filtered.is_empty() {
            list.child(
                div()
                    .px(px(8.))
                    .py(px(28.))
                    .text_center()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.muted_foreground)
                    .child(empty),
            )
        } else {
            list.child(
                div().px(px(8.)).pt(px(4.)).pb(px(6.)).child(
                    // `text-[11px] font-semibold tracking-wide uppercase`: 0.025em is 0.275px at 11px.
                    TrackedText::new(
                        group_label.to_uppercase(),
                        11.,
                        16.,
                        0.275,
                        p.muted_foreground,
                    )
                    .weight(FontWeight::SEMIBOLD),
                ),
            )
            .children(filtered.iter().enumerate().map(|(i, connection)| {
                let is_current = connection.id == active;
                self.row(
                    connection,
                    i == index,
                    is_current.then_some(current.as_str()),
                    &local_subtitle,
                    cx,
                )
            }))
        };

        let footer = div()
            .flex()
            .items_center()
            .gap(px(16.))
            .border_t_1()
            .border_color(p.border)
            .bg(p.muted.opacity(0.55))
            .px(px(16.))
            .py(px(10.))
            .child(hint(&["↑", "↓"], navigate, &p))
            .child(hint(&["↵"], select, &p))
            .child(hint(&["esc"], close, &p));

        let app = self.app.clone();
        Dialog::new("remote-cmd")
            .open(true)
            .show_close(false)
            .width(480.)
            // `top-[18%] translate-y-0`: a relative offset of 18% of the grid row, which is the popup's own height.
            .top(relative(0.18))
            .on_dismiss(move |_, cx| app.update(cx, |m, cx| m.close_cmd(cx)))
            .child(header)
            .child(list)
            .child(footer)
            .child(probe("remote.cmd", Kind::Dialog, group_label, true, ""))
            .into_any_element()
    }
}

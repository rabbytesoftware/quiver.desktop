//! What the arrow follows, in the header's version chip (board `ArrowNoBannerAfter`: "url · v1.4.4.9 ⌄"), the
//! same control in and out of the library. The chip opens one dialog with the Channel and Version pickers: for an
//! arrow not in the library it chooses what Install / Add to Library use; for one in it, it is the switch flow.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, SharedString};
use quiver_state::arrow_details::channel::{ChannelKind, ChannelSelection};
use quiver_state::i18n::en::arrow;
use quiver_state::i18n::Params;
use quiver_state::namespace::with_selector;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader,
    DialogPanel, DialogTitle, Select, SelectOption, SelectSize,
};

use crate::views::arrow_details::ArrowDetailsView;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Not in the library: picks what Install / Add to Library use.
    Choose,
    /// In the library, nothing installed: re-registers, no uninstall.
    SwitchIdle,
    /// In the library and installed: uninstall and reinstall.
    Switch,
}

fn pick_channel(
    this: &mut ArrowDetailsView,
    name: &str,
) {
    if let Some(s) = this.switch_selection.as_mut() {
        s.select_channel(name);
    }
}

fn pick_version(
    this: &mut ArrowDetailsView,
    version: &str,
) {
    if let Some(s) = this.switch_selection.as_mut() {
        s.select_version(version);
    }
}

fn pickers(
    tr: quiver_state::i18n::Translator,
    id: &str,
    selection: &ChannelSelection,
    loading: bool,
    disabled: bool,
    cx: &mut Context<ArrowDetailsView>,
) -> Vec<gpui::AnyElement> {
    let mut out = Vec::new();
    let channel_options: Vec<_> = selection
        .channels()
        .iter()
        .map(|c| {
            let label = if c.kind == ChannelKind::Pointer {
                format!("{} ({})", c.name, tr.t(arrow::CHANNEL_POINTER))
            } else {
                c.name.clone()
            };
            SelectOption::new(c.name.clone(), label)
        })
        .collect();
    if loading || !channel_options.is_empty() {
        let this = cx.entity();
        out.push(
            Select::new(SharedString::from(format!("{id}.channel")))
                .options(channel_options)
                .value(
                    selection
                        .selected_channel()
                        .map(|s| SharedString::from(s.to_owned())),
                )
                .placeholder(if loading {
                    tr.t(arrow::LOADING)
                } else {
                    String::new()
                })
                .disabled(loading || disabled)
                // The old pickers, on the page and in the Switch dialog alike: `h-6 font-mono text-xs`.
                .size(SelectSize::Sm)
                .min_h(px(24.))
                .h(px(24.))
                .font_family(FontFamily::Mono.family())
                .text_size(px(12.))
                .on_change(move |value, _, cx| {
                    this.update(cx, |t, cx| {
                        pick_channel(t, &value);
                        cx.notify();
                    })
                })
                .into_any_element(),
        );
    }
    match selection.selected_channel_entry() {
        Some(c) if c.kind == ChannelKind::Ordered => {
            let this = cx.entity();
            out.push(
                Select::new(SharedString::from(format!("{id}.version")))
                    .options(
                        c.members
                            .iter()
                            .map(|m| SelectOption::new(m.clone(), m.clone()))
                            .collect(),
                    )
                    .value(
                        selection
                            .selected_version()
                            .map(|s| SharedString::from(s.to_owned())),
                    )
                    .disabled(disabled)
                    // The old pickers, on the page and in the Switch dialog alike: `h-6 font-mono text-xs`.
                    .size(SelectSize::Sm)
                    .min_h(px(24.))
                    .h(px(24.))
                    .font_family(FontFamily::Mono.family())
                    .text_size(px(12.))
                    .on_change(move |value, _, cx| {
                        this.update(cx, |t, cx| {
                            pick_version(t, &value);
                            cx.notify();
                        })
                    })
                    .into_any_element(),
            );
        }
        Some(c) => out.push(
            div()
                .font_family(FontFamily::Mono.family())
                .text_size(px(12.))
                .child(selection.selected_version().unwrap_or(&c.latest).to_owned())
                .into_any_element(),
        ),
        None => {}
    }
    out
}

pub fn switch_dialog(
    view: &ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> Option<gpui::AnyElement> {
    let selection = view.switch_selection.clone()?;
    let detail = view.merged(cx)?;
    let model = view.app.read(cx);
    let tr = model.translator;
    let loading = view.load.read(|l| l.channels_loading);
    let p = cx.quiver().palette;
    let target = selection.selector();
    // Outside the library the dialog only chooses what Install / Add to Library will use; in it, it is the
    // switch flow, which reinstalls only when something is installed.
    let mode = if !detail.user_installed {
        Mode::Choose
    } else if detail.state == quiver_state::domain::ArrowState::Absent {
        Mode::SwitchIdle
    } else {
        Mode::Switch
    };
    let current = match mode {
        Mode::Choose => view.selection.selector().unwrap_or_default(),
        _ => detail.selector.clone(),
    };
    let from = match mode {
        Mode::Choose => with_selector(&detail.namespace, &current),
        _ => detail.namespace.clone(),
    };
    let unchanged = target.as_deref().is_none_or(|t| t == current);
    let (title, note, confirm) = match mode {
        Mode::Choose => (
            arrow::SELECTOR_CHOOSE_TITLE,
            arrow::SELECTOR_CHOOSE_PICK,
            arrow::SELECTOR_CHOOSE_CONFIRM,
        ),
        Mode::SwitchIdle => (
            arrow::SELECTOR_SWITCH_TITLE_IDLE,
            arrow::SELECTOR_SWITCH_NOTE_IDLE,
            arrow::SELECTOR_SWITCH_CONFIRM_IDLE,
        ),
        Mode::Switch => (
            arrow::SELECTOR_SWITCH_TITLE,
            arrow::SELECTOR_SWITCH_NOTE,
            arrow::SELECTOR_SWITCH_CONFIRM,
        ),
    };
    let body = if loading {
        tr.t(arrow::LOADING)
    } else if unchanged || mode == Mode::Choose {
        let pick = if mode == Mode::Choose {
            arrow::SELECTOR_CHOOSE_PICK
        } else {
            arrow::SELECTOR_SWITCH_PICK
        };
        tr.t_with(pick, &Params::new().with("from", from.clone()))
    } else {
        tr.t_with(
            note,
            &Params::new().with("from", from.clone()).with(
                "to",
                with_selector(&detail.namespace, target.as_deref().unwrap_or_default()),
            ),
        )
    };
    let pending = view.switch_pending;
    let controls = pickers(tr, "arrow.switch-pick", &selection, loading, pending, cx);
    let error = view.register_error.clone().map(|reason| {
        tr.t_with(
            arrow::SELECTOR_REGISTER_FAILED,
            &Params::new().with("reason", reason),
        )
    });
    Some(
        Dialog::new("arrow.switch-dialog")
            .open(true)
            .on_dismiss({
                let this = cx.entity();
                move |_, cx| this.update(cx, |t, cx| t.close_switch(cx))
            })
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(tr.t(title)))
                    .child(DialogDescription::new().child(body)),
            )
            .child(
                DialogPanel::new()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .children(controls),
                    )
                    .children(error.map(|text| {
                        div()
                            .mt(px(12.))
                            .text_size(px(12.))
                            .text_color(p.destructive)
                            .child(text)
                    })),
            )
            .child(
                DialogFooter::new()
                    .child(
                        Button::new("arrow.switch-cancel")
                            .variant(ButtonVariant::Outline)
                            .label(tr.t(arrow::SELECTOR_SWITCH_CANCEL))
                            .on_click(cx.listener(|this, _, _, cx| this.close_switch(cx))),
                    )
                    .child(
                        Button::new("arrow.switch-confirm")
                            .variant(if mode == Mode::Switch {
                                ButtonVariant::Destructive
                            } else {
                                ButtonVariant::Default
                            })
                            .label(tr.t(confirm))
                            .disabled(unchanged || pending || loading)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if mode == Mode::Choose {
                                    this.choose_version(cx)
                                } else {
                                    this.confirm_switch(cx)
                                }
                            })),
                    ),
            )
            .child(probe(
                "arrow.switch-dialog",
                Kind::Dialog,
                tr.t(title),
                true,
                match mode {
                    Mode::Choose => "mode=choose",
                    Mode::SwitchIdle => "mode=switch-idle",
                    Mode::Switch => "mode=switch",
                },
            ))
            .into_any_element(),
    )
}

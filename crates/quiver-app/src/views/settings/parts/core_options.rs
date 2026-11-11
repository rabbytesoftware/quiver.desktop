//! Quiver Core's subsystems on the Developer tab (`quiver_state::settings::core_options`): a switch per flag
//! and a text field per count or duration, committed on blur or Enter like the port fields, each with a reset
//! that puts the daemon's default back.

use std::collections::HashMap;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Entity, Focusable, Subscription, Window};
use quiver_state::i18n::en::settings as keys;
use quiver_state::settings::core_options::{
    core_option_rows, option_patch, parse_option, CoreOptionRow, OptionValue,
};
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, Input, InputEvent, InputState, Switch};
use serde_json::Value;

use super::rows::{probed, reset_button, row_with_reset, section};
use crate::views::settings::SettingsView;

/// One text box per count / duration option, keyed by option id, with the value it last showed.
#[derive(Default)]
pub struct CoreInputs {
    fields: HashMap<&'static str, (Entity<InputState>, String)>,
    _subscriptions: Vec<Subscription>,
}

fn text_of(value: &OptionValue) -> Option<&str> {
    match value {
        OptionValue::Text(text) => Some(text),
        OptionValue::Toggle(_) => None,
    }
}

impl SettingsView {
    /// Creates the fields the first time the rows exist and re-reads them whenever the daemon's value changes
    /// (or refused the typed one, which puts the held value back).
    pub(in crate::views::settings) fn sync_core_inputs(
        &mut self,
        rows: &[CoreOptionRow],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for row in rows {
            let Some(text) = text_of(&row.value) else {
                continue;
            };
            let id = row.option.id;
            if let Some((input, shown)) = self.core_inputs.fields.get_mut(id) {
                if shown != text || row.problem.is_some() {
                    *shown = text.to_owned();
                    let text = text.to_owned();
                    input.update(cx, |i, cx| i.set_value(text, window, cx));
                }
                continue;
            }
            let input = cx.new(|cx| InputState::new(window, cx).default_value(text.to_owned()));
            let option = row.option;
            let subscription = cx.subscribe_in(
                &input,
                window,
                move |this: &mut Self,
                      input: &Entity<InputState>,
                      event: &InputEvent,
                      window,
                      cx| {
                    if !matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                        return;
                    }
                    let raw = input.read(cx).value().to_string();
                    let shown = this
                        .core_inputs
                        .fields
                        .get(option.id)
                        .map(|(_, s)| s.clone())
                        .unwrap_or_default();
                    match parse_option(option.kind, &raw) {
                        Some(value) if raw.trim() != shown => {
                            this.patch(option_patch(option.path, value), cx)
                        }
                        Some(_) => {}
                        None => input.update(cx, |i, cx| i.set_value(shown, window, cx)),
                    }
                },
            );
            self.core_inputs.fields.insert(id, (input, text.to_owned()));
            self.core_inputs._subscriptions.push(subscription);
        }
    }
}

fn option_row(
    view: &SettingsView,
    row: &CoreOptionRow,
    window: &Window,
    cx: &Context<SettingsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let m = view.app.read(cx);
    let option = row.option;
    let label = m.t(option.label);
    let id = format!("settings.config.{}", option.id);
    let this = cx.entity();
    let control = match &row.value {
        OptionValue::Toggle(on) => {
            let path = option.path;
            let this = this.clone();
            probed(
                Switch::new(gpui::SharedString::from(format!(
                    "settings-config-{}",
                    option.id
                )))
                .checked(*on)
                .on_change(move |next, _, cx| {
                    this.update(cx, |v, cx| {
                        v.patch(option_patch(path, Value::Bool(next)), cx)
                    })
                }),
                id.clone(),
                Kind::Switch,
                label.clone().into(),
                if *on { "checked" } else { "" }.into(),
            )
        }
        OptionValue::Text(text) => match view.core_inputs.fields.get(option.id) {
            Some((input, _)) => {
                let surface = (!cx.quiver().is_dark()).then_some(p.background);
                probed(
                    div().w(px(96.)).child(
                        Input::new(
                            gpui::SharedString::from(format!("settings-config-{}", option.id)),
                            input,
                        )
                        .h(px(28.))
                        .w(px(96.))
                        .when_some(surface, |i, color| i.bg(color)),
                    ),
                    id.clone(),
                    Kind::Other,
                    label.clone().into(),
                    if input.read(cx).focus_handle(cx).is_focused(window) {
                        format!("value={text} [focused]")
                    } else {
                        format!("value={text}")
                    },
                )
            }
            None => div().into_any_element(),
        },
    };
    let description = match (&row.problem, row.pending_restart) {
        (Some(problem), _) => problem.clone(),
        (None, true) => m.t(keys::CORE_PENDING_RESTART),
        (None, false) => m.t(option.description),
    };
    let path = option.path;
    let reset = reset_button(
        &p,
        &format!("{id}.reset"),
        m.t(keys::ROW_RESET).replace("{setting}", &label),
        row.can_reset,
        move |_, cx| this.update(cx, |v, cx| v.patch(option_patch(path, Value::Null), cx)),
    );
    row_with_reset(
        &p,
        &id,
        label,
        Some(description.into()),
        vec![control],
        Some(reset),
    )
}

/// The sections, once the daemon's config has loaded (nothing until then: Fletcher's row shows the loading line).
pub fn core_sections(
    view: &mut SettingsView,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> Vec<AnyElement> {
    let Some(sections) = core_option_rows(&view.engine) else {
        return Vec::new();
    };
    let all: Vec<CoreOptionRow> = sections
        .iter()
        .flat_map(|(_, rows)| rows.iter().cloned())
        .collect();
    view.sync_core_inputs(&all, window, cx);
    let p = cx.quiver().palette;
    sections
        .iter()
        .map(|(title, rows)| {
            let title = view.app.read(cx).t(*title);
            let rows = rows
                .iter()
                .map(|row| option_row(view, row, window, cx))
                .collect();
            section(&p, title, false, rows).into_any_element()
        })
        .collect()
}

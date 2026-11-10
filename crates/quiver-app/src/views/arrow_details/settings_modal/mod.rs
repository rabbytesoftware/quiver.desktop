//! The Settings modal: one row per declared variable, each edited with the control its type asks for
//! (`variable_form::field_kind`), with the daemon's bounds checked as you type. The values live in the
//! shared [`FormValues`]; an action reads them from there.

use std::collections::{HashMap, HashSet};

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, FontWeight, Render, SharedString, Subscription, Window};
use quiver_state::arrow_details::release_variables::{is_self_arrow, RELEASE_VARIABLE_NAMES};
use quiver_state::arrow_details::variable_form::{
    field_kind, format_number, range_hint, shown_value, validate, FieldKind,
};
use quiver_state::domain::{ArrowDetail, ArrowVariable};
use quiver_state::i18n::en::arrow as keys;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Button, ButtonSize, ButtonVariant, Dialog, DialogDescription, DialogFooter,
    DialogHeader, DialogPanel, DialogTitle, Input, InputEvent, InputState, NumberField, Select,
    SelectOption, Switch,
};

use super::parts::icons::{pict, Pict};
use super::values::FormValues;
use crate::model::{close_modal, AppModel};

mod controls;

const ID: &str = "arrow-settings";

/// The variables the person is asked for: every declared one, except the release URL and checksum on
/// Quiver's own row, which are resolved at click time and overwritten.
pub(super) fn visible_variables(detail: &ArrowDetail) -> Vec<ArrowVariable> {
    let own = is_self_arrow(&detail.namespace);
    detail
        .variables
        .iter()
        .filter(|v| !(own && RELEASE_VARIABLE_NAMES.contains(&v.name.as_str())))
        .cloned()
        .collect()
}

pub struct SettingsModal {
    app: Entity<AppModel>,
    variables: Vec<ArrowVariable>,
    values: Entity<FormValues>,
    inputs: HashMap<String, Entity<InputState>>,
    revealed: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsModal {
    pub fn new(
        app: Entity<AppModel>,
        _arrow: String,
        variables: Vec<ArrowVariable>,
        values: Entity<FormValues>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = vec![cx.observe(&values, |_, _, cx| cx.notify())];
        Self {
            app,
            variables,
            values,
            inputs: HashMap::new(),
            revealed: HashSet::new(),
            _subscriptions: subscriptions,
        }
    }

    fn set(values: &Entity<FormValues>, name: &str, value: String, cx: &mut gpui::App) {
        values.update(cx, |f, cx| {
            f.values.insert(name.to_owned(), value);
            cx.notify();
        });
    }

    /// The text editors are made on first paint: they need the window.
    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for variable in &self.variables {
            let secret = field_kind(variable) == FieldKind::Secret;
            if !(secret || field_kind(variable) == FieldKind::Text)
                || self.inputs.contains_key(&variable.name)
            {
                continue;
            }
            let start = shown_value(variable, &self.values.read(cx).values);
            let state = cx.new(|cx| InputState::new(window, cx).masked(secret));
            state.update(cx, |s, cx| s.set_value(start, window, cx));
            let (name, values) = (variable.name.clone(), self.values.clone());
            self._subscriptions.push(cx.subscribe_in(
                &state,
                window,
                move |_, input, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        let text = input.read(cx).value().to_string();
                        Self::set(&values, &name, text, cx);
                    }
                },
            ));
            self.inputs.insert(variable.name.clone(), state);
        }
    }

    fn row(&self, variable: &ArrowVariable, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let shown = shown_value(variable, &self.values.read(cx).values);
        let error = validate(variable, &shown).map(|e| e.message());
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .py(px(12.))
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(variable.name.clone()),
                            )
                            .when(!variable.description.is_empty(), |d| {
                                d.child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(12.))
                                        .text_color(p.muted_foreground)
                                        .child(variable.description.clone()),
                                )
                            }),
                    )
                    .child(div().flex_none().child(self.control(variable, cx))),
            )
            .when_some(error, |d, message| {
                d.child(
                    div()
                        .flex()
                        .justify_end()
                        .text_size(px(11.5))
                        .text_color(p.destructive)
                        .child(message),
                )
            })
    }
}

impl Render for SettingsModal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let model = self.app.read(cx);
        let (title, subtitle, done) = (
            model.t(keys::SETTINGS_TITLE),
            model.t(keys::SETTINGS_SUBTITLE),
            model.t(keys::SETTINGS_DONE),
        );
        let variables = self.variables.clone();
        let rows: Vec<_> = variables.iter().map(|v| self.row(v, cx)).collect();
        Dialog::new(ID)
            .open(true)
            .width(520.)
            .on_dismiss(|_, cx| close_modal(cx, ID))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(title))
                    .child(DialogDescription::new().child(subtitle)),
            )
            .child(
                DialogPanel::new().child(
                    div()
                        .id("arrow.settings.list")
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .children(rows),
                ),
            )
            .child(
                DialogFooter::new().child(
                    Button::new("arrow.settings.done")
                        .label(done.clone())
                        .on_click(|_, _, cx| close_modal(cx, ID))
                        .child(probe("arrow.settings.done", Kind::Button, done, true, "")),
                ),
            )
            .child(probe("arrow.settings", Kind::Dialog, "Settings", true, ""))
    }
}

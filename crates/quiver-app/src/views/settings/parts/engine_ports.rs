//! The engine tab's ports row: HTTP and WebSocket port fields with Apply.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Entity, IntoElement, Subscription, Window};
use quiver_state::i18n::en::settings as keys;
use quiver_state::settings::engine::{parse_port_entry, port_patch};
use quiver_state::settings::EngineModel;
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, Input, InputEvent, InputState};
use serde_json::json;

use super::rows::{probed, reset_button, row_with_reset};
use crate::views::settings::SettingsView;

/// The two port fields: uncontrolled text boxes committed on blur or Enter, like the old app's
/// `<input type="number" defaultValue>`; re-read from the daemon's value whenever that changes.
pub struct PortInputs {
    start: Entity<InputState>,
    end: Entity<InputState>,
    shown: (Option<i64>, Option<i64>),
    _subscriptions: Vec<Subscription>,
}

fn text(value: Option<i64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn commit(
    view: &mut SettingsView,
    key: &'static str,
    input: &Entity<InputState>,
    configured: Option<i64>,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) {
    let raw = input.read(cx).value().to_string();
    match parse_port_entry(&raw) {
        Some(port) if Some(port) != configured => view.patch(port_patch(key, port), cx),
        Some(_) => {}
        None => input.update(cx, |i, cx| i.set_value(text(configured), window, cx)),
    }
}

impl SettingsView {
    pub(in crate::views::settings) fn sync_port_inputs(
        &mut self,
        model: &EngineModel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let now = (model.ports.start, model.ports.end);
        if let Some(inputs) = &mut self.ports {
            if inputs.shown != now || model.ports.problem.is_some() {
                inputs.shown = now;
                inputs
                    .start
                    .update(cx, |i, cx| i.set_value(text(now.0), window, cx));
                inputs
                    .end
                    .update(cx, |i, cx| i.set_value(text(now.1), window, cx));
            }
            return;
        }
        let start = cx.new(|cx| InputState::new(window, cx).default_value(text(now.0)));
        let end = cx.new(|cx| InputState::new(window, cx).default_value(text(now.1)));
        let handler =
            |key: &'static str, pick: fn(&PortInputs) -> (Entity<InputState>, Option<i64>)| {
                move |this: &mut Self,
                      _: &Entity<InputState>,
                      event: &InputEvent,
                      window: &mut Window,
                      cx: &mut Context<Self>| {
                    if !matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                        return;
                    }
                    let Some((input, configured)) = this.ports.as_ref().map(pick) else {
                        return;
                    };
                    commit(this, key, &input, configured, window, cx);
                }
            };
        let subscriptions = vec![
            cx.subscribe_in(
                &start,
                window,
                handler("ephemeral_port_start", |p| (p.start.clone(), p.shown.0)),
            ),
            cx.subscribe_in(
                &end,
                window,
                handler("ephemeral_port_end", |p| (p.end.clone(), p.shown.1)),
            ),
        ];
        self.ports = Some(PortInputs {
            start,
            end,
            shown: now,
            _subscriptions: subscriptions,
        });
    }
}

pub(super) fn ports_row(
    view: &SettingsView,
    model: &EngineModel,
    cx: &Context<SettingsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let m = view.app.read(cx);
    // The inputs are created when the Engine tab opens; until then there is no row to draw.
    let Some((start, end)) = view
        .ports
        .as_ref()
        .map(|i| (i.start.clone(), i.end.clone()))
    else {
        return div().into_any_element();
    };
    let surface = (!cx.quiver().is_dark()).then_some(p.background);
    let field = |id: &'static str, state: &Entity<InputState>| {
        div().w(px(86.)).child(
            Input::new(id, state)
                .h(px(28.))
                .w(px(86.))
                .when_some(surface, |input, color| input.bg(color)),
        )
    };
    let description = match &model.ports.problem {
        Some((which, why)) => m
            .translator
            .t_with(
                keys::ENGINE_PORTS_FIELD_REJECTED,
                &quiver_state::i18n::Params::new()
                    .with("field", m.t(which.label()))
                    .with("message", why.as_str()),
            )
            .into(),
        None => m.t(keys::ENGINE_PORTS_DESCRIPTION).into(),
    };
    let this_reset = this.clone();
    let controls = vec![
        probed(
            field("settings-port-start", &start),
            "settings.engine.port-start".into(),
            Kind::Other,
            m.t(keys::ENGINE_PORTS_LOWEST).into(),
            format!("value={}", model.ports.start.unwrap_or_default()),
        ),
        div()
            .text_color(p.foreground_subtle)
            .child("–")
            .into_any_element(),
        probed(
            field("settings-port-end", &end),
            "settings.engine.port-end".into(),
            Kind::Other,
            m.t(keys::ENGINE_PORTS_HIGHEST).into(),
            format!("value={}", model.ports.end.unwrap_or_default()),
        ),
    ];
    let controls_reset = reset_button(
        &p,
        "settings.engine.ports.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::ENGINE_PORTS_LABEL)),
        model.ports.can_reset,
        move |_, cx| {
            this_reset.update(cx, |v, cx| {
                v.patch(json!({"netbridge": {"ephemeral_port_start": null, "ephemeral_port_end": null}}), cx)
            })
        },
    );
    row_with_reset(
        &p,
        "settings.engine.ports",
        m.t(keys::ENGINE_PORTS_LABEL),
        Some(description),
        controls,
        Some(controls_reset),
    )
}

//! The engine tab's ports row: HTTP and WebSocket port fields with Apply.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, IntoElement};
use quiver_state::i18n::en::settings as keys;
use quiver_state::settings::engine::{parse_port_entry, port_patch};
use quiver_state::settings::EngineModel;
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, NumberField};
use serde_json::json;

use super::rows::{probed, reset_button, row};
use crate::views::settings::SettingsView;

pub(super) fn ports_row(
    view: &SettingsView,
    model: &EngineModel,
    cx: &Context<SettingsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let m = view.app.read(cx);
    let field = |id: &'static str, key: &'static str, value: Option<i64>| {
        let this = this.clone();
        div().w(px(132.)).child(
            NumberField::new(id)
                .value(value.map(|v| v as f64))
                .range(1., 65535.)
                .on_change(move |next, _, cx| {
                    this.update(cx, |v, cx| {
                        match next.and_then(|n| parse_port_entry(&n.to_string())) {
                            Some(port) => v.patch(port_patch(key, port), cx),
                            None => cx.notify(),
                        }
                    });
                }),
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
    let mut controls = vec![
        probed(
            field(
                "settings-port-start",
                "ephemeral_port_start",
                model.ports.start,
            ),
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
            field("settings-port-end", "ephemeral_port_end", model.ports.end),
            "settings.engine.port-end".into(),
            Kind::Other,
            m.t(keys::ENGINE_PORTS_HIGHEST).into(),
            format!("value={}", model.ports.end.unwrap_or_default()),
        ),
    ];
    controls.extend(reset_button(
        "settings.engine.ports.reset",
        m.t(keys::ROW_RESET).replace("{setting}", &m.t(keys::ENGINE_PORTS_LABEL)),
        model.ports.can_reset,
        move |_, cx| {
            this_reset.update(cx, |v, cx| {
                v.patch(json!({"netbridge": {"ephemeral_port_start": null, "ephemeral_port_end": null}}), cx)
            })
        },
    ));
    row(
        &p,
        "settings.engine.ports",
        m.t(keys::ENGINE_PORTS_LABEL),
        Some(description),
        controls,
    )
}

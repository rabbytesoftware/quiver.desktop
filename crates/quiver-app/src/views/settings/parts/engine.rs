//! Engine tab: ports, logs, updates (Core's channel and its update rows) and the command line.
//! `build_engine_model` decides what each row says; this lays it out and sends the patches.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, IntoElement, SharedString};
use quiver_state::i18n::en::{screens, settings as keys};
use quiver_state::settings::{build_engine_model, normalise_level, EngineModel};
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, ButtonVariant, Select, SelectOption, Switch};
use serde_json::json;

use super::notices::{banner, failure};
use super::rows::{probed, reset_button, row, section, small_button};
use super::updates::{update_rows, Which};
use crate::views::settings::SettingsView;

fn level_options(
    view: &SettingsView,
    cx: &Context<SettingsView>,
) -> Vec<SelectOption> {
    let m = view.app.read(cx);
    [
        ("debug", keys::ENGINE_LOGS_LEVEL_DEBUG),
        ("info", keys::ENGINE_LOGS_LEVEL_INFO),
        ("warn", keys::ENGINE_LOGS_LEVEL_WARN),
        ("error", keys::ENGINE_LOGS_LEVEL_ERROR),
    ]
    .into_iter()
    .map(|(value, key)| SelectOption::new(value, m.t(key)))
    .collect()
}

fn logs(
    view: &SettingsView,
    model: &EngineModel,
    cx: &Context<SettingsView>,
) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let m = view.app.read(cx);
    let disk_this = this.clone();
    let mut disk = vec![probed(
        Switch::new("settings-logs-disk")
            .checked(model.logs.disk_enabled)
            .on_change({
                let this = this.clone();
                move |on, _, cx| {
                    this.update(cx, |v, cx| v.patch(json!({"logger": {"enabled": on}}), cx))
                }
            }),
        "settings.engine.logs-disk".into(),
        Kind::Switch,
        m.t(keys::ENGINE_LOGS_DISK).into(),
        if model.logs.disk_enabled {
            "checked"
        } else {
            ""
        }
        .into(),
    )];
    disk.extend(reset_button(
        "settings.engine.logs-disk.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::ENGINE_LOGS_DISK)),
        model.logs.disk_can_reset,
        move |_, cx| {
            disk_this.update(cx, |v, cx| {
                v.patch(json!({"logger": {"enabled": null}}), cx)
            })
        },
    ));
    let level = model.logs.level;
    let level_this = this.clone();
    let mut level_controls = vec![probed(
        div().w(px(120.)).child(
            Select::new("settings-logs-level")
                .options(level_options(view, cx))
                .value(Some(level.as_str().into()))
                .on_change({
                    let this = this.clone();
                    move |value, _, cx| {
                        let next = normalise_level(&value).as_str();
                        this.update(cx, |v, cx| v.patch(json!({"logger": {"level": next}}), cx))
                    }
                }),
        ),
        "settings.engine.logs-level".into(),
        Kind::Select,
        m.t(keys::ENGINE_LOGS_LEVEL).into(),
        format!("value={}", level.as_str()),
    )];
    level_controls.extend(reset_button(
        "settings.engine.logs-level.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::ENGINE_LOGS_LEVEL)),
        model.logs.level_can_reset,
        move |_, cx| level_this.update(cx, |v, cx| v.patch(json!({"logger": {"level": null}}), cx)),
    ));
    let level_description: SharedString = model
        .logs
        .level_problem
        .clone()
        .unwrap_or_else(|| m.t(keys::ENGINE_LOGS_LEVEL_DESCRIPTION))
        .into();
    vec![
        row(
            &p,
            "settings.engine.logs-disk",
            m.t(keys::ENGINE_LOGS_DISK),
            Some(m.t(keys::ENGINE_LOGS_DISK_DESCRIPTION).into()),
            disk,
        ),
        row(
            &p,
            "settings.engine.logs-level",
            m.t(keys::ENGINE_LOGS_LEVEL),
            Some(level_description),
            level_controls,
        ),
    ]
}

fn channel_row(
    view: &SettingsView,
    model: &EngineModel,
    cx: &Context<SettingsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let m = view.app.read(cx);
    let label = m.t(keys::ENGINE_SELF_UPDATE_CHANNEL);
    match &view.channels {
        None => row(
            &p,
            "settings.engine.channel",
            label,
            Some(m.t(keys::ENGINE_SELF_UPDATE_LOADING).into()),
            vec![],
        ),
        Some(Err(_)) => {
            let retry = small_button(
                "settings.engine.channel.retry",
                m.t(keys::ENGINE_RETRY),
                ButtonVariant::Outline,
                true,
                move |_, cx| this.update(cx, |v, cx| v.load_channels(cx)),
            );
            row(
                &p,
                "settings.engine.channel",
                label,
                Some(m.t(screens::SETTINGS_CHANNEL_FAILED).into()),
                vec![
                    div()
                        .text_size(px(13.))
                        .text_color(p.muted_foreground)
                        .child(m.t(keys::ENGINE_SELF_UPDATE_UNSET))
                        .into_any_element(),
                    retry,
                ],
            )
        }
        Some(Ok(channels)) => {
            let options = channels
                .iter()
                .map(|c| SelectOption::new(c.name.clone(), c.name.clone()))
                .collect();
            let current = model.self_update.channel.clone();
            let reset_this = this.clone();
            let mut controls = vec![probed(
                div().w(px(150.)).child(
                    Select::new("settings-channel")
                        .options(options)
                        .value(current.clone().map(Into::into))
                        .placeholder(m.t(keys::ENGINE_SELF_UPDATE_UNSET))
                        .on_change(move |value, _, cx| {
                            let value = value.to_string();
                            this.update(cx, |v, cx| {
                                v.patch(json!({"arrows": {"self_update_channel": value}}), cx)
                            })
                        }),
                ),
                "settings.engine.channel-select".into(),
                Kind::Select,
                label.clone().into(),
                format!("value={}", current.unwrap_or_default()),
            )];
            controls.extend(reset_button(
                "settings.engine.channel.reset",
                m.t(keys::ROW_RESET).replace("{setting}", &label),
                model.self_update.can_reset,
                move |_, cx| {
                    reset_this.update(cx, |v, cx| {
                        v.patch(json!({"arrows": {"self_update_channel": null}}), cx)
                    })
                },
            ));
            let description = model
                .self_update
                .problem
                .clone()
                .unwrap_or_else(|| m.t(keys::ENGINE_SELF_UPDATE_CHANNEL_DESCRIPTION));
            row(
                &p,
                "settings.engine.channel",
                label,
                Some(description.into()),
                controls,
            )
        }
    }
}

pub fn engine(
    view: &mut SettingsView,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let (loading, retry_label, restart, corrected, logs_title, updates_title, path_title) = {
        let m = view.app.read(cx);
        (
            m.t(keys::ENGINE_LOADING),
            m.t(keys::ENGINE_RETRY),
            m.t(keys::ENGINE_RESTART),
            m.t(keys::ENGINE_CORRECTED),
            m.t(keys::ENGINE_LOGS_TITLE),
            m.t(keys::ENGINE_SELF_UPDATE_TITLE),
            m.t(keys::ENGINE_PATH_TITLE),
        )
    };
    let Some(model) = build_engine_model(&view.engine) else {
        let text = view.engine.error.clone().unwrap_or(loading);
        let this = cx.entity();
        let retry = view.engine.error.is_some().then(|| {
            small_button(
                "settings.engine.retry",
                retry_label,
                ButtonVariant::Outline,
                true,
                move |_, cx| this.update(cx, |v, cx| v.load_engine(cx)),
            )
        });
        return div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(12.))
            .text_size(px(13.))
            .text_color(p.muted_foreground)
            .child(text)
            .children(retry)
            .into_any_element();
    };
    let mut notices: Vec<AnyElement> = Vec::new();
    if model.restart_required {
        notices.push(banner(&p, restart).into_any_element());
    }
    if let Some(list) = &model.corrected {
        notices.push(banner(&p, corrected.replace("{settings}", list)).into_any_element());
    }
    if let Some(error) = &model.patch_error {
        notices.push(failure(&p, error.clone()).into_any_element());
    }
    let mut updates = vec![channel_row(view, &model, cx)];
    updates.extend(update_rows(view, Which::Core, cx));
    let command_line = super::command_line::command_line(view, cx);
    div()
        .flex()
        .flex_col()
        .children(notices)
        .child(
            div()
                .mb(px(0.))
                .children(vec![super::engine_ports::ports_row(view, &model, cx)]),
        )
        .child(section(&p, logs_title, false, logs(view, &model, cx)))
        .child(section(&p, updates_title, false, updates))
        .child(section(&p, path_title, false, command_line))
        .into_any_element()
}

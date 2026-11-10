//! The "Use Quiver commands in a terminal" row: whether Core's command directory is on `PATH`, and the
//! explicit button that sets it up (Core never edits `PATH` on its own).

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context};
use quiver_state::i18n::en::settings as keys;
use quiver_state::settings::{path_state, PathState};
use quiver_ui::{ActiveQuiver, ButtonVariant};

use super::rows::{row, small_button};
use crate::views::settings::SettingsView;

pub fn command_line(view: &SettingsView, cx: &Context<SettingsView>) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let m = view.app.read(cx);
    let label = m.t(keys::ENGINE_PATH_LABEL);
    let this = cx.entity();
    let status = &view.path;
    let Some(info) = &status.status else {
        // 404: a daemon that predates the route. Nothing to offer, so nothing is drawn.
        if status.unavailable {
            return vec![];
        }
        let text = status
            .error
            .clone()
            .unwrap_or_else(|| m.t(keys::ENGINE_PATH_LOADING));
        return vec![row(
            &p,
            "settings.engine.path",
            label,
            Some(text.into()),
            vec![],
        )];
    };
    let state = path_state(info);
    let mut description = match state {
        PathState::Setup => m
            .t(keys::ENGINE_PATH_DESCRIPTION_SETUP)
            .replace("{dir}", &info.bin_dir),
        PathState::Restart => m.t(keys::ENGINE_PATH_DESCRIPTION_RESTART),
        PathState::Ready => m.t(keys::ENGINE_PATH_DESCRIPTION_READY),
    };
    let mut lines = vec![description.clone()];
    if !info.files.is_empty() {
        lines.push(
            m.t(keys::ENGINE_PATH_FILES)
                .replace("{files}", &info.files.join(", ")),
        );
    }
    if let Some(error) = &status.setup_error {
        lines.push(error.clone());
    }
    description = lines.join(" ");
    let control = match state {
        PathState::Setup => small_button(
            "settings.engine.path.setup",
            m.t(keys::ENGINE_PATH_SETUP),
            ButtonVariant::Outline,
            !status.setting_up,
            move |_, cx| this.update(cx, |v, cx| v.run_setup_path(cx)),
        ),
        PathState::Restart | PathState::Ready => div()
            .text_size(px(13.))
            .text_color(p.muted_foreground)
            .child(m.t(if state == PathState::Ready {
                keys::ENGINE_PATH_STATE_READY
            } else {
                keys::ENGINE_PATH_STATE_RESTART
            }))
            .into_any_element(),
    };
    vec![row(
        &p,
        "settings.engine.path",
        label,
        Some(description.into()),
        vec![control],
    )]
}

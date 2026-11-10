//! Developer tab (debug builds): the build indicator, the daemon console, Quiver's own components in
//! lists, and automatic arrow registration (Fletcher).

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, IntoElement};
use quiver_state::i18n::en::settings as keys;
use quiver_state::settings::auto_register::{auto_register_patch, auto_register_reset_patch};
use quiver_state::settings::engine_model::AutoRegisterNote;
use quiver_state::settings::preferences::{save_build_indicator, save_show_self_components};
use quiver_state::settings::{build_engine_model, EngineModel};
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, ButtonVariant, Switch};

use super::rows::{probed, reset_button, row, section, small_button};
use crate::model::build_indicator;
use crate::views::settings::SettingsView;

fn switch(
    id: &'static str,
    label: String,
    on: bool,
    change: impl Fn(bool, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    probed(
        Switch::new(id).checked(on).on_change(change),
        format!("settings.{id}"),
        Kind::Switch,
        label.into(),
        if on { "checked" } else { "" }.into(),
    )
}

fn build_section(view: &SettingsView, cx: &Context<SettingsView>) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let app = view.app.clone();
    let m = app.read(cx);
    let setting = build_indicator(m);
    let toggle = {
        let app = app.clone();
        switch(
            "indicator",
            m.t(keys::GENERAL_BUILD_INDICATOR),
            setting.shown(),
            move |on, _, cx| {
                app.update(cx, |model, cx| {
                    let mut next = build_indicator(model);
                    next.set(on);
                    save_build_indicator(model.services.key_value.as_ref(), &next);
                    cx.notify();
                });
            },
        )
    };
    let mut indicator = vec![toggle];
    indicator.extend(reset_button(
        "settings.indicator.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::GENERAL_BUILD_INDICATOR)),
        setting.changed(),
        {
            let app = app.clone();
            move |_, cx| {
                app.update(cx, |model, cx| {
                    let mut next = build_indicator(model);
                    next.reset();
                    save_build_indicator(model.services.key_value.as_ref(), &next);
                    cx.notify();
                })
            }
        },
    ));
    let open = small_button(
        "settings.console.open",
        m.t(keys::GENERAL_BUILD_OPEN_CONSOLE),
        ButtonVariant::Outline,
        true,
        {
            let app = app.clone();
            move |_, cx| {
                app.update(cx, |model, cx| {
                    model.console_open = true;
                    cx.notify();
                })
            }
        },
    );
    vec![
        row(
            &p,
            "settings.indicator",
            m.t(keys::GENERAL_BUILD_INDICATOR),
            Some(m.t(keys::GENERAL_BUILD_INDICATOR_DESCRIPTION).into()),
            indicator,
        ),
        row(
            &p,
            "settings.console",
            m.t(keys::GENERAL_BUILD_CONSOLE),
            Some(m.t(keys::GENERAL_BUILD_CONSOLE_DESCRIPTION).into()),
            vec![open],
        ),
    ]
}

fn catalog_section(view: &SettingsView, cx: &Context<SettingsView>) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let app = view.app.clone();
    let m = app.read(cx);
    let shown = m.show_self_components;
    let set = |show: bool| {
        let app = app.clone();
        move |_: &mut gpui::Window, cx: &mut gpui::App| {
            app.update(cx, |model, cx| {
                model.show_self_components = show;
                save_show_self_components(model.services.key_value.as_ref(), show);
                cx.notify();
            })
        }
    };
    let toggle = switch("own-components", m.t(keys::GENERAL_LIBRARY_LABEL), shown, {
        let app = app.clone();
        move |on, _, cx| {
            app.update(cx, |model, cx| {
                model.show_self_components = on;
                save_show_self_components(model.services.key_value.as_ref(), on);
                cx.notify();
            })
        }
    });
    let mut controls = vec![toggle];
    controls.extend(reset_button(
        "settings.own-components.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::GENERAL_LIBRARY_LABEL)),
        !shown,
        set(true),
    ));
    vec![row(
        &p,
        "settings.own-components",
        m.t(keys::GENERAL_LIBRARY_LABEL),
        Some(m.t(keys::GENERAL_LIBRARY_DESCRIPTION).into()),
        controls,
    )]
}

/// Fletcher: the one switch for automatic arrow registration. It lives in Core's config, so it follows
/// the engine state (loaded the first time this tab or Engine opens).
fn auto_register_row(
    view: &SettingsView,
    model: &EngineModel,
    cx: &Context<SettingsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let this = cx.entity();
    let m = view.app.read(cx);
    let description = match &model.auto_register.note {
        AutoRegisterNote::Rejected(why) => why.clone(),
        AutoRegisterNote::Pending => m.t(keys::ENGINE_AUTO_REGISTER_PENDING),
        AutoRegisterNote::Normal => m.t(keys::ENGINE_AUTO_REGISTER_DESCRIPTION),
    };
    let toggle = {
        let this = this.clone();
        switch(
            "autoregister",
            m.t(keys::ENGINE_AUTO_REGISTER_LABEL),
            model.auto_register.on,
            move |on, _, cx| this.update(cx, |v, cx| v.patch(auto_register_patch(on), cx)),
        )
    };
    let mut controls = vec![toggle];
    controls.extend(reset_button(
        "settings.autoregister.reset",
        m.t(keys::ROW_RESET)
            .replace("{setting}", &m.t(keys::ENGINE_AUTO_REGISTER_LABEL)),
        model.auto_register.can_reset,
        move |_, cx| this.update(cx, |v, cx| v.patch(auto_register_reset_patch(), cx)),
    ));
    row(
        &p,
        "settings.autoregister",
        m.t(keys::ENGINE_AUTO_REGISTER_LABEL),
        Some(description.into()),
        controls,
    )
}

pub fn developer(view: &mut SettingsView, cx: &mut Context<SettingsView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let (builds, library, arrows, loading) = {
        let m = view.app.read(cx);
        (
            m.t(keys::GENERAL_BUILD_TITLE),
            m.t(keys::GENERAL_LIBRARY_TITLE),
            m.t(keys::ENGINE_AUTO_REGISTER_TITLE),
            m.t(keys::ENGINE_LOADING),
        )
    };
    let fletcher = match build_engine_model(&view.engine) {
        Some(model) => vec![auto_register_row(view, &model, cx)],
        None => vec![div()
            .text_size(px(13.))
            .text_color(p.muted_foreground)
            .child(loading)
            .into_any_element()],
    };
    div()
        .flex()
        .flex_col()
        .child(section(&p, builds, true, build_section(view, cx)))
        .child(section(&p, library, false, catalog_section(view, cx)))
        .child(section(&p, arrows, false, fletcher))
}

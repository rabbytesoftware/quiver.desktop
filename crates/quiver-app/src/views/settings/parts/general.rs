//! General tab: appearance (theme, rail side), language and, in a debug build, a pointer to the
//! Developer tab where the build, console and catalog settings moved.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, SharedString};
use quiver_state::i18n::en::{locale_meta, screens, settings as keys};
use quiver_state::i18n::{normalise_preference, translate_in, LocalePreference, LOCALES};
use quiver_state::shell::{
    normalise_side, normalise_theme_preference, SidebarSide, ThemePreference,
};
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, Select, SelectOption};

use super::appearance::{set_locale, set_side, set_theme};
use super::rows::{probed, row, section};
use crate::views::settings::SettingsView;

fn choice(
    id: &'static str,
    options: Vec<SelectOption>,
    value: &str,
    width: f32,
    label: SharedString,
    on_change: impl Fn(SharedString, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::AnyElement {
    let select = Select::new(id)
        .options(options)
        .value(Some(value.to_owned().into()))
        .on_change(on_change);
    probed(
        div().w(px(width.max(144.))).child(select),
        format!("settings.{id}"),
        Kind::Select,
        label,
        format!("value={value}"),
    )
}

pub fn general(
    view: &mut SettingsView,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let app = view.app.clone();
    let m = app.read(cx);
    let t = |key| m.t(key);
    let (theme, side) = (m.theme, m.shell.sidebar_side);
    let preference =
        quiver_state::settings::preferences::load_locale(m.services.key_value.as_ref());

    let themes = vec![
        SelectOption::new("system", t(keys::GENERAL_THEME_SYSTEM)),
        SelectOption::new("light", t(keys::GENERAL_THEME_LIGHT)),
        SelectOption::new("dark", t(keys::GENERAL_THEME_DARK)),
    ];
    let sides = vec![
        SelectOption::new("left", t(keys::GENERAL_SIDEBAR_LEFT)),
        SelectOption::new("right", t(keys::GENERAL_SIDEBAR_RIGHT)),
    ];
    let language_name = |locale| translate_in(locale, locale_meta::NAME, None);
    let detected = quiver_state::i18n::detect_locale(&[]);
    let mut locales = vec![SelectOption::new(
        "system",
        m.translator.t_with(
            keys::GENERAL_LANGUAGE_SYSTEM,
            &quiver_state::i18n::Params::new().with("language", language_name(detected)),
        ),
    )];
    locales.extend(
        LOCALES
            .iter()
            .map(|l| SelectOption::new(l.tag(), language_name(*l))),
    );

    let appearance = vec![
        row(
            &p,
            "settings.theme",
            t(keys::GENERAL_THEME_LABEL),
            Some(t(keys::GENERAL_THEME_DESCRIPTION).into()),
            vec![choice(
                "theme",
                themes,
                theme.as_stored(),
                132.,
                t(keys::GENERAL_THEME_LABEL).into(),
                {
                    let app = app.clone();
                    move |value, window, cx| {
                        let next: ThemePreference = normalise_theme_preference(Some(&value));
                        set_theme(&app, next, window, cx);
                    }
                },
            )],
        ),
        row(
            &p,
            "settings.sidebar",
            t(keys::GENERAL_SIDEBAR_LABEL),
            Some(t(keys::GENERAL_SIDEBAR_DESCRIPTION).into()),
            vec![choice(
                "sidebar",
                sides,
                side.as_stored(),
                132.,
                t(keys::GENERAL_SIDEBAR_LABEL).into(),
                {
                    let app = app.clone();
                    move |value, _, cx| {
                        let next: SidebarSide = normalise_side(Some(&value));
                        set_side(&app, next, cx);
                    }
                },
            )],
        ),
    ];
    let language = vec![row(
        &p,
        "settings.language",
        t(keys::GENERAL_LANGUAGE_LABEL),
        Some(t(keys::GENERAL_LANGUAGE_HINT).into()),
        vec![choice(
            "language",
            locales,
            preference.as_stored(),
            168.,
            t(keys::GENERAL_LANGUAGE_LABEL).into(),
            {
                let app = app.clone();
                move |value, _, cx| {
                    let next: LocalePreference = normalise_preference(Some(&value));
                    set_locale(&app, next, cx);
                }
            },
        )],
    )];
    div()
        .flex()
        .flex_col()
        .child(section(
            &p,
            t(keys::GENERAL_APPEARANCE_TITLE),
            true,
            appearance,
        ))
        .child(section(
            &p,
            t(keys::GENERAL_LANGUAGE_TITLE),
            false,
            language,
        ))
        .when(cfg!(debug_assertions), |d| {
            d.child(
                div()
                    .px(px(8.))
                    .pt(px(20.))
                    .text_size(px(12.))
                    .line_height(px(19.5))
                    .text_color(p.muted_foreground)
                    .child(t(screens::SETTINGS_DEVELOPER_POINTER)),
            )
        })
}

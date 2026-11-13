//! What the General tab's choices do beyond saving them: re-theme the window, move the rail, switch
//! the translator.

use gpui::Entity;
use gpui::{App, Window, WindowAppearance};
use quiver_state::i18n::{detect_locale, LocalePreference, Translator};
use quiver_state::settings::preferences::save_locale;
use quiver_state::shell::{ShellSettings, SidebarSide, ThemePreference, THEME_STORAGE_KEY};
use quiver_theme::Mode;

use crate::model::{save_shell, AppModel};

pub fn set_theme(
    app: &Entity<AppModel>,
    preference: ThemePreference,
    window: &mut Window,
    cx: &mut App,
) {
    app.update(cx, |model, cx| {
        model.theme = preference;
        if let Err(err) = model
            .services
            .key_value
            .set(THEME_STORAGE_KEY, preference.as_stored())
        {
            log::warn!("could not save the theme: {err}");
        }
        cx.notify();
    });
    let system_dark = matches!(
        window.appearance(),
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    let mode = if preference.is_dark(system_dark) {
        Mode::Dark
    } else {
        Mode::Light
    };
    quiver_ui::install(cx, mode);
    window.refresh();
}

pub fn set_side(
    app: &Entity<AppModel>,
    side: SidebarSide,
    cx: &mut App,
) {
    app.update(cx, |model, cx| {
        let mut shell: ShellSettings = model.shell;
        shell.set_side(side);
        model.shell = shell;
        save_shell(model.services.key_value.as_ref(), &model.shell);
        cx.notify();
    });
}

pub fn set_locale(
    app: &Entity<AppModel>,
    preference: LocalePreference,
    cx: &mut App,
) {
    app.update(cx, |model, cx| {
        save_locale(model.services.key_value.as_ref(), preference);
        let detected = detect_locale(&[]);
        let forced = std::env::var("QUIVER_LOCALE").ok();
        model.translator = Translator::new(preference.resolve(detected, forced.as_deref()));
        cx.notify();
    });
}

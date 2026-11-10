//! The native menu bar (macOS) and the keyboard shortcuts that work on every platform.
//!
//! The macOS menu is `quiver_platform::menu::macos_menu()` turned into gpui menus; each entry that has a
//! standard role goes to the OS role (so Cut/Copy/Paste reach the focused field) and the rest to the
//! actions below. The shortcuts are bound on all platforms: the old app had none beyond per-field keys,
//! so these are the navigation ones a desktop app is expected to have.

use gpui::{actions, App, KeyBinding, Menu, MenuItem, OsAction};
use quiver_platform::menu::{macos_menu, MenuAction, MenuItemSpec};

use crate::model::{app_model, navigate, Route};

actions!(
    quiver,
    [
        Quit,
        HideApp,
        HideOthers,
        ShowAll,
        Minimize,
        CloseWindow,
        GoBack,
        GoForward,
        OpenSettings,
        ToggleConsole,
    ]
);

/// The key bindings, as (keystroke, action) in the form the platform expects: `cmd` on macOS, `ctrl`
/// elsewhere.
pub fn bindings() -> Vec<KeyBinding> {
    let m = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    vec![
        KeyBinding::new(&format!("{m}-q"), Quit, None),
        KeyBinding::new(&format!("{m}-w"), CloseWindow, None),
        KeyBinding::new(&format!("{m}-["), GoBack, None),
        KeyBinding::new(&format!("{m}-]"), GoForward, None),
        KeyBinding::new("alt-left", GoBack, None),
        KeyBinding::new("alt-right", GoForward, None),
        KeyBinding::new(&format!("{m}-,"), OpenSettings, None),
        KeyBinding::new(&format!("{m}-`"), ToggleConsole, None),
    ]
}

fn item(
    action: MenuAction,
    label: &'static str,
) -> MenuItem {
    match action {
        MenuAction::Quit => MenuItem::action(label, Quit),
        MenuAction::Hide => MenuItem::action(label, HideApp),
        MenuAction::HideOthers => MenuItem::action(label, HideOthers),
        MenuAction::ShowAll => MenuItem::action(label, ShowAll),
        MenuAction::Minimize => MenuItem::action(label, Minimize),
        MenuAction::CloseWindow => MenuItem::action(label, CloseWindow),
        MenuAction::Cut => MenuItem::os_action(label, NoAction, OsAction::Cut),
        MenuAction::Copy => MenuItem::os_action(label, NoAction, OsAction::Copy),
        MenuAction::Paste => MenuItem::os_action(label, NoAction, OsAction::Paste),
        MenuAction::SelectAll => MenuItem::os_action(label, NoAction, OsAction::SelectAll),
        MenuAction::About | MenuAction::Services => MenuItem::action(label, OpenSettings),
    }
}

// The OS performs Cut/Copy/Paste/Select All itself through its role; the action is a stand-in.
actions!(quiver, [NoAction]);

/// The menus of the macOS build.
pub fn menus() -> Vec<Menu> {
    macos_menu()
        .submenus
        .into_iter()
        .map(|sub| Menu {
            name: sub.title.into(),
            items: sub
                .items
                .into_iter()
                .map(|i| match i {
                    MenuItemSpec::Action { action, label, .. } => item(action, label),
                    MenuItemSpec::Separator => MenuItem::separator(),
                })
                .collect(),
        })
        .collect()
}

/// Binds the keys, handles the actions and (macOS only) installs the menu bar.
pub fn install(cx: &mut App) {
    cx.bind_keys(bindings());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &HideApp, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &GoBack, cx| {
        app_model(cx).update(cx, |m, cx| m.back(cx));
    });
    cx.on_action(|_: &GoForward, cx| {
        app_model(cx).update(cx, |m, cx| m.forward(cx));
    });
    cx.on_action(|_: &OpenSettings, cx| {
        navigate(
            cx,
            Route::Settings {
                tab: quiver_state::settings::SettingsTab::General,
            },
        )
    });
    cx.on_action(|_: &ToggleConsole, cx| {
        app_model(cx).update(cx, |m, cx| {
            let open = !m.console_open;
            m.set_console_open(open, cx);
        });
    });
    cx.on_action(|_: &Minimize, cx| {
        if let Some(window) = cx.active_window() {
            window
                .update(cx, |_, window, _| window.minimize_window())
                .ok();
        }
    });
    cx.on_action(|_: &CloseWindow, cx| {
        if let Some(window) = cx.active_window() {
            window
                .update(cx, |_, window, _| window.remove_window())
                .ok();
        }
    });
    if cfg!(target_os = "macos") {
        cx.set_menus(menus());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_keeps_the_three_submenus_and_every_item() {
        let menus = menus();
        let names: Vec<_> = menus.iter().map(|m| m.name.to_string()).collect();
        assert_eq!(names, ["Quiver", "Edit", "Window"]);
        let spec = macos_menu();
        for (menu, sub) in menus.iter().zip(&spec.submenus) {
            assert_eq!(menu.items.len(), sub.items.len());
        }
    }

    #[test]
    fn every_binding_parses() {
        assert!(!bindings().is_empty());
    }
}

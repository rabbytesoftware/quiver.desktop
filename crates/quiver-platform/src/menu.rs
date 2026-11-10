//! The macOS application menu, as data.
//!
//! Without this Quiver would ship the stock menu, which captures Cmd+Z
//! natively: AppKit resolves menu key-equivalents *before* the app sees them,
//! and a native Undo targets whatever the toolkit's own undo stack is, not
//! anything the app knows about. Omitting Edit > Undo/Redo lets those
//! keystrokes reach the app, which can own them.
//!
//! Window > Close is deliberately KEPT, which is where this diverges from
//! Crowbar's menu. Crowbar frees Cmd+W because it has in-app tabs to close;
//! Quiver has no such binding yet, so omitting it would only make Cmd+W do
//! nothing at all. Revisit when there is something in-app for it to close.
//!
//! The app crate turns a [`MenuSpec`] into native menus. Actions with a
//! standard macOS role (everything here) should be wired to that role.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    About,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Minimize,
    CloseWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemSpec {
    Action {
        action: MenuAction,
        label: &'static str,
        /// Key equivalent in `cmd-alt-h` form, `None` when the role has none.
        shortcut: Option<&'static str>,
    },
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmenuSpec {
    pub title: &'static str,
    pub items: Vec<MenuItemSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSpec {
    pub submenus: Vec<SubmenuSpec>,
}

impl MenuSpec {
    /// Every action in the menu, in order.
    pub fn actions(&self) -> impl Iterator<Item = MenuAction> + '_ {
        self.submenus
            .iter()
            .flat_map(|s| s.items.iter())
            .filter_map(|item| match item {
                MenuItemSpec::Action { action, .. } => Some(*action),
                MenuItemSpec::Separator => None,
            })
    }
}

fn item(
    action: MenuAction,
    label: &'static str,
    shortcut: Option<&'static str>,
) -> MenuItemSpec {
    MenuItemSpec::Action {
        action,
        label,
        shortcut,
    }
}

/// The menu bar of the macOS build: Quiver, Edit, Window.
pub fn macos_menu() -> MenuSpec {
    use MenuAction::*;
    use MenuItemSpec::Separator;
    MenuSpec {
        submenus: vec![
            SubmenuSpec {
                title: "Quiver",
                items: vec![
                    item(About, "About Quiver", None),
                    Separator,
                    item(Services, "Services", None),
                    Separator,
                    item(Hide, "Hide Quiver", Some("cmd-h")),
                    item(HideOthers, "Hide Others", Some("cmd-alt-h")),
                    item(ShowAll, "Show All", None),
                    Separator,
                    item(Quit, "Quit Quiver", Some("cmd-q")),
                ],
            },
            // No Undo/Redo: freed for the app (see module docs).
            SubmenuSpec {
                title: "Edit",
                items: vec![
                    item(Cut, "Cut", Some("cmd-x")),
                    item(Copy, "Copy", Some("cmd-c")),
                    item(Paste, "Paste", Some("cmd-v")),
                    item(SelectAll, "Select All", Some("cmd-a")),
                ],
            },
            SubmenuSpec {
                title: "Window",
                items: vec![
                    item(Minimize, "Minimize", Some("cmd-m")),
                    item(CloseWindow, "Close Window", Some("cmd-w")),
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_has_the_three_submenus_in_order() {
        let titles: Vec<_> = macos_menu().submenus.iter().map(|s| s.title).collect();
        assert_eq!(titles, ["Quiver", "Edit", "Window"]);
    }

    /// The reason the menu exists: Cmd+Z must reach the app.
    #[test]
    fn no_undo_or_redo_captures_cmd_z() {
        for item in macos_menu().submenus.iter().flat_map(|s| &s.items) {
            if let MenuItemSpec::Action {
                label, shortcut, ..
            } = item
            {
                assert!(!label.contains("Undo") && !label.contains("Redo"));
                assert_ne!(*shortcut, Some("cmd-z"));
                assert_ne!(*shortcut, Some("cmd-shift-z"));
            }
        }
    }

    #[test]
    fn close_window_is_kept_on_cmd_w() {
        assert!(macos_menu().actions().any(|a| a == MenuAction::CloseWindow));
        let window = &macos_menu().submenus[2];
        assert!(window.items.contains(&item(
            MenuAction::CloseWindow,
            "Close Window",
            Some("cmd-w")
        )));
    }

    #[test]
    fn no_two_items_share_a_shortcut() {
        let mut seen = std::collections::HashSet::new();
        for item in macos_menu().submenus.iter().flat_map(|s| &s.items) {
            if let MenuItemSpec::Action {
                shortcut: Some(s), ..
            } = item
            {
                assert!(seen.insert(*s), "{s} is bound twice");
            }
        }
    }
}

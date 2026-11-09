use super::outcome::Toast;

pub const TOAST_DURATION_MS: u64 = 2600;

/// Transient messages. Expiry is the view's timer calling `dismiss` after [`TOAST_DURATION_MS`]; ids are a
/// counter, so tests need no clock or randomness.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ToastStack {
    toasts: Vec<Toast>,
    next_id: u64,
}

impl ToastStack {
    pub fn push(&mut self, message: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.toasts.push(Toast { id, message });
        id
    }

    pub fn dismiss(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn toasts(&self) -> &[Toast] {
        &self.toasts
    }
}

/// Which overlay the Remote feature has open. Opening the command palette (the global, always-reachable
/// switcher) must not leave a dialog or a row menu stacked underneath it.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RemoteOverlays {
    pub add_open: bool,
    pub rename_id: Option<String>,
    pub remove_id: Option<String>,
    pub open_menu_id: Option<String>,
    pub cmd_open: bool,
    pub cmd_query: String,
    pub cmd_index: usize,
}

impl RemoteOverlays {
    pub fn toggle_menu(&mut self, id: &str) {
        self.open_menu_id = if self.open_menu_id.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_owned())
        };
    }

    pub fn open_cmd(&mut self) {
        if self.cmd_open {
            return;
        }
        self.add_open = false;
        self.rename_id = None;
        self.remove_id = None;
        self.open_menu_id = None;
        self.cmd_open = true;
        self.cmd_query.clear();
        self.cmd_index = 0;
    }

    pub fn close_cmd(&mut self) {
        self.cmd_open = false;
    }

    /// A new query resets the highlighted row.
    pub fn set_cmd_query(&mut self, query: &str) {
        self.cmd_query = query.to_owned();
        self.cmd_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_the_palette_closes_every_other_overlay_and_starts_fresh() {
        let mut ui = RemoteOverlays {
            add_open: true,
            rename_id: Some("a".into()),
            remove_id: Some("b".into()),
            open_menu_id: Some("c".into()),
            cmd_query: "old".into(),
            cmd_index: 3,
            ..RemoteOverlays::default()
        };
        ui.open_cmd();
        assert!(ui.cmd_open);
        assert!(
            !ui.add_open
                && ui.rename_id.is_none()
                && ui.remove_id.is_none()
                && ui.open_menu_id.is_none()
        );
        assert_eq!((ui.cmd_query.as_str(), ui.cmd_index), ("", 0));
    }

    #[test]
    fn opening_an_open_palette_keeps_what_was_typed() {
        let mut ui = RemoteOverlays::default();
        ui.open_cmd();
        ui.set_cmd_query("abc");
        ui.cmd_index = 2;
        ui.open_cmd();
        assert_eq!(ui.cmd_query, "abc");
        assert_eq!(ui.cmd_index, 2);
    }

    #[test]
    fn a_new_query_resets_the_highlight_and_the_menu_toggles() {
        let mut ui = RemoteOverlays {
            cmd_index: 4,
            ..RemoteOverlays::default()
        };
        ui.set_cmd_query("x");
        assert_eq!(ui.cmd_index, 0);
        ui.toggle_menu("a");
        assert_eq!(ui.open_menu_id.as_deref(), Some("a"));
        ui.toggle_menu("b");
        assert_eq!(ui.open_menu_id.as_deref(), Some("b"));
        ui.toggle_menu("b");
        assert_eq!(ui.open_menu_id, None);
    }

    #[test]
    fn toasts_stack_and_dismiss_by_id() {
        let mut stack = ToastStack::default();
        let a = stack.push("one".into());
        let b = stack.push("two".into());
        assert_ne!(a, b);
        stack.dismiss(a);
        assert_eq!(stack.toasts().len(), 1);
        assert_eq!(stack.toasts()[0].message, "two");
        stack.dismiss(a);
        assert_eq!(stack.toasts().len(), 1);
    }
}

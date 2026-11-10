use gpui::{AnyView, SharedString};

/// The modal layer: views that float over the whole window (Dialog, Sheet). A modal is a view that
/// renders a `quiver_ui::Dialog`/`Sheet` with `.open(true)` and calls `close_modal(id)` from its
/// `on_dismiss`. The shell draws them in the order they were opened; the last one is on top.
#[derive(Default)]
pub struct ModalStack {
    entries: Vec<(SharedString, AnyView)>,
}

impl ModalStack {
    /// Opens a modal. Opening an id that is already open replaces its view and moves it to the top.
    pub fn open(
        &mut self,
        id: impl Into<SharedString>,
        view: AnyView,
    ) {
        let id = id.into();
        self.entries.retain(|(open, _)| *open != id);
        self.entries.push((id, view));
    }

    pub fn close(
        &mut self,
        id: &str,
    ) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(open, _)| open.as_ref() != id);
        self.entries.len() != before
    }

    pub fn is_open(
        &self,
        id: &str,
    ) -> bool {
        self.entries.iter().any(|(open, _)| open.as_ref() == id)
    }

    pub fn views(&self) -> impl Iterator<Item = &AnyView> {
        self.entries.iter().map(|(_, view)| view)
    }

    pub fn ids(&self) -> impl Iterator<Item = &SharedString> {
        self.entries.iter().map(|(id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;

    struct Empty;

    impl gpui::Render for Empty {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn opening_twice_replaces_and_moves_to_the_top(cx: &mut TestAppContext) {
        let (a, b) = (cx.new(|_| Empty), cx.new(|_| Empty));
        let mut stack = ModalStack::default();
        stack.open("settings", a.clone().into());
        stack.open("methods", b.into());
        stack.open("settings", a.into());
        let order: Vec<_> = stack.ids().map(|id| id.to_string()).collect();
        assert_eq!(order, ["methods", "settings"]);
        assert!(stack.close("methods"));
        assert!(!stack.close("methods"));
        assert!(stack.is_open("settings"));
    }
}

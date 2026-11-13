//! The gallery's mutable demo state.

use gpui::{AppContext, Context, Entity, SharedString, Window};
use quiver_theme::Mode;

use crate::InputState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Page {
    Actions,
    Forms,
    Navigation,
    Overlays,
    Surfaces,
    Tiles,
}

impl Page {
    pub const ALL: [(Page, &'static str); 6] = [
        (Page::Actions, "actions"),
        (Page::Forms, "forms"),
        (Page::Navigation, "navigation"),
        (Page::Overlays, "overlays"),
        (Page::Surfaces, "surfaces"),
        (Page::Tiles, "tiles"),
    ];

    pub fn from_env() -> Page {
        let want = std::env::var("QUIVER_GALLERY_PAGE").unwrap_or_default();
        Self::ALL
            .iter()
            .find(|(_, n)| *n == want)
            .map(|(p, _)| *p)
            .unwrap_or(Page::Actions)
    }
}

pub(super) struct Gallery {
    pub mode: Mode,
    pub page: Page,
    pub text: Entity<InputState>,
    pub switch_on: bool,
    pub slider: f32,
    pub latency: Option<f64>,
    pub rate: Option<f64>,
    pub select: Option<SharedString>,
    pub tab: usize,
    pub underline_tab: usize,
    pub dialog_open: bool,
    pub sheet_open: Option<crate::SheetSide>,
    pub steps_open: bool,
    pub search: Entity<InputState>,
    pub last_action: SharedString,
}

impl Gallery {
    pub fn new(
        mode: Mode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let open = std::env::var("QUIVER_GALLERY_OPEN").unwrap_or_default();
        Self {
            mode,
            page: Page::from_env(),
            text: cx.new(|cx| InputState::new(window, cx).placeholder("Search Arrows")),
            switch_on: true,
            slider: 35.0,
            latency: Some(250.0),
            rate: Some(5.0),
            select: Some("stable".into()),
            tab: 0,
            underline_tab: 1,
            dialog_open: open == "dialog",
            sheet_open: (open == "sheet").then_some(crate::SheetSide::Right),
            steps_open: false,
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search Arrows")),
            last_action: "(nothing yet)".into(),
        }
    }
}

/// An `on_dismiss` callback that runs `f` on the gallery and re-renders it.
pub(super) fn dismiss(
    cx: &Context<Gallery>,
    f: fn(&mut Gallery),
) -> impl Fn(&mut Window, &mut gpui::App) + 'static {
    let view = cx.entity();
    move |_, cx| {
        view.update(cx, |this, cx| {
            f(this);
            cx.notify();
        })
    }
}

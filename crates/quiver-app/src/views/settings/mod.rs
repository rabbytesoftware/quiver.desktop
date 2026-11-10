//! Settings (`SettingsAfter` / `SettingsEngineAfter`): General, Engine and, in debug builds, Developer.
//! Every decision is `quiver_state::settings`'; this view holds the loaded engine state, the update
//! cards and the scroll position.

mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, Window};
use quiver_state::arrow_details::invoke::ActionFailure;
use quiver_state::domain::ArrowChannel;
use quiver_state::settings::{EngineState, PathStore, SettingsTab};
use quiver_ui::{ActiveQuiver, ScrollArea};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::screen_probe;

pub use parts::UpdateCard;

pub struct SettingsView {
    pub(super) app: Entity<AppModel>,
    pub(super) tab: SettingsTab,
    pub(super) engine: EngineState,
    pub(super) path: PathStore,
    /// quiver.core's release channels; `None` until the fetch answered, `Err` when it failed.
    pub(super) channels: Option<Result<Vec<ArrowChannel>, ActionFailure>>,
    pub(super) core: UpdateCard,
    pub(super) desktop: UpdateCard,
    loaded: bool,
}

impl SettingsView {
    pub fn new(
        app: Entity<AppModel>,
        tab: SettingsTab,
        cx: &mut Context<Self>,
    ) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        let mut view = Self {
            app,
            tab,
            engine: EngineState::default(),
            path: PathStore::default(),
            channels: None,
            core: UpdateCard::default(),
            desktop: UpdateCard::default(),
            loaded: false,
        };
        view.ensure_loaded(cx);
        view
    }

    pub(super) fn set_tab(
        &mut self,
        tab: SettingsTab,
        cx: &mut Context<Self>,
    ) {
        self.tab = tab;
        self.ensure_loaded(cx);
        cx.notify();
    }

    /// The engine tab and the developer tab read the same config; it is fetched the first time either
    /// is opened, and again by "Try again".
    fn ensure_loaded(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.loaded || self.tab == SettingsTab::General {
            return;
        }
        self.loaded = true;
        if self.app.read(cx).demo {
            return;
        }
        self.load_engine(cx);
        self.load_channels(cx);
        self.load_path(cx);
    }
}

impl Render for SettingsView {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let body = match self.tab {
            SettingsTab::General => parts::general(self, cx).into_any_element(),
            SettingsTab::Engine => parts::engine(self, cx).into_any_element(),
            SettingsTab::Developer => parts::developer(self, cx).into_any_element(),
        };
        div()
            .id("settings")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .child(parts::tabs_bar(self, cx))
            .child(
                div().relative().flex_1().min_h_0().child(
                    ScrollArea::new("settings.scroll")
                        .fade_color(p.background)
                        .child(
                            div()
                                .pl(px(24.))
                                .pr(px(40.))
                                .pt(px(27.))
                                .pb(px(32.))
                                .child(body),
                        ),
                ),
            )
            .child(screen_probe("settings"))
    }
}

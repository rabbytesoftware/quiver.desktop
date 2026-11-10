//! Home (`HomeAfter` / `HomeLowerAfter`): Recents, Library, then Popular with category chips and
//! "Load more". The decisions come from `quiver_state::home::build_home` and
//! `quiver_state::recommended::build_recommended`.

mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, Window};
use quiver_state::domain::ConnectionStatus;
use quiver_state::home::{build_home, HomeInput};
use quiver_state::store::CatalogStatus;
use quiver_ui::{ActiveQuiver, ScrollArea};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::screen_probe;

pub struct HomeView {
    pub(super) app: Entity<AppModel>,
    pub(super) category: Option<String>,
    /// Presses of "Load more".
    pub(super) extra: usize,
}

impl HomeView {
    pub fn new(app: Entity<AppModel>, cx: &mut Context<Self>) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        let home = app.read(cx).home.store().clone();
        observe_store(cx, &home);
        observe_store(cx, services.status.store());
        Self {
            app,
            category: None,
            extra: 0,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.read(cx);
        let (arrows, catalog) = app
            .services
            .arrows
            .store()
            .read(|s| (s.arrows.values().cloned().collect::<Vec<_>>(), s.catalog));
        let home = app.home.snapshot();
        let connected = app.services.status.status() == ConnectionStatus::Ready;
        let model = build_home(&HomeInput {
            arrows: &arrows,
            collections: &[],
            home: home.data.as_ref(),
            catalog_loading: catalog == CatalogStatus::Loading,
            collections_loading: false,
            home_loading: home.loading && home.data.is_none(),
            show_self_components: app.show_self_components,
        });
        let failed = home.error.is_some() && home.data.is_none() && arrows.is_empty();
        let body = if failed || (!connected && arrows.is_empty() && catalog == CatalogStatus::Error)
        {
            parts::error_state(self, cx).into_any_element()
        } else if model.empty {
            parts::empty_state(self, cx).into_any_element()
        } else {
            parts::sections(self, &model, home.data.as_ref(), cx).into_any_element()
        };
        div()
            .id("home")
            .relative()
            .size_full()
            .bg(p.background)
            .child(
                ScrollArea::new("home.scroll")
                    .fade_color(p.background)
                    .child(
                        div()
                            .px(px(40.))
                            .pl(px(24.))
                            .pt(px(24.))
                            .pb(px(32.))
                            .child(body),
                    ),
            )
            .child(screen_probe("home"))
    }
}

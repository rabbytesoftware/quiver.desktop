//! Home (`HomeAfter` / `HomeLowerAfter`): Recents, Library, then Popular with category chips and
//! every tile its shelf holds, then a preview of the collections you follow. The decisions come from `quiver_state::home::build_home` and
//! `quiver_state::recommended::build_recommended`.

mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, Window};
use quiver_state::domain::{CollectionListItem, ConnectionStatus};
use quiver_state::home::{build_home, HomeInput};
use quiver_state::queries::collection::fetch_followed_collections;
use quiver_state::queries::keys::followed_collections_key;
use quiver_state::queries::Query;
use quiver_state::store::CatalogStatus;
use quiver_ui::{ActiveQuiver, ScrollArea};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::grid_page::{start_feed, Feed};
use crate::views::screen_probe;

pub struct HomeView {
    pub(super) app: Entity<AppModel>,
    pub(super) category: Option<String>,
    /// The collections you follow: Home previews four of them and links to the Collections page.
    followed: Query<Vec<CollectionListItem>>,
    _followed_feed: Option<Feed>,
}

impl HomeView {
    pub fn new(
        app: Entity<AppModel>,
        cx: &mut Context<Self>,
    ) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        let home = app.read(cx).home.store().clone();
        observe_store(cx, &home);
        observe_store(cx, services.status.store());
        let followed = Query::default();
        observe_store(cx, followed.store());
        // The demo has no daemon: it is seeded, never fetched.
        let feed = if app.read(cx).demo {
            crate::demo::collections::seed_followed(&followed);
            None
        } else {
            Some(start_feed(
                &services,
                followed.clone(),
                followed_collections_key(),
                |api| async move { fetch_followed_collections(api.as_ref()).await },
            ))
        };
        Self {
            app,
            category: None,
            followed,
            _followed_feed: feed,
        }
    }
}

impl Render for HomeView {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        // The sections' width outright (the main area inside the 24px + 40px padding): GPUI leaves a column's
        // children at their content width here, so a shelf with one tile shrank and its header's link sat
        // against the title.
        let sidebar = self.app.read(cx).shell.sidebar_width as f32;
        let content = (f32::from(window.viewport_size().width) - sidebar - 64.).max(0.);
        let app = self.app.read(cx);
        let (arrows, catalog) = app
            .services
            .arrows
            .store()
            .read(|s| (s.arrows.values().cloned().collect::<Vec<_>>(), s.catalog));
        let home = app.home.snapshot();
        let followed = self.followed.snapshot();
        let connected = app.services.status.status() == ConnectionStatus::Ready;
        let model = build_home(&HomeInput {
            arrows: &arrows,
            collections: followed.data.as_deref().unwrap_or_default(),
            home: home.data.as_ref(),
            catalog_loading: catalog == CatalogStatus::Loading,
            // Until the first answer or error, so Home does not flash its empty state before the list lands.
            collections_loading: followed.data.is_none() && followed.error.is_none(),
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
            parts::sections(self, &model, home.data.as_ref(), content, cx).into_any_element()
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
                            .w_full()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;
    use quiver_state::home::HomeSection;

    /// Home lost its Collections section once: the view fed `build_home` an empty list. The followed
    /// collections must reach the model, in name order, so the section and its "View all" link exist.
    #[gpui::test]
    fn home_previews_the_followed_collections(cx: &mut gpui::TestAppContext) {
        let app = demo_app(cx);
        let view = cx.new(|cx| HomeView::new(app, cx));
        let (sections, names, total) = view.read_with(cx, |v, _| {
            let followed = v.followed.snapshot().data.unwrap_or_default();
            let model = build_home(&HomeInput {
                arrows: &[],
                collections: &followed,
                home: None,
                catalog_loading: false,
                collections_loading: false,
                home_loading: false,
                show_self_components: false,
            });
            let names: Vec<String> = model
                .collections_preview
                .iter()
                .map(|c| c.name.clone())
                .collect();
            (model.sections, names, model.collections_total)
        });
        assert!(sections.contains(&HomeSection::Collections));
        assert_eq!(names, ["Game Servers", "Monitoring Stack"]);
        assert_eq!(total, 2);
    }
}

//! Collections (`CollectionsAfter`): the collections you follow, one tile each. The badge on the art
//! says how many arrows are inside and whether you hold them all. Which collections you follow can
//! only change from a collection's own page (Follow / Following); there is nothing to create or
//! rename here, because quiver.core owns collections (they are git repositories).

use gpui::prelude::*;
use gpui::{div, Context, Entity, Render, Window};
use quiver_state::collections::{collection_badge_label, sorted_collections};
use quiver_state::domain::CollectionListItem;
use quiver_state::i18n::en::{collections as keys, home};
use quiver_state::queries::collection::fetch_followed_collections;
use quiver_state::queries::keys::followed_collections_key;
use quiver_state::queries::Query;
use quiver_state::tile::TileInput;
use quiver_ui::{ActiveQuiver, TileSize};

use crate::bridge::observe_store;
use crate::model::{AppModel, Route};
use crate::views::grid_page::{
    error_state, notice, page, page_header, skeleton, start_feed, tile_grid, Feed,
};
use crate::views::screen_probe;
use crate::views::tiles::tile;

pub struct CollectionsView {
    app: Entity<AppModel>,
    followed: Query<Vec<CollectionListItem>>,
    _feed: Option<Feed>,
}

impl CollectionsView {
    pub fn new(
        app: Entity<AppModel>,
        cx: &mut Context<Self>,
    ) -> Self {
        let followed = Query::default();
        observe_store(cx, followed.store());
        let (services, demo) = {
            let a = app.read(cx);
            (a.services.clone(), a.demo)
        };
        // The demo has no daemon: it is seeded, never fetched.
        let feed = if demo {
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
            followed,
            _feed: feed,
        }
    }
}

impl Render for CollectionsView {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.clone();
        let state = self.followed.snapshot();
        let items = sorted_collections(state.data.as_deref().unwrap_or_default());
        let (title, subtitle) = {
            let a = app.read(cx);
            (
                a.t(home::COLLECTIONS),
                a.translator.t_with(
                    keys::SUBTITLE,
                    &quiver_state::i18n::Params::count(items.len()),
                ),
            )
        };
        let header = page_header(&app, "collections", title, Some(subtitle), vec![], cx);
        let body = if state.data.is_none() && state.error.is_some() {
            error_state("collections", &app, followed_collections_key(), cx).into_any_element()
        } else if state.data.is_none() {
            skeleton("collections", 4, TileSize::Compact.width(), cx).into_any_element()
        } else if items.is_empty() {
            let a = app.read(cx);
            let (title, body) = (a.t(home::EMPTY_TITLE), a.t(home::EMPTY_DESCRIPTION));
            notice("collections.empty", title, body, None, cx).into_any_element()
        } else {
            let tiles = items.iter().map(|item| {
                // The list payload names no members, so how many are already in the library is
                // unknown here and the badge says the count only.
                let label =
                    collection_badge_label(&app.read(cx).translator, item.arrow_count, None);
                let input = TileInput::from_collection_item(item, label);
                let go = app.clone();
                let id = item.namespace.clone();
                tile("collections", &input, TileSize::Compact, &app, cx)
                    .on_click(move |_, _, cx| {
                        let route = Route::Collection { id: id.clone() };
                        go.update(cx, |m, cx| m.navigate(route, cx));
                    })
                    .into_any_element()
            });
            tile_grid(tiles).into_any_element()
        };
        div()
            .id("collections")
            .relative()
            .size_full()
            .bg(p.background)
            .child(page(
                "collections.scroll",
                p.background,
                div().flex().flex_col().child(header).child(body),
            ))
            .child(screen_probe("collections"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;

    #[gpui::test]
    fn the_demo_followed_list_is_seeded_and_sorted_by_name(cx: &mut gpui::TestAppContext) {
        let app = demo_app(cx);
        let view = cx.new(|cx| CollectionsView::new(app, cx));
        let names: Vec<String> = view.read_with(cx, |v, _| {
            let data = v.followed.snapshot().data.unwrap_or_default();
            sorted_collections(&data)
                .into_iter()
                .map(|c| c.name)
                .collect()
        });
        assert_eq!(names, ["Dev Toolbox", "Game Servers"]);
    }
}

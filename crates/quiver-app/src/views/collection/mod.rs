//! One collection (`CollectionDetailAfter`): identity and Follow, then the member grid with any
//! unresolved member as a cell of its own. Decisions: `quiver_state::collection`.

mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, FontWeight, Render, Window};
use quiver_state::domain::CollectionDetail;
use quiver_state::queries::collection::fetch_collection_detail;
use quiver_state::queries::keys::collection_key;
use quiver_state::queries::Query;
use quiver_ui::{ActiveQuiver, TileSize};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::grid_page::{back_link, error_state, page, skeleton, start_feed, Feed};
use crate::views::screen_probe;

pub struct CollectionView {
    pub(super) app: Entity<AppModel>,
    /// The collection's namespace (the route id).
    pub(super) id: String,
    pub(super) detail: Query<CollectionDetail>,
    /// A follow / unfollow request is in flight.
    pub(super) follow_pending: bool,
    pub(super) demo: bool,
    _feed: Option<Feed>,
}

impl CollectionView {
    pub fn new(
        app: Entity<AppModel>,
        id: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let detail = Query::default();
        observe_store(cx, detail.store());
        let (services, demo) = {
            let a = app.read(cx);
            (a.services.clone(), a.demo)
        };
        observe_store(cx, services.arrows.store());
        // The demo has no daemon: it is seeded, never fetched.
        let feed = if demo {
            crate::demo::collections::seed_detail(&detail, &id);
            None
        } else {
            let ns = id.clone();
            Some(start_feed(
                &services,
                detail.clone(),
                collection_key(&id),
                move |api| {
                    let ns = ns.clone();
                    async move { fetch_collection_detail(api.as_ref(), &ns).await }
                },
            ))
        };
        Self {
            app,
            id,
            detail,
            follow_pending: false,
            demo,
            _feed: feed,
        }
    }

    /// Demo only: the mutation has no daemon to answer, so flip the seeded flag here.
    pub(super) fn flip_followed(&self) {
        self.detail.store().update(|s| {
            if let Some(d) = &mut s.data {
                d.item.followed = !d.item.followed;
            }
        });
    }
}

impl Render for CollectionView {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.clone();
        let state = self.detail.snapshot();
        let header = back_link(&app, "collection", cx);
        let body = match &state.data {
            None if state.error.is_some() => {
                error_state("collection", &app, collection_key(&self.id), cx).into_any_element()
            }
            None => skeleton("collection", 4, TileSize::Compact.width(), cx).into_any_element(),
            Some(detail) => div()
                .flex()
                .flex_col()
                .gap(px(28.))
                .child(parts::hero(self, detail, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(16.))
                        .child(
                            div()
                                .text_size(px(18.))
                                .line_height(px(22.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Arrows"),
                        )
                        .child(div().h(px(1.)).bg(p.border))
                        .child(parts::members(self, detail, cx)),
                )
                .into_any_element(),
        };
        div()
            .id("collection")
            .relative()
            .size_full()
            .bg(p.background)
            .child(page(
                "collection.scroll",
                p.background,
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(header)
                    .child(body),
            ))
            .child(screen_probe("collection"))
    }
}

#[cfg(test)]
mod tests {
    use quiver_state::collection::{grid_items, GridItem};

    use super::*;
    use crate::views::grid_page::testing::demo_app;

    #[gpui::test]
    fn the_demo_collection_has_an_unresolved_member_in_the_grid_and_follow_flips(
        cx: &mut gpui::TestAppContext
    ) {
        let app = demo_app(cx);
        let view =
            cx.new(|cx| CollectionView::new(app, "github.com/rabbyte/game-servers".into(), cx));
        view.read_with(cx, |v, _| {
            let detail = v.detail.snapshot().data.expect("seeded");
            let items = grid_items(&detail.arrows);
            assert_eq!(items.len(), 4);
            assert!(matches!(items[3], GridItem::Unresolved { .. }));
            assert!(detail.item.followed);
            v.flip_followed();
            assert!(!v.detail.snapshot().data.expect("seeded").item.followed);
        });
    }
}

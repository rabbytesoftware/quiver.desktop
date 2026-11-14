//! One collection (`CollectionDetailAfter`): identity and Follow, then the member grid with any
//! unresolved member as a cell of its own. Decisions: `quiver_state::collection`.

mod parts;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, FontWeight, KeyDownEvent, MouseDownEvent, Render, Window};
use quiver_state::domain::CollectionDetail;
use quiver_state::queries::collection::fetch_collection_detail;
use quiver_state::queries::keys::collection_key;
use quiver_state::queries::Query;
use quiver_ui::{ActiveQuiver, TileSize, TrackedText};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::card_dismiss::CardDismiss;
use crate::views::grid_page::{error_state, page, skeleton, start_feed, Feed};
use crate::views::hero_fit::Fit;
use crate::views::screen_probe;

pub struct CollectionView {
    pub(super) app: Entity<AppModel>,
    /// The collection's namespace (the route id).
    pub(super) id: String,
    pub(super) detail: Query<CollectionDetail>,
    /// A follow / unfollow request is in flight.
    pub(super) follow_pending: bool,
    /// The ⓘ popover of the Follow button.
    pub(super) info_open: bool,
    /// Outside press and Escape close the ⓘ card; see [`CardDismiss`].
    pub(super) card: CardDismiss<()>,
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
            info_open: false,
            card: CardDismiss::new(cx.focus_handle()),
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

impl CollectionView {
    pub(super) fn set_info(
        &mut self,
        open: bool,
        cx: &mut Context<Self>,
    ) {
        self.info_open = open;
        cx.notify();
    }

    /// The ⓘ button was clicked: toggle the card, but never reopen what the press that began the click just
    /// closed from outside.
    pub(super) fn toggle_info(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let open = self.info_open.then_some(());
        let next = self.card.next((), open);
        self.set_info(next.is_some(), cx);
    }
}

impl Render for CollectionView {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.clone();
        let state = self.detail.snapshot();
        // The page's padding is 24 px at the left and 40 at the right (`grid_page::page`).
        let sidebar = app.read(cx).shell.sidebar_width as f32;
        let fit = Fit::at(f32::from(window.viewport_size().width), sidebar, 64.);
        let body = match &state.data {
            None if state.error.is_some() => {
                error_state("collection", &app, collection_key(&self.id), cx).into_any_element()
            }
            None => skeleton("collection", 4, TileSize::Compact.width(), cx).into_any_element(),
            Some(detail) => div()
                .flex()
                .flex_col()
                .gap(px(28.))
                .child(parts::hero(self, detail, fit, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(16.))
                        .child(
                            TrackedText::new("Arrows", 18., 22., -0.3, p.foreground)
                                .weight(FontWeight::SEMIBOLD),
                        )
                        .child(div().h(px(1.)).bg(p.border))
                        .child(parts::members(self, detail, window, cx)),
                )
                .into_any_element(),
        };
        // A card up takes the keyboard so Escape reaches this root.
        if self.info_open && !self.card.focus.contains_focused(window, cx) {
            let focus = self.card.focus.clone();
            window.defer(cx, move |window, _| window.focus(&focus));
        }
        div()
            .id("collection")
            .relative()
            .track_focus(&self.card.focus)
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, _| {
                this.card.press(this.info_open.then_some(()))
            }))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && this.info_open {
                    this.set_info(false, cx);
                }
            }))
            .size_full()
            .bg(p.background)
            .child(page(
                "collection.scroll",
                p.background,
                div().flex().flex_col().gap(px(8.)).child(body),
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

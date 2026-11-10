//! Search results (`SearchAfter`): "N results", the source line and sort at the top, then the
//! "In your vault" and "From the network" shelves of the one tile. The query comes from the route (the
//! sidebar field commits it); the pass itself runs in `quiver_state::store::search_pass`.

mod parts;
pub mod session;

use std::sync::Arc;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Render, Window};
use quiver_state::search::{
    build_results, tile_width, ResultsInput, Selection, SortKey, DEFAULT_SORT,
};
use quiver_ui::ActiveQuiver;

use self::session::SearchSession;
use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::grid_page::{page, skeleton};
use crate::views::screen_probe;

pub struct SearchView {
    pub(super) app: Entity<AppModel>,
    pub(super) query: String,
    pub(super) session: Arc<SearchSession>,
    pub(super) sort: SortKey,
    pub(super) selection: Selection,
}

impl SearchView {
    pub fn new(app: Entity<AppModel>, query: String, cx: &mut Context<Self>) -> Self {
        let session = session::session(cx);
        let demo = app.read(cx).demo;
        observe_store(cx, session.store.store());
        if demo {
            // No daemon: the answer is seeded, the controller never runs.
            crate::demo::search::seed(&session.store, &query);
        } else {
            session.show(&query);
            session.take_submit(&query);
            let mut changes = session.store.store().subscribe();
            let watched = session.clone();
            let shown = query.clone();
            cx.spawn(async move |_, _| {
                while changes.changed().await.is_ok() {
                    watched.take_submit(&shown);
                }
            })
            .detach();
        }
        Self {
            app,
            query,
            session,
            sort: DEFAULT_SORT,
            selection: Selection::default(),
        }
    }
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let state = self.session.store.snapshot();
        let show_self = self.app.read(cx).show_self_components;
        // The store may still hold the previous query's answer for a frame after the route changed.
        let current = state.query == self.query;
        let model = build_results(&ResultsInput {
            query: &self.query,
            phase: state.phase,
            local: if current { &state.local } else { &[] },
            streamed: if current { &state.streamed } else { &[] },
            summary: state.summary.as_ref(),
            local_error: state.local_error,
            pass_failed: state.pass_failed,
            show_self_components: show_self,
            selection: &self.selection,
            sort: self.sort,
        });
        let sidebar = self.app.read(cx).shell.sidebar_width as f32;
        let available = f32::from(window.viewport_size().width) - sidebar - 24. - 40.;
        let width = tile_width(model.total, available);
        let body = if let Some(empty) = &model.empty {
            parts::empty(self, empty, model.narrowed_to_nothing, cx).into_any_element()
        } else if model.narrowed_to_nothing {
            parts::empty(
                self,
                &quiver_state::search::EmptyState::EmptyEverywhere,
                true,
                cx,
            )
            .into_any_element()
        } else if model.total == 0 {
            skeleton("search", 4, width, cx).into_any_element()
        } else {
            parts::shelves(self, &model, width, cx).into_any_element()
        };
        let facets = parts::facet_row(self, &model.answer, cx);
        div()
            .id("search")
            .relative()
            .size_full()
            .bg(p.background)
            .child(page(
                "search.scroll",
                p.background,
                div()
                    .flex()
                    .flex_col()
                    .pt(px(8.))
                    .when(model.show_header, |d| {
                        d.child(parts::header(self, &model, cx))
                    })
                    .children(facets)
                    .child(body),
            ))
            .child(screen_probe("search"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;

    #[gpui::test]
    fn the_demo_answer_for_rust_has_one_vault_hit_one_network_hit_and_a_refusal(
        cx: &mut gpui::TestAppContext,
    ) {
        let app = demo_app(cx);
        let view = cx.new(|cx| SearchView::new(app, "rust".into(), cx));
        let model = view.read_with(cx, |v, _| {
            let state = v.session.store.snapshot();
            build_results(&ResultsInput {
                query: "rust",
                phase: state.phase,
                local: &state.local,
                streamed: &state.streamed,
                summary: state.summary.as_ref(),
                local_error: false,
                pass_failed: false,
                show_self_components: false,
                selection: &Selection::default(),
                sort: DEFAULT_SORT,
            })
        });
        assert_eq!((model.vault.len(), model.network.len()), (1, 1));
        let line = model.source_line.expect("summary");
        assert_eq!((line.sources, line.unavailable), (2, 1));
    }
}

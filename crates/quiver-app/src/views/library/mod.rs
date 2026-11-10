//! Library (`LibraryAfter`): every installed arrow, with All / Running / Updates chips that carry
//! their counts. The decisions come from `quiver_state::library::build_library`.

use gpui::prelude::*;
use gpui::{div, Context, Entity, Render, Window};
use quiver_state::domain::ArrowState;
use quiver_state::i18n::en::{app, home, library as keys, search, sidebar};
use quiver_state::library::{build_library, LibraryFilter};
use quiver_state::store::CatalogStatus;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonVariant, TileSize};

use crate::bridge::observe_store;
use crate::model::{AppModel, Route};
use crate::views::grid_page::{chip, notice, page, page_header, skeleton, tile_grid};
use crate::views::screen_probe;
use crate::views::tiles::{library_input, tile};

pub struct LibraryView {
    app: Entity<AppModel>,
    filter: LibraryFilter,
}

impl LibraryView {
    pub fn new(app: Entity<AppModel>, cx: &mut Context<Self>) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        Self {
            app,
            filter: LibraryFilter::All,
        }
    }
}

fn filter_key(filter: LibraryFilter) -> &'static str {
    match filter {
        LibraryFilter::All => "all",
        LibraryFilter::Running => "running",
        LibraryFilter::Updates => "updates",
    }
}

impl Render for LibraryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.clone();
        let (arrows, catalog) = app
            .read(cx)
            .services
            .arrows
            .store()
            .read(|s| (s.arrows.values().cloned().collect::<Vec<_>>(), s.catalog));
        // The catalog row carries no release information; the runtime's `outdated` state is the
        // client's only record that something newer exists.
        let updates: Vec<String> = arrows
            .iter()
            .filter(|a| a.state == ArrowState::Outdated)
            .map(|a| a.namespace.clone())
            .collect();
        let show_self = app.read(cx).show_self_components;
        let model = build_library(&arrows, &updates, show_self, self.filter);
        let (title, subtitle) = {
            let a = app.read(cx);
            (
                a.t(home::LIBRARY),
                a.translator.t_count(keys::SUBTITLE, model.total),
            )
        };
        let header = page_header(&app, "library", title, Some(subtitle), vec![], cx);
        let body = if catalog == CatalogStatus::Loading && arrows.is_empty() {
            skeleton("library", 8, TileSize::Compact.width(), cx).into_any_element()
        } else if catalog == CatalogStatus::Error && arrows.is_empty() {
            let a = app.read(cx);
            let (title, body, retry) = (
                a.t(sidebar::ARROWS_ERROR_TITLE),
                a.t(sidebar::ARROWS_ERROR_BODY),
                a.t(app::CRASH_RETRY),
            );
            let services = a.services.clone();
            let action = Button::new("library.error.retry")
                .variant(ButtonVariant::Outline)
                .label(retry.clone())
                .child(probe("library.error.retry", Kind::Button, retry, true, ""))
                .on_click(move |_, _, _| services.arrows.refresh_catalog());
            notice("library.error", title, body, Some(action), cx).into_any_element()
        } else if model.total == 0 {
            let a = app.read(cx);
            let (title, body, cta) = (
                a.t(home::EMPTY_TITLE),
                a.t(home::EMPTY_DESCRIPTION),
                a.t(home::EMPTY_CTA),
            );
            let go = app.clone();
            let action = Button::new("library.empty.search")
                .label(cta.clone())
                .child(probe("library.empty.search", Kind::Button, cta, true, ""))
                .on_click(move |_, _, cx| {
                    go.update(cx, |m, cx| {
                        m.navigate(
                            Route::Search {
                                query: String::new(),
                            },
                            cx,
                        )
                    });
                });
            notice("library.empty", title, body, Some(action), cx).into_any_element()
        } else {
            let mut chips = div().flex().flex_wrap().gap_2().mb_5();
            for (filter, count) in model.counts {
                let label = format!("{} {count}", app.read(cx).t(filter.label()));
                chips = chips.child(chip(
                    format!("library.filter.{}", filter_key(filter)),
                    label,
                    self.filter == filter,
                    cx,
                    move |this: &mut Self, cx| {
                        this.filter = filter;
                        cx.notify();
                    },
                ));
            }
            let grid = if model.shown.is_empty() {
                let nothing = app.read(cx).t(search::NARROW_EMPTY);
                notice("library.filtered-empty", nothing, String::new(), None, cx)
                    .into_any_element()
            } else {
                let tiles = model.shown.iter().map(|entry| {
                    tile(
                        "library",
                        &library_input(entry),
                        TileSize::Compact,
                        &app,
                        cx,
                    )
                    .into_any_element()
                });
                tile_grid(tiles).into_any_element()
            };
            div().child(chips).child(grid).into_any_element()
        };
        div()
            .id("library")
            .relative()
            .size_full()
            .bg(p.background)
            .child(page(
                "library.scroll",
                p.background,
                div().flex().flex_col().child(header).child(body),
            ))
            .child(screen_probe("library"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;

    #[gpui::test]
    fn the_view_starts_on_all_and_each_chip_has_its_own_e2e_key(cx: &mut gpui::TestAppContext) {
        let app = demo_app(cx);
        let view = cx.new(|cx| LibraryView::new(app, cx));
        assert_eq!(view.read_with(cx, |v, _| v.filter), LibraryFilter::All);
        let keys: Vec<_> = quiver_state::library::FILTERS.map(filter_key).to_vec();
        assert_eq!(keys, ["all", "running", "updates"]);
    }
}

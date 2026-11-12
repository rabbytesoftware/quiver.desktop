//! Recommended (`RecommendedAfter`): "Popular" in full. Header with "N loaded", category chips, the
//! and the tiles. The decisions come from `quiver_state::recommended`.

use gpui::prelude::*;
use gpui::{div, Context, Entity, Render, Window};
use quiver_state::i18n::en::{design, recommended as keys};
use quiver_state::i18n::Params;
use quiver_state::queries::keys::home_key;
use quiver_state::recommended::build_recommended;
use quiver_state::tile::BadgePolicy;
use quiver_ui::{ActiveQuiver, TileSize};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::grid_page::{
    chip, error_state, grid_tile_width, notice, page, page_header, skeleton, tile_grid,
};
use crate::views::screen_probe;
use crate::views::tiles::{discovered_input, tile};

pub struct RecommendedView {
    app: Entity<AppModel>,
    category: Option<String>,
}

impl RecommendedView {
    pub fn new(
        app: Entity<AppModel>,
        cx: &mut Context<Self>,
    ) -> Self {
        let home = app.read(cx).home.store().clone();
        observe_store(cx, &home);
        Self {
            app,
            category: None,
        }
    }

    fn select(
        &mut self,
        category: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.category = category;
        cx.notify();
    }
}

impl Render for RecommendedView {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let app = self.app.clone();
        let home = app.read(cx).home.snapshot();
        let model = build_recommended(home.data.as_ref(), self.category.as_deref());
        let first_load = home.data.is_none() && home.error.is_none();
        let (title, subtitle, all) = {
            let a = app.read(cx);
            (
                a.t(design::POPULAR_TITLE),
                a.translator
                    .t_with(design::POPULAR_LOADED, &Params::count(model.loaded)),
                a.t(design::POPULAR_ALL),
            )
        };
        let header = page_header(&app, "recommended", title, Some(subtitle), vec![], cx);
        let body = if home.error.is_some() && home.data.is_none() {
            error_state("recommended", &app, home_key(), cx).into_any_element()
        } else if first_load || model.show_skeleton {
            skeleton("recommended", 8, TileSize::Compact.width(), cx).into_any_element()
        } else if model.total == 0 {
            let empty = app.read(cx).t(keys::EMPTY);
            notice("recommended.empty", empty, String::new(), None, cx).into_any_element()
        } else {
            let mut chips = div().flex().flex_wrap().gap_2().mb_4();
            chips = chips.child(chip(
                "recommended.chip.all".into(),
                all,
                self.category.is_none(),
                cx,
                |this: &mut Self, cx| this.select(None, cx),
            ));
            for tag in &model.categories {
                let value = tag.clone();
                chips = chips.child(chip(
                    format!("recommended.chip.{tag}"),
                    tag.clone(),
                    self.category.as_deref() == Some(tag.as_str()),
                    cx,
                    move |this: &mut Self, cx| this.select(Some(value.clone()), cx),
                ));
            }
            let width = grid_tile_width(&app, window, model.arrows.len(), cx);
            let tiles = model.arrows.iter().map(|entry| {
                let input = discovered_input(entry, BadgePolicy::ActivityOnly);
                tile("recommended", &input, TileSize::Custom(width), &app, cx).into_any_element()
            });
            div()
                .child(chips)
                .child(tile_grid(tiles))
                .into_any_element()
        };
        div()
            .id("recommended")
            .relative()
            .size_full()
            .bg(p.background)
            .child(page(
                "recommended.scroll",
                p.background,
                div().flex().flex_col().child(header).child(body),
            ))
            .child(screen_probe("recommended"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::grid_page::testing::demo_app;

    #[gpui::test]
    fn a_chip_narrows_the_grid_to_its_tag(cx: &mut gpui::TestAppContext) {
        let app = demo_app(cx);
        let view = cx.new(|cx| RecommendedView::new(app.clone(), cx));
        view.update(cx, |v, cx| {
            v.select(Some("game".into()), cx);
            assert_eq!(v.category.as_deref(), Some("game"));
            v.select(None, cx);
            assert_eq!(v.category, None);
        });
        let model = app.read_with(cx, |m, _| {
            build_recommended(m.home.snapshot().data.as_ref(), Some("game"))
        });
        assert_eq!(model.arrows.len(), 5, "the demo shelf has five game arrows");
    }
}

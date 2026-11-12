//! Popular: category chips and every tile the shelf holds.

use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, FontWeight, IntoElement};
use quiver_state::domain::Home;
use quiver_state::i18n::en::{design, home as keys};
use quiver_state::recommended::build_recommended;
use quiver_state::search::tile_width;
use quiver_state::tile::BadgePolicy;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, SectionHeader, TileSize};

use crate::model::Route;
use crate::views::home::HomeView;
use crate::views::tiles::{discovered_input, tile};

fn chip(
    id: String,
    probe_id: String,
    label: String,
    active: bool,
    cx: &mut Context<HomeView>,
    on_click: impl Fn(&mut HomeView, &mut Context<HomeView>) + 'static,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .id(gpui::SharedString::from(id))
        .relative()
        .flex()
        .items_center()
        .h(px(28.))
        .px(px(12.))
        .rounded(px(14.))
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor(CursorStyle::PointingHand)
        .when(active, |d| d.bg(p.primary).text_color(p.primary_foreground))
        .when(!active, |d| {
            d.bg(p.background)
                .text_color(p.foreground)
                .border_1()
                .border_color(p.foreground.opacity(0.17))
        })
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(label.clone())
        .child(probe(
            probe_id,
            Kind::Button,
            label,
            true,
            if active { "selected" } else { "" },
        ))
}

pub fn popular(
    view: &mut HomeView,
    home: Option<&Home>,
    content: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let app = view.app.clone();
    let recommended = build_recommended(home, view.category.as_deref());
    let (title, link, all) = {
        let a = app.read(cx);
        (
            a.t(design::POPULAR_TITLE),
            a.translator
                .t_count(keys::VIEW_ALL_RECOMMENDED, recommended.total),
            a.t(design::POPULAR_ALL),
        )
    };
    let go = app.clone();
    let mut chips = div().flex().flex_wrap().gap(px(8.)).mb(px(16.)).child(chip(
        "home-chip-all".into(),
        "home.chip.all".into(),
        all,
        view.category.is_none(),
        cx,
        |this, cx| {
            this.category = None;
            cx.notify();
        },
    ));
    for tag in &recommended.categories {
        let selected = view.category.as_deref() == Some(tag.as_str());
        let value = tag.clone();
        chips = chips.child(chip(
            format!("home-chip-{tag}"),
            format!("home.chip.{tag}"),
            tag.clone(),
            selected,
            cx,
            move |this, cx| {
                this.category = Some(value.clone());
                cx.notify();
            },
        ));
    }
    // The tile size follows the room there is, as on every grid page: a window too narrow for two wide tiles
    // gets the smaller ones, several to a row, not one wide tile and a gap.
    let width = tile_width(recommended.arrows.len(), content);
    let tiles = recommended.arrows.iter().map(|entry| {
        let input = discovered_input(entry, BadgePolicy::ActivityOnly);
        tile("home.popular", &input, TileSize::Custom(width), &app, cx)
    });
    div()
        .child(SectionHeader::new("home-popular", title).link(
            link,
            "home.popular.browse-all",
            move |_, _, cx| {
                go.update(cx, |m, cx| m.navigate(Route::Recommended, cx));
            },
        ))
        .when(!recommended.categories.is_empty(), |d| d.child(chips))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(12.))
                .gap_y(px(20.))
                .children(tiles),
        )
}

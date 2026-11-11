//! Popular: category chips, the shelf's tiles and "Load more" on a wavy line.

use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, FontWeight, IntoElement};
use quiver_state::domain::Home;
use quiver_state::home::popular_slice;
use quiver_state::i18n::en::{design, home as keys};
use quiver_state::recommended::build_recommended;
use quiver_state::tile::BadgePolicy;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, SectionHeader, TileSize, WavyLine};

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
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let app = view.app.clone();
    let recommended = build_recommended(home, view.category.as_deref(), 1);
    let slice = popular_slice(recommended.arrows.len(), view.extra);
    let (shown, can_more) = (slice.shown, slice.can_load_more);
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
            this.extra = 0;
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
                this.extra = 0;
                cx.notify();
            },
        ));
    }
    let tiles = recommended.arrows.iter().take(shown).map(|entry| {
        let input = discovered_input(entry, BadgePolicy::ActivityOnly);
        tile("home.popular", &input, TileSize::Wide, &app, cx)
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
        .when(can_more, |d| {
            let label = app.read(cx).t(design::POPULAR_LOAD_MORE);
            d.child(
                div()
                    .relative()
                    .h(px(64.))
                    .mt(px(26.))
                    .child(
                        div().absolute().left_0().right_0().top(px(21.)).child(
                            WavyLine::new()
                                .color(p.border)
                                .amplitude(3.)
                                .wavelength(14.),
                        ),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px(17.))
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .id("home-load-more")
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(124.))
                                    .h(px(30.))
                                    .rounded(px(15.))
                                    .bg(p.background)
                                    .border_1()
                                    .border_color(p.foreground.opacity(0.17))
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .cursor(CursorStyle::PointingHand)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.extra += 1;
                                        cx.notify();
                                    }))
                                    .child(label.clone())
                                    .child(probe(
                                        "home.popular.load-more",
                                        Kind::Button,
                                        label,
                                        true,
                                        "",
                                    )),
                            ),
                    ),
            )
        })
}

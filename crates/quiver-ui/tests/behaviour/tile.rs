use gpui::prelude::*;
use gpui::{Context, TestAppContext, Window};
use quiver_ui::{CardArt, Tile, TileArtKind, TileSize};

use crate::open;

fn card() -> CardArt {
    CardArt {
        initial: "D".into(),
        name: "DOOM".into(),
        publisher: "id".into(),
        hue: 20,
    }
}

struct Host {
    size: TileSize,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> impl IntoElement {
        Tile::new("tile", "github.com/x/doom", TileArtKind::Banner(card()))
            .media(Some("file:///nonexistent/banner.png".into()), None)
            .size(self.size)
            .hover("DOOM", vec!["fps".into(), "game".into()])
    }
}

fn has_tags(
    cx: &mut TestAppContext,
    size: TileSize,
) -> bool {
    let (_host, cx) = open(cx, move |_, _| Host { size });
    cx.debug_bounds("tile-tag").is_some()
}

#[gpui::test]
fn small_tiles_have_no_tags(cx: &mut TestAppContext) {
    assert!(!has_tags(cx, TileSize::Compact));
}

#[gpui::test]
fn shelf_tiles_have_no_tags(cx: &mut TestAppContext) {
    assert!(!has_tags(cx, TileSize::Shelf));
}

#[gpui::test]
fn the_big_variants_keep_their_tags(cx: &mut TestAppContext) {
    assert!(has_tags(cx, TileSize::Wide));
}

#[gpui::test]
fn the_large_variant_keeps_its_tags(cx: &mut TestAppContext) {
    assert!(has_tags(cx, TileSize::Large));
}

#[gpui::test]
fn a_banner_that_has_not_loaded_shows_the_drawn_card(cx: &mut TestAppContext) {
    let (_host, cx) = open(cx, move |_, _| Host {
        size: TileSize::Wide,
    });
    assert!(
        cx.debug_bounds("tile-card").is_some(),
        "until the banner has loaded the tile draws its card"
    );
}

//! Gallery page: the arrow tile in every state, search input, section and page headers.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context};

use super::layout::{row, section};
use super::state::Gallery;
use crate::tile_art::TileArtKind;
use crate::{
    PageHeader, SearchInput, SectionHeader, Tile, TileBadge, TileSize, TileStart, TileStatus,
};

fn drawn(
    name: &str,
    hue: u16,
) -> TileArtKind {
    TileArtKind::Drawn {
        initial: name[..1].to_string().into(),
        name: name.to_string().into(),
        publisher: "rabbyte".into(),
        hue,
    }
}

fn tile(
    id: &'static str,
    name: &str,
    hue: u16,
    status: Option<(TileStatus, &str)>,
) -> Tile {
    Tile::new(id, id, drawn(name, hue))
        .size(TileSize::Compact)
        .caption("Automatic-HTTPS reverse proxy with a second line of text that wraps and is cut.")
        .badge(status.map(|(s, l)| TileBadge::new(s, l.to_string())))
}

pub(super) fn page(
    g: &mut Gallery,
    cx: &mut Context<Gallery>,
) -> Vec<AnyElement> {
    let banner = Tile::new("t-mc", "t-mc", TileArtKind::Banner { hue: 62 })
        .caption("Vanilla Minecraft dedicated server with automatic world backups.")
        .badge(Some(TileBadge::new(TileStatus::Running, "Running")))
        .hover(
            "Minecraft Server",
            vec!["game".into(), "server".into(), "java".into()],
        );
    let start = Tile::new("t-pg", "t-pg", drawn("PostgreSQL", 255))
        .caption("PostgreSQL 17 with the timescale extension preinstalled.")
        .start(Some(TileStart::new("Start", |_, _, _| {})));
    let attention = Tile::new("t-st", "t-st", drawn("Syncthing", 110))
        .caption("Running, but the daemon lost track of the process it started.")
        .badge(Some(TileBadge::new(
            TileStatus::Detached,
            "Needs attention",
        )));
    let states = vec![
        tile(
            "t-a",
            "Caddy",
            330,
            Some((TileStatus::Updating, "Updating…")),
        ),
        tile(
            "t-b",
            "Grafana",
            225,
            Some((TileStatus::Stopping, "Stopping…")),
        ),
        tile(
            "t-c",
            "Terraria",
            345,
            Some((TileStatus::UpdateAvailable, "Update")),
        ),
        tile(
            "t-d",
            "Valheim",
            80,
            Some((TileStatus::Installed, "Installed")),
        ),
        tile(
            "t-e",
            "Redis",
            70,
            Some((TileStatus::NotInstalled, "Not installed")),
        ),
    ];
    vec![
        section(
            "Tile",
            vec![
                div()
                    .flex()
                    .gap(px(12.))
                    .child(banner)
                    .child(start)
                    .child(attention)
                    .into_any_element(),
                div()
                    .flex()
                    .gap(px(12.))
                    .children(states)
                    .into_any_element(),
            ],
            cx,
        ),
        section(
            "Search input and headers",
            vec![
                row(
                    "SearchInput",
                    vec![div()
                        .w(px(230.))
                        .child(SearchInput::new("g-search", &g.search))
                        .into_any_element()],
                    cx,
                ),
                SectionHeader::new("g-sec", "Library")
                    .link("View all 13 →", "g.sec.link", |_, _, _| {})
                    .into_any_element(),
                PageHeader::new("Library")
                    .description("13 installed arrows")
                    .into_any_element(),
            ],
            cx,
        ),
    ]
}

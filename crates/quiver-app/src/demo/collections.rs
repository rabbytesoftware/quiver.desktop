//! Demo data for Collections and Collection (the boards' "Game Servers" with a member that did not
//! resolve). Seeded straight into the views' queries: the demo has no daemon to fetch from.

use quiver_state::domain::{
    CollectionArrow, CollectionDetail, CollectionListItem, CollectionMedia,
};
use quiver_state::queries::Query;

const GAME_SERVERS: &str = "github.com/rabbyte/game-servers";

fn item(
    namespace: &str,
    name: &str,
    description: &str,
    tags: &[&str],
    count: usize,
) -> CollectionListItem {
    CollectionListItem {
        namespace: namespace.into(),
        name: name.into(),
        description: description.into(),
        tags: tags.iter().map(|t| (*t).into()).collect(),
        followed: true,
        arrow_count: count,
    }
}

fn game_servers() -> CollectionListItem {
    item(
        GAME_SERVERS,
        "Game Servers",
        "Everything needed to host a weekend with friends.",
        &["gaming", "self-hosted"],
        4,
    )
}

fn followed() -> Vec<CollectionListItem> {
    vec![
        game_servers(),
        item(
            "github.com/rabbyte/monitoring-stack",
            "Monitoring Stack",
            "A minimal metrics and caching layer for other arrows to lean on.",
            &["ops", "monitoring"],
            2,
        ),
    ]
}

pub fn seed_followed(query: &Query<Vec<CollectionListItem>>) {
    query.store().update(|s| s.data = Some(followed()));
}

fn member(
    namespace: &str,
    version: Option<&str>,
    name: Option<&str>,
    description: &str,
) -> CollectionArrow {
    CollectionArrow {
        namespace: namespace.into(),
        version: version.map(Into::into),
        resolved: name.is_some(),
        name: name.map(Into::into),
        description: name.map(|_| description.into()),
        icon: None,
        banner: None,
    }
}

pub fn seed_detail(
    query: &Query<CollectionDetail>,
    id: &str,
) {
    let item = followed()
        .into_iter()
        .find(|c| c.namespace == id)
        .unwrap_or_else(game_servers);
    let arrows = vec![
        member(
            "github.com/rabbyte/minecraft",
            Some("v1.21.4"),
            Some("Minecraft Server"),
            "Vanilla Minecraft dedicated server with automatic world backups.",
        ),
        member(
            "github.com/rabbyte/terraria",
            Some("v1.4.4.9"),
            Some("Terraria Server"),
            "tModLoader-compatible Terraria dedicated server.",
        ),
        member(
            "github.com/rabbyte/factorio",
            Some("v2.0.28"),
            Some("Factorio Headless"),
            "Headless Factorio server for co-op games.",
        ),
        member("github.com/rabbyte/ark-survival", Some("v3.1.0"), None, ""),
    ];
    let detail = CollectionDetail {
        url: Some(format!("https://{}", item.namespace)),
        maintainers: vec!["rabbyte".into()],
        media: CollectionMedia {
            icon: None,
            banner: Some("demo".into()),
        },
        arrows,
        item,
    };
    query.store().update(|s| s.data = Some(detail));
}

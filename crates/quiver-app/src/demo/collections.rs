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
        &["game", "server"],
        4,
    )
}

fn followed() -> Vec<CollectionListItem> {
    vec![
        item(
            "github.com/sparkdev/dev-toolbox",
            "Dev Toolbox",
            "Version managers, runners and pooling for a developer machine.",
            &["developer-tools"],
            3,
        ),
        game_servers(),
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

pub fn seed_detail(query: &Query<CollectionDetail>, id: &str) {
    let item = followed()
        .into_iter()
        .find(|c| c.namespace == id)
        .unwrap_or_else(game_servers);
    let arrows = vec![
        member(
            "github.com/rabbyte/valheim",
            Some("v0.218.15"),
            Some("Valheim Server"),
            "Valheim dedicated server.",
        ),
        member(
            "github.com/northwind/rust-server",
            Some("v2.1.0"),
            Some("Rust Server"),
            "Rust dedicated server with wipe scheduling and Oxide mod support.",
        ),
        member(
            "github.com/aetherforge/terraria-tshock",
            Some("v1.4.4.9"),
            Some("Terraria Server"),
            "Dedicated Terraria server with world autosave and a TShock plugin loader.",
        ),
        member("github.com/rabbyte/valheim-plus", Some("v1.0.0"), None, ""),
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

//! `quiver --demo`: the app over seeded stores and no daemon, so screens can be captured and clicked
//! through without Core. The data is the AFTER boards' data (library, recents, popular).

use std::sync::Arc;

use quiver_core_client::stores::MemorySecretStore;
use quiver_core_client::LocalConfig;
use quiver_state::domain::{ArrowState, Home, HomeShelf, RuntimeUpdate, SearchEntry};
use quiver_state::persistence::ArrowCatalogRecord;

use crate::model::AppModel;
use crate::services::{AppServices, NativeReleaseSource, ServicesConfig};

/// Services for the demo: a scratch data dir, an in-memory keyring, no helper process. The daemon
/// is not found, so the connection simply stays down.
pub fn services(
    runtime: &'static tokio::runtime::Runtime,
) -> crate::services::Result<Arc<AppServices>> {
    let scratch = std::env::temp_dir().join(format!("quiver-demo-{}", std::process::id()));
    std::fs::create_dir_all(&scratch)
        .map_err(|e| crate::services::Error::Runtime(e.to_string()))?;
    let config = ServicesConfig {
        data_dir: scratch.clone(),
        local: LocalConfig::new(scratch.to_str(), None),
        secrets: Arc::new(MemorySecretStore::default()),
        launcher: None,
        release: Arc::new(NativeReleaseSource::new()),
        platform: quiver_platform::platform_key(),
    };
    Ok(Arc::new(runtime.block_on(AppServices::build(config))))
}

struct Row {
    name: &'static str,
    slug: &'static str,
    version: &'static str,
    state: ArrowState,
    description: &'static str,
    used: Option<&'static str>,
    banner: bool,
}

const fn row(
    name: &'static str,
    slug: &'static str,
    version: &'static str,
    state: ArrowState,
    description: &'static str,
    used: Option<&'static str>,
) -> Row {
    Row {
        name,
        slug,
        version,
        state,
        description,
        used,
        banner: false,
    }
}

fn library() -> Vec<Row> {
    use ArrowState::*;
    vec![
        row(
            "Caddy",
            "caddy",
            "v2.8.4",
            Updating,
            "Automatic-HTTPS reverse proxy.",
            None,
        ),
        row(
            "Factorio Headless",
            "factorio",
            "v2.0.28",
            Ready,
            "Headless Factorio server for co-op games.",
            None,
        ),
        row(
            "Grafana",
            "grafana",
            "v11.4.0",
            Stopping,
            "Dashboards for the metrics your arrows emit.",
            None,
        ),
        row(
            "Jellyfin",
            "jellyfin",
            "v10.10.3",
            Uninstalling,
            "Media server. Currently being removed.",
            None,
        ),
        Row {
            banner: true,
            ..row(
                "Minecraft Server",
                "minecraft",
                "v1.21.4",
                Running,
                "Vanilla Minecraft dedicated server with automatic world backups.",
                Some("2026-11-09T10:00:00Z"),
            )
        },
        row(
            "NATS",
            "nats",
            "v2.10.24",
            Draining,
            "Message broker. Draining connections before it stops.",
            None,
        ),
        row(
            "Pixelmon Asset Pack",
            "pixelmon-assets",
            "v9.2.1",
            Ready,
            "Texture and model assets. A package with nothing to run.",
            None,
        ),
        row(
            "Plex",
            "plex",
            "v1.41.3",
            Removed,
            "Removed from disk; the entry survives so it can be reinstalled.",
            None,
        ),
        row(
            "PostgreSQL",
            "postgres",
            "v17.2",
            Ready,
            "PostgreSQL 17 with the timescale extension preinstalled.",
            Some("2026-11-08T10:00:00Z"),
        ),
        row(
            "Redis",
            "redis",
            "v7.4.1",
            Installing,
            "Redis 7 keystore.",
            None,
        ),
        row(
            "Syncthing",
            "syncthing",
            "v1.28.1",
            Detached,
            "Running, but the daemon lost track of the process it started.",
            Some("2026-11-07T10:00:00Z"),
        ),
        row(
            "Terraria Server",
            "terraria",
            "v1.4.4.9",
            Outdated,
            "Dedicated Terraria server.",
            None,
        ),
        row(
            "Valheim Server",
            "valheim",
            "v0.218.15",
            Ready,
            "Valheim dedicated server.",
            None,
        ),
    ]
}

fn popular(
    publisher: &str,
    slug: &str,
    name: &str,
    description: &str,
    tags: &[&str],
) -> SearchEntry {
    SearchEntry {
        namespace: format!("github.com/{publisher}/{slug}"),
        name: name.into(),
        description: description.into(),
        tags: tags.iter().map(|t| (*t).into()).collect(),
        icon: None,
        banner: None,
        versions: vec!["v1".into()],
        compatible_os: vec![],
        provenance: None,
        installed: false,
        known: true,
        stars: 0,
        source: None,
        origin: None,
        confidence: None,
    }
}

/// Fills the arrow store and the home query.
pub fn seed(model: &AppModel) {
    let rows = library();
    let records: Vec<ArrowCatalogRecord> = rows
        .iter()
        .map(|r| ArrowCatalogRecord {
            connection_id: "local".into(),
            namespace: format!("github.com/rabbyte/{}", r.slug),
            name: r.name.into(),
            description: r.description.into(),
            tags: if r.banner {
                vec!["game".into(), "server".into(), "java".into()]
            } else {
                vec![]
            },
            icon: None,
            banner: r.banner.then(|| "demo".into()),
            version: r.version.into(),
            last_used_at: r.used.map(Into::into),
            origin: None,
            confidence: None,
        })
        .collect();
    model.services.arrows.set_catalog(&records);
    for (r, record) in rows.iter().zip(&records) {
        model.services.arrows.seed_initial_state(RuntimeUpdate {
            namespace: record.namespace.clone(),
            state: r.state,
            active_run: None,
            last_return: None,
        });
    }
    let shelf = HomeShelf {
        id: "popular".into(),
        title: "Popular".into(),
        refreshed_at: None,
        arrows: vec![
            popular(
                "aetherforge",
                "terraria-tshock",
                "Terraria Server",
                "Dedicated Terraria server with world autosave and a TShock plugin loader.",
                &["game", "server"],
            ),
            popular(
                "northwind",
                "rust-server",
                "Rust Server",
                "Rust dedicated server with wipe scheduling and Oxide mod support.",
                &["game", "server"],
            ),
            popular(
                "loomlabs",
                "factorio-headless",
                "Factorio Headless",
                "Headless Factorio server with automatic save rotation.",
                &["game"],
            ),
            popular(
                "sparkdev",
                "mise",
                "Mise",
                "Polyglot tool version manager for your dev environments.",
                &["developer-tools"],
            ),
            popular(
                "hearthchat",
                "matrix-synapse",
                "Synapse",
                "A Matrix homeserver for private, federated chat.",
                &["chat", "server"],
            ),
            popular(
                "pgfleet",
                "pg-bouncer",
                "PgBouncer",
                "Lightweight connection pooler for PostgreSQL.",
                &["database"],
            ),
            popular(
                "loomlabs",
                "valkey",
                "Valkey",
                "Open source, in-memory key value store.",
                &["database"],
            ),
            popular(
                "sparkdev",
                "act-runner",
                "Act Runner",
                "Run your CI workflows on this machine.",
                &["developer-tools"],
            ),
        ],
    };
    model.home.store().update(|s| {
        s.data = Some(Home {
            shelves: vec![shelf],
            refreshing: false,
        });
    });
}

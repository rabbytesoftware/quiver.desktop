//! `quiver --demo`: the app over seeded stores and no daemon, so screens can be captured and clicked
//! through without Core. The data is the AFTER boards' data (library, recents, popular).

use std::sync::Arc;

use quiver_core_client::stores::MemorySecretStore;
use quiver_core_client::LocalConfig;
use quiver_state::domain::{ArrowState, Home, HomeShelf, RuntimeUpdate, SearchEntry};
use quiver_state::persistence::ArrowCatalogRecord;

pub mod collections;
pub mod route;
pub mod screens;
pub mod search;

use crate::model::AppModel;
use crate::services::{AppServices, NativeReleaseSource, ServicesConfig};

/// Services for the demo: a scratch data dir, an in-memory keyring, no helper process. The daemon
/// is not found, so the connection simply stays down.
pub fn services(
    runtime: &'static tokio::runtime::Runtime
) -> crate::services::Result<Arc<AppServices>> {
    let scratch = std::env::temp_dir().join(format!("quiver-demo-{}", std::process::id()));
    std::fs::create_dir_all(&scratch)
        .map_err(|e| crate::services::Error::Runtime(e.to_string()))?;
    let config = ServicesConfig {
        data_dir: scratch.clone(),
        local: LocalConfig::new(scratch.to_str(), None),
        secrets: Arc::new(MemorySecretStore::default()),
        pages: None,
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
    tags: &'static [&'static str],
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
        tags: tags_of(slug),
    }
}

const fn tags_of(slug: &str) -> &'static [&'static str] {
    match slug.as_bytes() {
        b"minecraft" => &["game", "server", "java"],
        b"valheim" | b"factorio" | b"terraria" => &["game", "server"],
        b"pixelmon-assets" => &["assets", "package"],
        b"postgres" | b"redis" => &["database", "service"],
        b"caddy" | b"nats" => &["network", "service"],
        b"grafana" => &["observability", "service"],
        b"syncthing" => &["storage", "service"],
        _ => &["media", "service"],
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
            "Message broker. Draining connections before shutdown.",
            None,
        ),
        row(
            "Pixelmon Asset Pack",
            "pixelmon-assets",
            "v9.2.1",
            Ready,
            "Texture and model assets. A package — there is nothing to run.",
            None,
        ),
        row(
            "Plex",
            "plex",
            "v1.41.3",
            Removed,
            "Removed from disk; the entry survives until the catalog prunes it.",
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
            "tModLoader-compatible Terraria dedicated server.",
            None,
        ),
        row(
            "Valheim Server",
            "valheim",
            "v0.218.15",
            Ready,
            "Dedicated Valheim server. Linux only — no macOS build is published upstream.",
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

/// `QUIVER_DEMO_ARROWS=<n>`: n extra generated library arrows (sidebar and Library tiles) for scroll and
/// long-list measurements (`cargo xtask perf`); any n > 0 also turns every demo arrow Ready (no spinners). Unset: none.
fn generated_count() -> usize {
    std::env::var("QUIVER_DEMO_ARROWS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn generated_records(count: usize) -> Vec<ArrowCatalogRecord> {
    (0..count)
        .map(|i| ArrowCatalogRecord {
            connection_id: "local".into(),
            namespace: format!("github.com/generated/arrow-{i:05}"),
            name: format!("Generated Arrow {i:05}"),
            description: format!("Generated library entry number {i} for list measurements."),
            tags: vec![],
            icon: None,
            banner: None,
            version: "v1.0.0".into(),
            last_used_at: None,
            origin: None,
            confidence: None,
        })
        .collect()
}

/// Fills the arrow store and the home query.
pub fn seed(model: &AppModel) {
    screens::seed(model);
    let rows = library();
    let records: Vec<ArrowCatalogRecord> = rows
        .iter()
        .map(|r| ArrowCatalogRecord {
            connection_id: "local".into(),
            namespace: format!("github.com/rabbyte/{}", r.slug),
            name: r.name.into(),
            description: r.description.into(),
            tags: r.tags.iter().map(|t| (*t).into()).collect(),
            icon: None,
            banner: r.banner.then(|| "demo".into()),
            version: r.version.into(),
            last_used_at: r.used.map(Into::into),
            origin: None,
            confidence: None,
        })
        .collect();
    // A long-list measurement must not include spinners: every busy arrow animates at the display rate.
    let calm = generated_count() > 0;
    let generated = generated_records(generated_count());
    let mut catalog = records.clone();
    catalog.extend(generated.iter().cloned());
    model.services.arrows.set_catalog(&catalog);
    for record in &generated {
        model.services.arrows.seed_initial_state(RuntimeUpdate {
            namespace: record.namespace.clone(),
            state: ArrowState::Ready,
            active_run: None,
            last_return: None,
        });
    }
    for (r, record) in rows.iter().zip(&records) {
        model.services.arrows.seed_initial_state(RuntimeUpdate {
            namespace: record.namespace.clone(),
            state: if calm { ArrowState::Ready } else { r.state },
            active_run: None,
            last_return: None,
        });
    }
    if calm {
        model.services.arrows.apply_runtime_update(RuntimeUpdate {
            namespace: screens::APP_NAMESPACE.into(),
            state: ArrowState::Ready,
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
                "terraria-server",
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
                &["game", "server", "headless"],
            ),
            popular(
                "quietharbor",
                "valheim-plus-server",
                "Valheim Plus Server",
                "Valheim dedicated server bundled with the ValheimPlus configuration mod.",
                &["game", "server"],
            ),
            popular(
                "pearline",
                "satisfactory-server",
                "Satisfactory Server",
                "Satisfactory dedicated server with blueprint sync.",
                &["game", "server"],
            ),
            popular(
                "dormant",
                "abandoned-server",
                "Abandoned Server",
                "Advertised by the topic, but its manifest no longer parses.",
                &["server"],
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

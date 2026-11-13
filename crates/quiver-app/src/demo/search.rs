//! Demo data for Search: a small pool of arrows (two the library already holds), narrowed by the
//! query exactly as a real answer would be, then pushed through the store the way a finished pass is.

use quiver_state::domain::{DiscoveryProvider, DiscoverySummary, SearchEntry};
use quiver_state::search::matches_query;
use quiver_state::store::{SearchJob, SearchStore};

fn entry(
    namespace: &str,
    name: &str,
    description: &str,
    tags: &[&str],
    held: bool,
    stars: u64,
) -> SearchEntry {
    SearchEntry {
        namespace: namespace.into(),
        name: name.into(),
        description: description.into(),
        tags: tags.iter().map(|t| (*t).into()).collect(),
        icon: None,
        // The library's Minecraft row has the banner (the old mock's only one), so its search card does too.
        banner: (namespace == "github.com/rabbyte/minecraft").then(|| "demo".into()),
        versions: vec!["v1".into()],
        compatible_os: vec![],
        provenance: None,
        installed: held,
        known: true,
        stars,
        source: Some("github.com".into()),
        origin: None,
        confidence: None,
    }
}

fn pool() -> Vec<SearchEntry> {
    vec![
        entry(
            "github.com/rabbyte/minecraft",
            "Minecraft Server",
            "Vanilla Minecraft dedicated server with automatic world backups.",
            &["game", "server", "java"],
            true,
            870,
        ),
        entry(
            "github.com/rabbyte/valheim",
            "Valheim Server",
            "Dedicated Valheim server. Linux only — no macOS build is published upstream.",
            &["game", "server"],
            true,
            280,
        ),
        entry(
            "github.com/rabbyte/factorio",
            "Factorio Headless",
            "Headless Factorio server for co-op games.",
            &["game", "server"],
            true,
            260,
        ),
        entry(
            "github.com/rabbyte/terraria",
            "Terraria Server",
            "tModLoader-compatible Terraria dedicated server.",
            &["game", "server"],
            true,
            300,
        ),
        entry(
            "github.com/rabbyte/rust-dedicated",
            "Rust Dedicated Server",
            "Dedicated server for Rust, with Oxide mod support.",
            &["game", "server"],
            true,
            310,
        ),
        entry(
            "github.com/rabbyte/satisfactory",
            "Satisfactory Server",
            "Dedicated server for Satisfactory.",
            &["game", "server"],
            true,
            200,
        ),
        entry(
            "github.com/aetherforge/terraria-server",
            "Terraria Server",
            "Dedicated Terraria server with world autosave and a TShock plugin loader.",
            &["game", "server"],
            false,
            95,
        ),
        entry(
            "github.com/northwind/rust-server",
            "Rust Server",
            "Rust dedicated server with wipe scheduling and Oxide mod support.",
            &["game", "server"],
            false,
            120,
        ),
        entry(
            "github.com/loomlabs/factorio-server",
            "Factorio Headless",
            "Headless Factorio server with automatic save rotation.",
            &["game", "server", "headless"],
            false,
            240,
        ),
        entry(
            "github.com/quietharbor/valheim-plus-server",
            "Valheim Plus Server",
            "Valheim dedicated server bundled with the ValheimPlus configuration mod.",
            &["game", "server"],
            false,
            150,
        ),
        entry(
            "github.com/pearline/satisfactory-server",
            "Satisfactory Server",
            "Satisfactory dedicated server with blueprint sync.",
            &["game", "server"],
            false,
            60,
        ),
    ]
}

/// What the store holds after a finished pass for `query` (nothing for a blank one).
pub fn seed(
    store: &SearchStore,
    query: &str,
) {
    store.set_query(query);
    if query.trim().is_empty() {
        return;
    }
    let (held, open): (Vec<_>, Vec<_>) = pool()
        .into_iter()
        .filter(|e| matches_query(e, query))
        .partition(|e| e.installed);
    store.set_local(held.clone());
    store.begin_pass(SearchJob {
        id: "demo".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
    });
    for entry in open {
        store.receive(entry);
    }
    let provider = |host: &str, ok: bool| DiscoveryProvider {
        host: host.into(),
        ok,
        returned: 0,
        reason: None,
        retry_after: None,
    };
    store.end_pass(DiscoverySummary {
        job_id: "demo".into(),
        query: query.into(),
        found: 0,
        verified: 0,
        skipped: 0,
        providers: vec![provider("github.com", true), provider("gitlab.com", false)],
    });
    store.settle(held);
}

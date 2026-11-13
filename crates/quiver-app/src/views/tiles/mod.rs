//! Maps `quiver_state::tile` models onto the one `quiver_ui::Tile`. Every screen that lists arrows
//! (Home, Search, Library, Collections, Collection, Recommended) builds its tiles here.

mod launch;

use gpui::{App, Entity};
use quiver_state::arrow_details::{ArrowActionKind, RunState};
use quiver_state::domain::{ArrowEntry, SearchEntry};
use quiver_state::i18n::en::arrow;
use quiver_state::namespace::bare_namespace;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_state::tile::{
    build_tile, BadgePolicy, RevealFacts, TileArt, TileBadgeKind, TileInput, TileKind, TileModel,
};
use quiver_ui::{
    CardArt, Tile, TileArtKind, TileBadge, TileReveal, TileSize, TileStart, TileStatus,
};

use crate::bridge::spawn_detached;
use crate::model::{AppModel, Route};

fn status(kind: &TileBadgeKind) -> TileStatus {
    match kind {
        TileBadgeKind::Running => TileStatus::Running,
        TileBadgeKind::NeedsAttention => TileStatus::Detached,
        TileBadgeKind::Updating => TileStatus::Updating,
        TileBadgeKind::Stopping => TileStatus::Stopping,
        TileBadgeKind::Uninstalling => TileStatus::Uninstalling,
        TileBadgeKind::Installing => TileStatus::Installing,
        TileBadgeKind::Update => TileStatus::UpdateAvailable,
        TileBadgeKind::Installed => TileStatus::Installed,
        TileBadgeKind::NotInstalled => TileStatus::NotInstalled,
        TileBadgeKind::Custom(_) => TileStatus::Custom,
    }
}

/// The model of a library arrow. Its Start button is offered for an arrow at rest: the catalog row does
/// not carry the manifest, so "has something to run" is left to Core (it refuses a no-op execute).
pub fn library_input(entry: &ArrowEntry) -> TileInput {
    TileInput {
        description: entry.description.clone(),
        icon: entry.icon.clone(),
        banner: entry.banner.clone(),
        tags: entry.tags.clone(),
        state: Some(RunState::from(entry.state)),
        user_installed: true,
        start_label: Some(arrow::ACTION_START),
        reveal: RevealFacts {
            text: entry.version.clone(),
            ..RevealFacts::default()
        },
        ..TileInput::arrow(bare_namespace(&entry.namespace), &entry.name)
    }
}

/// The model of a search / shelf result.
pub fn discovered_input(
    entry: &SearchEntry,
    policy: BadgePolicy,
) -> TileInput {
    TileInput {
        kind: TileKind::Arrow,
        description: entry.description.clone(),
        icon: entry.icon.clone(),
        banner: entry.banner.clone(),
        tags: entry.tags.clone(),
        state: Some(if entry.installed {
            RunState::Ready
        } else {
            RunState::Absent
        }),
        user_installed: entry.installed,
        start_label: entry.installed.then_some(arrow::ACTION_START),
        policy,
        // A shelf tile (old `shelf-section.tsx`) says the newest version only.
        reveal: RevealFacts {
            text: entry.versions.last().cloned().unwrap_or_default(),
            ..RevealFacts::default()
        },
        ..TileInput::arrow(bare_namespace(&entry.namespace), &entry.name)
    }
}

fn run_action(
    app: &AppModel,
    namespace: String,
    kind: ArrowActionKind,
) {
    let mutations = Mutations::new(app.services.api.clone(), app.services.queries.clone());
    spawn_detached(app.handle(), async move {
        let vars = Variables::new();
        let outcome = match kind {
            ArrowActionKind::Stop => mutations.stop(&namespace, &vars).await,
            _ => mutations.execute_arrow(&namespace, &vars).await,
        };
        if let Err(err) = outcome {
            log::warn!("{kind:?} on {namespace} failed: {err}");
        }
    });
}

/// A tile for `input`. `screen` namespaces the e2e id: `<screen>.tile.<namespace>`. It must be unique per shelf
/// (`home.recents`, `home.library`): it also keys the tile's hover state, so two tiles of one arrow would
/// otherwise lift together.
pub fn tile(
    screen: &str,
    input: &TileInput,
    size: TileSize,
    app: &Entity<AppModel>,
    cx: &App,
) -> Tile {
    // A row that assumes Start (it has no manifest) gets it only once its manifest says there is something to run.
    let gated;
    let input = if input.start_label.is_some() && input.kind == TileKind::Arrow {
        gated = TileInput {
            start_label: launch::launch_of(&input.namespace, app, cx).flatten(),
            ..input.clone()
        };
        &gated
    } else {
        input
    };
    let model = build_tile(input);
    let reveal = reveal(&input.reveal, app, cx);
    tile_from_model(screen, &model, &input.name, size, app, cx).reveal(reveal)
}

/// The strip's line: `<held> · <text>`, or `<text>` alone.
fn reveal(
    facts: &RevealFacts,
    app: &Entity<AppModel>,
    cx: &App,
) -> TileReveal {
    let text = match facts.held_as {
        Some(key) if !facts.text.is_empty() => format!("{} · {}", app.read(cx).t(key), facts.text),
        Some(key) => app.read(cx).t(key),
        None => facts.text.clone(),
    };
    TileReveal {
        text: text.into(),
        os: facts.os,
        stars: facts.stars.map(|n| compact(n).into()),
    }
}

/// `Intl.NumberFormat(…, { notation: 'compact' })` for the stars: 950, 1.2K, 34K, 1.5M.
fn compact(n: u64) -> String {
    let scaled = |v: f64, unit: &str| {
        if v < 10. {
            let s = format!("{v:.1}");
            format!("{}{unit}", s.trim_end_matches(".0"))
        } else {
            format!("{}{unit}", v.floor())
        }
    };
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => scaled(n as f64 / 1e3, "K"),
        1_000_000..=999_999_999 => scaled(n as f64 / 1e6, "M"),
        _ => scaled(n as f64 / 1e9, "B"),
    }
}

fn tile_from_model(
    screen: &str,
    model: &TileModel,
    name: &str,
    size: TileSize,
    app: &Entity<AppModel>,
    cx: &App,
) -> Tile {
    let t = |key| app.read(cx).t(key);
    let card = |drawn: &quiver_state::tile::DrawnArt| CardArt {
        initial: drawn.initial.clone().into(),
        name: drawn.name.clone().into(),
        publisher: drawn.publisher.clone().into(),
        hue: drawn.hue,
    };
    let art = match &model.art {
        TileArt::Banner { drawn, .. } => TileArtKind::Banner(card(drawn)),
        TileArt::Drawn(drawn) => TileArtKind::Drawn(card(drawn)),
    };
    let badge = model.badge.as_ref().map(|b| {
        let label = match &b.kind {
            TileBadgeKind::Custom(text) => text.clone(),
            _ => b.label().map(t).unwrap_or_default(),
        };
        TileBadge::new(status(&b.kind), label)
    });
    let namespace = model.namespace.clone();
    let id = format!("{screen}-tile-{namespace}");
    let banner = match &model.art {
        TileArt::Banner { url, .. } => Some(gpui::SharedString::from(url.clone())),
        TileArt::Drawn(_) => None,
    };
    let mut tile = Tile::new(gpui::SharedString::from(id), namespace.clone(), art)
        .media(banner, model.icon.clone().map(Into::into))
        .size(size)
        .caption(model.caption.clone())
        .badge(badge)
        .probe(format!("{screen}.tile.{namespace}"), name.to_owned());
    if let Some(hover) = &model.hover {
        tile = tile.hover(
            hover.title.clone(),
            hover.tags.iter().cloned().map(Into::into).collect(),
        );
    }
    if let Some(action) = model.hover_action {
        let (app, ns) = (app.clone(), namespace.clone());
        tile = tile.start(Some(TileStart::new(t(action.label), move |_, _, cx| {
            run_action(app.read(cx), ns.clone(), action.kind);
        })));
    }
    let app = app.clone();
    tile.on_click(move |_, _, cx| {
        let route = Route::Arrow {
            namespace: namespace.clone(),
        };
        app.update(cx, |m, cx| m.navigate(route, cx));
    })
}

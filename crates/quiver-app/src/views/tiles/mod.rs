//! Maps `quiver_state::tile` models onto the one `quiver_ui::Tile`. Every screen that lists arrows
//! (Home, Search, Library, Collections, Collection, Recommended) builds its tiles here.

use gpui::{App, Entity};
use quiver_state::arrow_details::{ArrowActionKind, RunState};
use quiver_state::domain::{ArrowEntry, SearchEntry};
use quiver_state::i18n::en::arrow;
use quiver_state::namespace::bare_namespace;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_state::tile::{
    build_tile, BadgePolicy, TileArt, TileBadgeKind, TileInput, TileKind, TileModel,
};
use quiver_ui::{Tile, TileArtKind, TileBadge, TileSize, TileStart, TileStatus};

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

/// A tile for `input`. `screen` namespaces the e2e id: `<screen>.tile.<namespace>`.
pub fn tile(
    screen: &str,
    input: &TileInput,
    size: TileSize,
    app: &Entity<AppModel>,
    cx: &App,
) -> Tile {
    let model = build_tile(input);
    tile_from_model(screen, &model, &input.name, size, app, cx)
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
    let art = match &model.art {
        TileArt::Banner { .. } => TileArtKind::Banner {
            hue: quiver_state::tile::hue_for(&model.namespace),
        },
        TileArt::Drawn {
            initial,
            name,
            publisher,
            hue,
            ..
        } => TileArtKind::Drawn {
            initial: initial.clone().into(),
            name: name.clone().into(),
            publisher: publisher.clone().into(),
            hue: *hue,
        },
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
    let mut tile = Tile::new(gpui::SharedString::from(id), namespace.clone(), art)
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

//! Everything a tile needs: art, badge, caption, hover content and hover action.

use super::badge::{badge_for, BadgePolicy, TileBadge, TileBadgeKind};
use crate::arrow_details::actions::{launch_label, ArrowActionKind};
use crate::arrow_details::facts::LifecycleFacts;
use crate::arrow_details::status::RunState;
use crate::i18n::{en::arrow, Key};
use crate::namespace::{bare_namespace, owner_of};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileKind {
    Arrow,
    Collection,
}

/// What the view layer gives a tile, filled from the store (arrow entry, search hit, collection row).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TileInput {
    pub kind: TileKind,
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub tags: Vec<String>,
    /// `None` for rows that are not arrows (collections).
    pub state: Option<RunState>,
    pub user_installed: bool,
    pub update_ahead: bool,
    /// The label of the arrow's Start action ("Start" or "Open"), `None` when it has nothing to run.
    pub start_label: Option<Key>,
    /// A badge text the caller owns (a collection's "12 arrows"); wins over the status badge.
    pub custom_badge: Option<String>,
    pub policy: BadgePolicy,
    /// What the hover lift uncovers under the art (the old tiles' 30px strip).
    pub reveal: RevealFacts,
}

/// The strip under a lifted tile (old `ArrowTile` / search `ArrowCard`): the arrow's mark, then a mono line. A
/// library arrow says its version; a search result says how the catalog holds it and the newest version, the OS
/// families it publishes for, and its stars; a collection says how many arrows it has.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevealFacts {
    /// The mono line as given (a version, "12 arrows").
    pub text: String,
    /// Prefixed to `text` as `<held> · <text>` (search: "installed").
    pub held_as: Option<Key>,
    /// Linux, macOS, Windows: whether a build exists for each (search results only).
    pub os: Option<[bool; 3]>,
    pub stars: Option<u64>,
}

/// `compatible_os` is a list of `os/arch` pairs; the family is the half before the slash.
pub fn os_families(compatible: &[String]) -> [bool; 3] {
    let has = |family: &str| {
        compatible
            .iter()
            .any(|v| v.split('/').next() == Some(family))
    };
    [has("linux"), has("darwin"), has("windows")]
}

impl TileInput {
    /// Takes the Start action (and whether it reads "Open") from the target's lifecycle.
    pub fn with_lifecycle(
        mut self,
        lifecycle: Option<&LifecycleFacts>,
    ) -> Self {
        self.start_label = lifecycle
            .filter(|l| !l.execute.is_empty())
            .map(|l| launch_label(Some(l)));
        self
    }

    pub fn arrow(
        namespace: &str,
        name: &str,
    ) -> Self {
        Self {
            kind: TileKind::Arrow,
            namespace: namespace.into(),
            name: name.into(),
            description: String::new(),
            icon: None,
            banner: None,
            tags: Vec::new(),
            state: None,
            user_installed: false,
            update_ahead: false,
            start_label: None,
            custom_badge: None,
            policy: BadgePolicy::ActivityOnly,
            reveal: RevealFacts::default(),
        }
    }
}

/// The art area. A banner is an image; without one the tile draws itself: the initial as a ghost letter,
/// the icon, the name and the publisher.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TileArt {
    /// The arrow published a banner: it covers the art once loaded. Until then (and if it cannot load) the
    /// drawn card is what shows.
    Banner { url: String, drawn: DrawnArt },
    /// No banner: the drawn card.
    Drawn(DrawnArt),
}

/// The card drawn on a tile with no banner, and under one whose banner has not loaded: ghost initial, icon,
/// title and publisher.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DrawnArt {
    pub initial: String,
    pub icon: Option<String>,
    pub name: String,
    pub publisher: String,
    pub hue: u16,
}

/// Shown over a banner on hover (a drawn tile already carries its title on the art).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HoverContent {
    pub title: String,
    pub tags: Vec<String>,
}

/// The button that appears top-right on hover.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TileAction {
    pub kind: ArrowActionKind,
    pub label: Key,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TileModel {
    pub namespace: String,
    pub kind: TileKind,
    pub art: TileArt,
    pub badge: Option<TileBadge>,
    /// Description only; the title lives on the art or the hover.
    pub caption: String,
    pub hover: Option<HoverContent>,
    pub hover_action: Option<TileAction>,
    /// `media.icon`, for the strip under the lifted art of any tile (a drawn tile also puts it in its mark).
    pub icon: Option<String>,
}

/// The mock world's swatch for a namespace (`media.ts`): one of 24 hues, 15 degrees apart, in the range
/// 0..360, plus 360 when the swatch is the lighter of its two lightnesses. `hue % 360` is the hue.
///
/// The arrow, not the identity: `ns@stable` and `ns` get the same art, so an arrow keeps its colour across the
/// sidebar (which lists identities), its tile and its page.
pub fn hue_for(namespace: &str) -> u16 {
    let namespace = crate::namespace::bare_namespace(namespace);
    let mut hash: u32 = 5381;
    for unit in namespace.encode_utf16() {
        hash = hash.wrapping_mul(33) ^ u32::from(unit);
    }
    hash = (hash ^ (hash >> 16)).wrapping_mul(0x7feb_352d);
    hash = (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;
    let hue = (hash % 24) as u16 * 15;
    hue + if (hash >> 8) % 2 == 1 { 360 } else { 0 }
}

/// Only two actions are worth a hover button, and only because Core can really do them from here:
/// Start/Open on an arrow at rest (execute is hard-gated to `ready`), and Stop on a NeedsAttention
/// (detached) one. Core has no cancel for busy states, so those badges stay informational.
fn hover_action(input: &TileInput) -> Option<TileAction> {
    if input.kind != TileKind::Arrow || !input.user_installed {
        return None;
    }
    match input.state? {
        RunState::Ready => input.start_label.map(|label| TileAction {
            kind: ArrowActionKind::Execute,
            label,
        }),
        RunState::Detached => Some(TileAction {
            kind: ArrowActionKind::Stop,
            label: arrow::ACTION_STOP,
        }),
        _ => None,
    }
}

pub fn build_tile(input: &TileInput) -> TileModel {
    let banner = input.banner.clone().filter(|b| !b.is_empty());
    let drawn = DrawnArt {
        initial: input
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_else(|| "?".into()),
        icon: input.icon.clone(),
        name: input.name.clone(),
        publisher: owner_of(bare_namespace(&input.namespace)).to_owned(),
        hue: hue_for(&input.namespace),
    };
    let art = match banner.clone() {
        Some(url) => TileArt::Banner { url, drawn },
        None => TileArt::Drawn(drawn),
    };
    let badge = match (&input.custom_badge, input.state) {
        (Some(label), _) => Some(TileBadge {
            kind: TileBadgeKind::Custom(label.clone()),
            glyph: None,
        }),
        (None, Some(state)) => badge_for(
            state,
            input.user_installed,
            input.update_ahead,
            input.policy,
        ),
        (None, None) => None,
    };
    TileModel {
        namespace: input.namespace.clone(),
        kind: input.kind,
        art,
        badge,
        caption: input.description.clone(),
        hover: banner.map(|_| HoverContent {
            title: input.name.clone(),
            tags: input.tags.clone(),
        }),
        hover_action: hover_action(input),
        icon: input.icon.clone().filter(|i| !i.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_details::facts::StepSpec;

    fn lifecycle_with_execute() -> LifecycleFacts {
        LifecycleFacts {
            execute: vec![StepSpec {
                step_type: "run".into(),
                ..StepSpec::default()
            }],
            ..LifecycleFacts::default()
        }
    }

    fn installed(state: RunState) -> TileInput {
        TileInput {
            user_installed: true,
            state: Some(state),
            start_label: Some(arrow::ACTION_START),
            description: "PostgreSQL 17.".into(),
            ..TileInput::arrow("github.com/rabbyte/postgres", "PostgreSQL")
        }
    }

    #[test]
    fn the_caption_is_the_description_only() {
        assert_eq!(
            build_tile(&installed(RunState::Ready)).caption,
            "PostgreSQL 17."
        );
    }

    #[test]
    fn a_banner_tile_shows_title_and_tags_on_hover() {
        let input = TileInput {
            banner: Some("https://x/b.png".into()),
            tags: vec!["db".into(), "sql".into()],
            ..installed(RunState::Ready)
        };
        let tile = build_tile(&input);
        assert!(matches!(
            &tile.art,
            TileArt::Banner { url, drawn } if url == "https://x/b.png" && drawn.name == "PostgreSQL"
        ));
        assert_eq!(
            tile.hover,
            Some(HoverContent {
                title: "PostgreSQL".into(),
                tags: vec!["db".into(), "sql".into()]
            })
        );
    }

    #[test]
    fn a_tile_without_a_banner_draws_itself_and_has_no_hover_card() {
        let tile = build_tile(&installed(RunState::Ready));
        let TileArt::Drawn(DrawnArt {
            initial,
            publisher,
            name,
            ..
        }) = tile.art
        else {
            panic!("expected a drawn tile")
        };
        assert_eq!(
            (initial.as_str(), publisher.as_str(), name.as_str()),
            ("P", "rabbyte", "PostgreSQL")
        );
        assert!(tile.hover.is_none());
        let empty = TileInput {
            banner: Some(String::new()),
            ..installed(RunState::Ready)
        };
        assert!(matches!(build_tile(&empty).art, TileArt::Drawn(_)));
    }

    #[test]
    fn the_hue_is_stable_and_in_range() {
        assert_eq!(
            hue_for("github.com/rabbyte/postgres"),
            hue_for("github.com/rabbyte/postgres")
        );
        assert!(hue_for("a") % 360 < 360 && hue_for("b") < 720);
        assert_ne!(hue_for("github.com/a/x"), hue_for("github.com/a/y"));
    }

    #[test]
    fn start_appears_for_a_ready_arrow_that_has_something_to_run() {
        let tile = build_tile(&installed(RunState::Ready));
        assert_eq!(
            tile.hover_action,
            Some(TileAction {
                kind: ArrowActionKind::Execute,
                label: arrow::ACTION_START
            })
        );
        let no_execute =
            installed(RunState::Ready).with_lifecycle(Some(&LifecycleFacts::default()));
        assert_eq!(build_tile(&no_execute).hover_action, None);
    }

    #[test]
    fn open_replaces_start_for_an_arrow_that_launches_its_app() {
        let mut lifecycle = lifecycle_with_execute();
        lifecycle.execute[0].command = Some("${ARROW_APP}".into());
        let input = installed(RunState::Ready).with_lifecycle(Some(&lifecycle));
        assert_eq!(
            build_tile(&input).hover_action.unwrap().label,
            arrow::ACTION_OPEN
        );
    }

    #[test]
    fn only_needs_attention_offers_a_real_stop() {
        let tile = build_tile(&installed(RunState::Detached));
        assert_eq!(
            tile.hover_action,
            Some(TileAction {
                kind: ArrowActionKind::Stop,
                label: arrow::ACTION_STOP
            })
        );
        for state in [
            RunState::Running,
            RunState::Installing,
            RunState::Updating,
            RunState::Stopping,
            RunState::Outdated,
        ] {
            assert_eq!(
                build_tile(&installed(state)).hover_action,
                None,
                "{state:?}"
            );
        }
    }

    #[test]
    fn nothing_on_hover_outside_the_library_or_for_collections() {
        let discovered = TileInput {
            user_installed: false,
            ..installed(RunState::Ready)
        };
        assert_eq!(build_tile(&discovered).hover_action, None);
        let collection = TileInput {
            kind: TileKind::Collection,
            state: None,
            ..installed(RunState::Ready)
        };
        assert_eq!(build_tile(&collection).hover_action, None);
    }

    #[test]
    fn a_custom_badge_wins_over_the_status() {
        let input = TileInput {
            kind: TileKind::Collection,
            state: None,
            custom_badge: Some("12 arrows".into()),
            ..installed(RunState::Running)
        };
        assert_eq!(
            build_tile(&input).badge.unwrap().kind,
            TileBadgeKind::Custom("12 arrows".into())
        );
    }

    #[test]
    fn the_badge_follows_the_policy() {
        let mixed = TileInput {
            policy: BadgePolicy::WithLibraryState,
            ..installed(RunState::Ready)
        };
        assert_eq!(
            build_tile(&mixed).badge.unwrap().kind,
            TileBadgeKind::Installed
        );
        assert!(build_tile(&installed(RunState::Ready)).badge.is_none());
        assert_eq!(
            build_tile(&installed(RunState::Running))
                .badge
                .unwrap()
                .kind,
            TileBadgeKind::Running
        );
    }
}

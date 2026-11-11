//! Filling a [`TileInput`] from the records each screen holds.

use super::badge::BadgePolicy;
use super::model::{os_families, RevealFacts, TileInput, TileKind};
use crate::arrow_details::RunState;
use crate::domain::{ArrowEntry, CollectionListItem, SearchEntry};
use crate::i18n::en::arrow;

impl TileInput {
    /// A library arrow (Home recents, Library). Every arrow the reactive store holds is `user_installed`
    /// by construction (the listener only seeds library arrows). Start is assumed to exist: the entry
    /// carries no lifecycle, and Core refuses an `execute` that has nothing to run.
    pub fn from_arrow_entry(entry: &ArrowEntry) -> Self {
        Self {
            description: entry.description.clone(),
            icon: entry.icon.clone(),
            banner: entry.banner.clone(),
            tags: entry.tags.clone(),
            state: Some(entry.state.into()),
            user_installed: true,
            start_label: Some(arrow::ACTION_START),
            reveal: RevealFacts {
                text: entry.version.clone(),
                ..RevealFacts::default()
            },
            ..Self::arrow(&entry.namespace, &entry.name)
        }
    }

    /// A search / shelf result. Its runtime state is not known here, so an installed arrow reads as at
    /// rest. It is mixed with arrows you do not have, so the library state is information.
    pub fn from_search_entry(entry: &SearchEntry) -> Self {
        let state = if entry.installed {
            RunState::Ready
        } else {
            RunState::Absent
        };
        Self {
            description: entry.description.clone(),
            icon: entry.icon.clone(),
            banner: entry.banner.clone(),
            tags: entry.tags.clone(),
            state: Some(state),
            user_installed: entry.installed,
            policy: BadgePolicy::WithLibraryState,
            reveal: search_reveal(entry),
            ..Self::arrow(&entry.namespace, &entry.name)
        }
    }

    /// A collection tile: the badge is its arrow count.
    pub fn from_collection_item(
        item: &CollectionListItem,
        count_label: String,
    ) -> Self {
        Self {
            kind: TileKind::Collection,
            description: item.description.clone(),
            tags: item.tags.clone(),
            reveal: RevealFacts {
                text: count_label.clone(),
                ..RevealFacts::default()
            },
            custom_badge: Some(count_label),
            ..Self::arrow(&item.namespace, &item.name)
        }
    }
}

/// The search card's strip: a held result says how the catalog holds it, then the newest version; one from the
/// network says the newest version and the OS families; both end on the stars. Refs arrive sorted ascending.
pub fn search_reveal(entry: &SearchEntry) -> RevealFacts {
    use crate::domain::SearchProvenance;
    use crate::i18n::en::search;
    let held = entry.installed || entry.provenance.is_some();
    let held_as = held.then_some(match entry.provenance {
        Some(SearchProvenance::Dependency) => search::PROVENANCE_DEPENDENCY,
        Some(SearchProvenance::Collection) => search::PROVENANCE_COLLECTION,
        Some(SearchProvenance::Seen) => search::PROVENANCE_SEEN,
        _ => search::PROVENANCE_INSTALLED,
    });
    RevealFacts {
        text: entry.versions.last().cloned().unwrap_or_default(),
        held_as,
        os: (!held).then(|| os_families(&entry.compatible_os)),
        stars: Some(entry.stars),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ArrowState, SearchProvenance};
    use crate::tile::{build_tile, TileBadgeKind};

    fn search(installed: bool) -> SearchEntry {
        SearchEntry {
            namespace: "github.com/a/b".into(),
            name: "B".into(),
            description: "d".into(),
            tags: vec![],
            icon: None,
            banner: None,
            versions: vec![],
            compatible_os: vec![],
            provenance: Some(SearchProvenance::Seen),
            installed,
            known: true,
            stars: 0,
            source: None,
            origin: None,
            confidence: None,
        }
    }

    #[test]
    fn search_results_say_whether_you_have_them() {
        let kind = |installed| {
            build_tile(&TileInput::from_search_entry(&search(installed)))
                .badge
                .unwrap()
                .kind
        };
        assert_eq!(kind(true), TileBadgeKind::Installed);
        assert_eq!(kind(false), TileBadgeKind::NotInstalled);
    }

    #[test]
    fn a_library_arrow_starts_from_its_live_state() {
        let entry = ArrowEntry {
            namespace: "github.com/a/b@stable".into(),
            name: "B".into(),
            description: "d".into(),
            tags: vec![],
            icon: None,
            banner: None,
            version: "v1".into(),
            state: ArrowState::Running,
            active_run: None,
            last_return: None,
            last_used_at: None,
            origin: None,
            confidence: None,
        };
        let tile = build_tile(&TileInput::from_arrow_entry(&entry));
        assert_eq!(tile.badge.unwrap().kind, TileBadgeKind::Running);
        let ready = ArrowEntry {
            state: ArrowState::Ready,
            ..entry
        };
        assert!(build_tile(&TileInput::from_arrow_entry(&ready))
            .hover_action
            .is_some());
    }

    #[test]
    fn a_collection_tile_wears_its_arrow_count() {
        let item = CollectionListItem {
            namespace: "github.com/a/c".into(),
            name: "C".into(),
            description: "d".into(),
            tags: vec![],
            followed: true,
            arrow_count: 3,
        };
        let tile = build_tile(&TileInput::from_collection_item(&item, "3 arrows".into()));
        assert_eq!(
            tile.badge.unwrap().kind,
            TileBadgeKind::Custom("3 arrows".into())
        );
        assert!(tile.hover_action.is_none());
    }
}

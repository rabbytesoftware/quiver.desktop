//! A single collection (port of `src/features/collection`): hero, member grid, unresolved routes.

mod membership;

pub use membership::{installed_members, is_member_installed};

use crate::domain::{collection_arrow_route, CollectionArrow, CollectionDetail};
use crate::i18n::{en::design, Key};
use crate::namespace::owner_of;

/// One cell of the member grid. The new design puts an unresolved member in the grid at the same
/// width and gap as the tiles; it used to surface only in a dialog.
#[derive(Clone, PartialEq, Debug)]
pub enum GridItem {
    Tile(MemberTile),
    Unresolved { route: String },
}

#[derive(Clone, PartialEq, Debug)]
pub struct MemberTile {
    /// The full route, not the bare namespace: two members can share a namespace at different versions.
    pub route: String,
    pub name: String,
    pub owner: String,
    pub description: String,
    pub icon: Option<String>,
    pub banner: Option<String>,
}

/// A resolved member always carries a name (quiver.core sets it in the same branch as `resolved: true`);
/// this is a display floor, not a real fallback path.
pub fn display_name(arrow: &CollectionArrow) -> String {
    match arrow.name.as_deref() {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => arrow
            .namespace
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&arrow.namespace)
            .to_owned(),
    }
}

pub fn grid_items(arrows: &[CollectionArrow]) -> Vec<GridItem> {
    arrows
        .iter()
        .map(|arrow| {
            let route = collection_arrow_route(arrow);
            if !arrow.resolved {
                return GridItem::Unresolved { route };
            }
            GridItem::Tile(MemberTile {
                name: display_name(arrow),
                owner: owner_of(&arrow.namespace).to_owned(),
                description: arrow.description.clone().unwrap_or_default(),
                icon: arrow.icon.clone(),
                banner: arrow.banner.clone(),
                route,
            })
        })
        .collect()
}

/// The exact routes that were asked for (`github.com/rabbyte/ark-survival@v3.1.0`), not the bare package
/// path: `to_collection_arrow` already split the version off every member, resolved or not.
pub fn unresolved_routes(arrows: &[CollectionArrow]) -> Vec<String> {
    arrows
        .iter()
        .filter(|a| !a.resolved)
        .map(collection_arrow_route)
        .collect()
}

#[derive(Clone, PartialEq, Debug)]
pub struct FollowButton {
    pub label: Key,
    /// Shows a check while followed.
    pub checked: bool,
    pub outline: bool,
    pub enabled: bool,
}

pub fn follow_button(followed: bool, pending: bool) -> FollowButton {
    FollowButton {
        label: if followed {
            design::COLLECTION_FOLLOWING
        } else {
            design::COLLECTION_FOLLOW
        },
        checked: followed,
        outline: followed,
        enabled: !pending,
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct HeroModel {
    pub arrow_count: usize,
    pub maintainers: Vec<String>,
    pub unresolved: usize,
    pub follow: FollowButton,
}

pub fn build_hero(detail: &CollectionDetail, follow_pending: bool) -> HeroModel {
    HeroModel {
        arrow_count: detail.arrows.len(),
        maintainers: detail.maintainers.clone(),
        unresolved: detail.arrows.iter().filter(|a| !a.resolved).count(),
        follow: follow_button(detail.item.followed, follow_pending),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(
        ns: &str,
        version: Option<&str>,
        resolved: bool,
        name: Option<&str>,
    ) -> CollectionArrow {
        CollectionArrow {
            namespace: ns.into(),
            version: version.map(Into::into),
            resolved,
            name: name.map(Into::into),
            description: None,
            icon: None,
            banner: None,
        }
    }

    #[test]
    fn a_member_without_a_name_falls_back_to_the_last_path_segment() {
        assert_eq!(
            display_name(&member("github.com/x/ark", None, true, Some("Ark"))),
            "Ark"
        );
        assert_eq!(
            display_name(&member("github.com/x/ark", None, true, None)),
            "ark"
        );
        assert_eq!(
            display_name(&member("github.com/x/ark", None, true, Some(""))),
            "ark"
        );
    }

    #[test]
    fn unresolved_routes_keep_the_version_that_was_asked_for() {
        let arrows = [
            member("github.com/x/ark", Some("v3.1.0"), false, None),
            member("github.com/x/ok", None, true, Some("Ok")),
            member("github.com/x/bare", None, false, None),
        ];
        assert_eq!(
            unresolved_routes(&arrows),
            ["github.com/x/ark@v3.1.0", "github.com/x/bare"]
        );
    }

    #[test]
    fn an_unresolved_member_sits_in_the_grid_as_a_fourth_item() {
        let items = grid_items(&[
            member("github.com/x/a", Some("v1"), true, Some("A")),
            member("github.com/x/a", Some("v2"), true, Some("A")),
            member("github.com/x/gone", None, false, None),
        ]);
        assert_eq!(items.len(), 3);
        match (&items[0], &items[1], &items[2]) {
            (GridItem::Tile(a), GridItem::Tile(b), GridItem::Unresolved { route }) => {
                assert_ne!(a.route, b.route);
                assert_eq!(a.owner, "x");
                assert_eq!(route, "github.com/x/gone");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_follow_button_flips_and_locks_while_a_request_is_in_flight() {
        let off = follow_button(false, false);
        assert_eq!(
            (off.label, off.checked, off.outline, off.enabled),
            (design::COLLECTION_FOLLOW, false, false, true)
        );
        let on = follow_button(true, true);
        assert_eq!(
            (on.label, on.checked, on.outline, on.enabled),
            (design::COLLECTION_FOLLOWING, true, true, false)
        );
    }
}

//! The full Collections view that "View all N collections" on Home leads to (port of
//! `src/features/collections`). The count and install state live in the tile's status badge.

use crate::domain::CollectionListItem;
use crate::i18n::{en::design, Params, Translator};

/// "3 arrows · all installed" when every member is in the library, else "3 arrows". `installed_members`
/// is what the view layer knows of the catalog; `None` means it does not know.
pub fn collection_badge_label(
    tr: &Translator,
    arrow_count: usize,
    installed_members: Option<usize>,
) -> String {
    if arrow_count > 0 && installed_members == Some(arrow_count) {
        tr.t_with(
            design::COLLECTION_ALL_INSTALLED,
            &Params::count(arrow_count),
        )
    } else {
        tr.t_count(design::COLLECTION_ARROWS, arrow_count)
    }
}

/// Followed collections, by name.
pub fn sorted_collections(collections: &[CollectionListItem]) -> Vec<CollectionListItem> {
    let mut sorted = collections.to_vec();
    sorted.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> CollectionListItem {
        CollectionListItem {
            namespace: format!("github.com/x/{name}"),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            followed: true,
            arrow_count: 1,
        }
    }

    #[test]
    fn the_badge_says_what_is_inside_and_whether_you_have_it() {
        let tr = Translator::default();
        assert_eq!(
            collection_badge_label(&tr, 3, Some(3)),
            "3 arrows · all installed"
        );
        assert_eq!(collection_badge_label(&tr, 3, Some(1)), "3 arrows");
        assert_eq!(collection_badge_label(&tr, 3, None), "3 arrows");
        assert_eq!(collection_badge_label(&tr, 1, Some(0)), "1 arrow");
        assert_eq!(collection_badge_label(&tr, 0, Some(0)), "0 arrows");
    }

    #[test]
    fn sorted_by_name_ignoring_case() {
        let names: Vec<_> = sorted_collections(&[item("b"), item("A"), item("c")])
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["A", "b", "c"]);
    }
}

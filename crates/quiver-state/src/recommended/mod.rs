//! The Popular page that "View all" on Home's lead shelf leads to (port of `src/features/recommended`):
//! the shelf's arrows, category chips from their tags and a Load more button.

use crate::domain::{Home, SearchEntry};
use crate::home::visible_shelves;

/// One page of tiles; the board's "6 loaded" is a sample, not a size.
pub const PAGE_SIZE: usize = 12;
/// Category chips offered above the grid.
pub const CATEGORY_TAKE: usize = 8;

#[derive(Clone, PartialEq, Debug)]
pub struct RecommendedModel {
    /// Everything the snapshot holds that you do not already have.
    pub total: usize,
    pub categories: Vec<String>,
    /// "N loaded".
    pub loaded: usize,
    pub arrows: Vec<SearchEntry>,
    pub can_load_more: bool,
    pub show_skeleton: bool,
}

/// `category` is a tag chip (None is "All"); `pages` is how many times Load more has been pressed
/// plus one.
pub fn build_recommended(
    home: Option<&Home>,
    category: Option<&str>,
    pages: usize,
) -> RecommendedModel {
    let all: Vec<SearchEntry> = visible_shelves(home)
        .into_iter()
        .flat_map(|s| s.arrows)
        .collect();
    let categories = categories_of(&all);
    let matching: Vec<SearchEntry> = all
        .iter()
        .filter(|a| category.is_none_or(|c| a.tags.iter().any(|t| t == c)))
        .cloned()
        .collect();
    let limit = PAGE_SIZE * pages.max(1);
    let loaded = matching.len().min(limit);
    RecommendedModel {
        total: all.len(),
        categories,
        loaded,
        can_load_more: matching.len() > loaded,
        arrows: matching.into_iter().take(limit).collect(),
        show_skeleton: all.is_empty() && home.is_some_and(|h| h.refreshing),
    }
}

/// Every tag at least one arrow carries, in first-seen order (unlike result facets, a tag all arrows share is
/// still a category).
fn categories_of(all: &[SearchEntry]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in all.iter().flat_map(|a| a.tags.iter()) {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }
    tags.truncate(CATEGORY_TAKE);
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HomeShelf;
    use crate::search::relevance::fixtures::entry;

    fn open(i: usize, tags: &[&str]) -> SearchEntry {
        SearchEntry {
            provenance: None,
            tags: tags.iter().map(|t| (*t).into()).collect(),
            ..entry(&format!("github.com/x/a{i}"), &format!("a{i}"))
        }
    }

    fn home(arrows: Vec<SearchEntry>) -> Home {
        Home {
            shelves: vec![HomeShelf {
                id: "p".into(),
                title: "Popular".into(),
                refreshed_at: None,
                arrows,
            }],
            refreshing: false,
        }
    }

    #[test]
    fn pages_grow_with_load_more_and_the_count_says_what_is_loaded() {
        let h = home((0..30).map(|i| open(i, &[])).collect());
        let first = build_recommended(Some(&h), None, 1);
        assert_eq!(
            (first.loaded, first.total, first.can_load_more),
            (12, 30, true)
        );
        let last = build_recommended(Some(&h), None, 3);
        assert_eq!((last.loaded, last.can_load_more), (30, false));
    }

    #[test]
    fn a_category_chip_narrows_to_arrows_carrying_the_tag() {
        let h = home(vec![
            open(0, &["game"]),
            open(1, &["chat", "game"]),
            open(2, &["database"]),
        ]);
        let model = build_recommended(Some(&h), Some("game"), 1);
        assert_eq!(model.arrows.len(), 2);
        assert_eq!(model.categories, ["game", "chat", "database"]);
        assert_eq!(model.total, 3);
    }

    #[test]
    fn held_arrows_never_appear_and_an_empty_refreshing_snapshot_shows_the_skeleton() {
        let held = SearchEntry {
            installed: true,
            ..entry("h", "h")
        };
        assert_eq!(build_recommended(Some(&home(vec![held])), None, 1).total, 0);
        let refreshing = Home {
            shelves: vec![],
            refreshing: true,
        };
        assert!(build_recommended(Some(&refreshing), None, 1).show_skeleton);
        assert!(!build_recommended(None, None, 1).show_skeleton);
    }
}

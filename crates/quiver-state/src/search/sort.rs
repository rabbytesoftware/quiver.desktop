//! Sorting the answer the client holds. Core exposes no ordering parameter (`GET /v0/search` takes `q`,
//! `limit` and `os`), so relevance is the order core returned and the other two are client-side. That is
//! honest only because the client asks for core's cap rather than its default.

use crate::domain::SearchEntry;
use crate::i18n::{en::search, Key};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SortKey {
    #[default]
    Relevance,
    Name,
    Stars,
}

pub const SORT_KEYS: [SortKey; 3] = [SortKey::Relevance, SortKey::Name, SortKey::Stars];
pub const DEFAULT_SORT: SortKey = SortKey::Relevance;

impl SortKey {
    pub fn label(self) -> Key {
        match self {
            Self::Relevance => search::SORT_RELEVANCE,
            Self::Name => search::SORT_NAME,
            Self::Stars => search::SORT_STARS,
        }
    }
}

/// Case-insensitive name order (the reader-locale collation, English).
fn by_name(
    a: &SearchEntry,
    b: &SearchEntry,
) -> std::cmp::Ordering {
    a.name
        .to_lowercase()
        .cmp(&b.name.to_lowercase())
        .then_with(|| a.name.cmp(&b.name))
}

pub fn sort_entries(
    entries: &[SearchEntry],
    sort: SortKey,
) -> Vec<SearchEntry> {
    let mut sorted = entries.to_vec();
    match sort {
        SortKey::Relevance => {}
        SortKey::Name => sorted.sort_by(by_name),
        // Descending; the sort is stable, so ties keep the order core ranked them in.
        SortKey::Stars => sorted.sort_by_key(|a| std::cmp::Reverse(a.stars)),
    }
    sorted
}

#[cfg(test)]
mod tests {
    use super::super::relevance::fixtures::entry;
    use super::*;

    fn entries() -> Vec<SearchEntry> {
        [("Velocity", 12), ("Paper", 900), ("geyser", 40)]
            .into_iter()
            .map(|(name, stars)| SearchEntry {
                stars,
                ..entry(&format!("github.com/x/{name}"), name)
            })
            .collect()
    }

    fn names(entries: &[SearchEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn relevance_is_the_order_core_ranked() {
        assert_eq!(
            names(&sort_entries(&entries(), SortKey::Relevance)),
            ["Velocity", "Paper", "geyser"]
        );
    }

    #[test]
    fn name_sorts_case_insensitively_not_by_code_point() {
        assert_eq!(
            names(&sort_entries(&entries(), SortKey::Name)),
            ["geyser", "Paper", "Velocity"]
        );
    }

    #[test]
    fn stars_sort_descending_and_do_not_mutate_the_input() {
        let input = entries();
        let sorted = sort_entries(&input, SortKey::Stars);
        assert_eq!(
            sorted.iter().map(|e| e.stars).collect::<Vec<_>>(),
            [900, 40, 12]
        );
        assert_eq!(names(&input), ["Velocity", "Paper", "geyser"]);
    }
}

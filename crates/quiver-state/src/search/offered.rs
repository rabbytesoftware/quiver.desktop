//! Which facet chips the results screen offers.

use super::narrow::{facets_for, Facet, FacetKind, Selection};
use crate::domain::SearchEntry;

const HOSTS: usize = 4;
const TAGS: usize = 12;

/// Below this the answer is readable as it stands and the row is just chrome.
const MIN_TO_NARROW: usize = 4;

/// Every host and (all but one-of-a-kind) tag of the whole answer, as the old results header offered
/// them, host counts included even when every result carries the host. A chosen facet stays so it can
/// be unchosen.
pub fn offered_facets(
    answer: &[SearchEntry],
    _selection: &Selection,
) -> Vec<(FacetKind, Facet)> {
    if answer.len() < MIN_TO_NARROW {
        return Vec::new();
    }
    [(FacetKind::Host, HOSTS), (FacetKind::Tag, TAGS)]
        .into_iter()
        .flat_map(|(kind, take)| {
            facets_for(answer, kind, take)
                .into_iter()
                .map(move |f| (kind, f))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::relevance::fixtures::entry;
    use super::*;

    fn tagged(
        ns: &str,
        tags: &[&str],
    ) -> SearchEntry {
        SearchEntry {
            tags: tags.iter().map(|t| (*t).into()).collect(),
            ..entry(ns, ns)
        }
    }

    #[test]
    fn a_short_answer_offers_nothing_to_narrow() {
        let answer = [tagged("github.com/a/x", &[]), tagged("gitlab.com/b/y", &[])];
        assert!(offered_facets(&answer, &Selection::default()).is_empty());
    }

    #[test]
    fn a_host_every_result_carries_is_still_offered_with_its_count() {
        let answer: Vec<_> = (0..4)
            .map(|i| tagged(&format!("github.com/a/x{i}"), &[]))
            .collect();
        let offered = offered_facets(&answer, &Selection::default());
        assert_eq!(offered.len(), 1);
        assert_eq!(offered[0].1.count, 4);
    }

    #[test]
    fn a_second_host_makes_hosts_a_choice_and_a_selected_one_stays() {
        let answer = [
            tagged("github.com/a/x", &[]),
            tagged("gitlab.com/b/y", &[]),
            tagged("github.com/c/z", &[]),
            tagged("github.com/d/w", &[]),
        ];
        let offered = offered_facets(&answer, &Selection::default());
        assert_eq!(offered.len(), 2);
        let narrowed = Selection {
            host: vec!["github.com".into()],
            ..Selection::default()
        };
        let kept = offered_facets(&answer, &narrowed);
        assert_eq!(
            kept.len(),
            2,
            "the facets come from the whole answer, so a chosen one never leaves"
        );
    }
}

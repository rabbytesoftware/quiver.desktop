//! Which facet chips the results screen offers.

use super::narrow::{facets_for, Facet, FacetKind, Selection};
use crate::domain::SearchEntry;

const HOSTS: usize = 4;
const TAGS: usize = 6;

/// A facet every result carries filters nothing (and a one-host answer would always show its host), so
/// it is not offered -- unless it is selected, when it must stay to be unselected.
pub fn offered_facets(answer: &[SearchEntry], selection: &Selection) -> Vec<(FacetKind, Facet)> {
    let selected = |kind: FacetKind, value: &str| match kind {
        FacetKind::Host => selection.host.iter().any(|v| v == value),
        FacetKind::Tag => selection.tag.iter().any(|v| v == value),
    };
    [(FacetKind::Host, HOSTS), (FacetKind::Tag, TAGS)]
        .into_iter()
        .flat_map(|(kind, take)| {
            facets_for(answer, kind, take)
                .into_iter()
                .filter(move |f| f.count < answer.len() || selected(kind, &f.value))
                .map(move |f| (kind, f))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::relevance::fixtures::entry;
    use super::*;

    fn tagged(ns: &str, tags: &[&str]) -> SearchEntry {
        SearchEntry {
            tags: tags.iter().map(|t| (*t).into()).collect(),
            ..entry(ns, ns)
        }
    }

    #[test]
    fn a_facet_everything_carries_is_not_offered() {
        let answer = [tagged("github.com/a/x", &[]), tagged("github.com/b/y", &[])];
        assert!(offered_facets(&answer, &Selection::default()).is_empty());
    }

    #[test]
    fn a_second_host_makes_hosts_a_choice_and_a_selected_one_stays() {
        let answer = [tagged("github.com/a/x", &[]), tagged("gitlab.com/b/y", &[])];
        let offered = offered_facets(&answer, &Selection::default());
        assert_eq!(offered.len(), 2);
        let narrowed = Selection {
            host: vec!["github.com".into()],
            ..Selection::default()
        };
        let only_github = [answer[0].clone()];
        let kept = offered_facets(&only_github, &narrowed);
        assert_eq!(
            kept.len(),
            1,
            "the selected host stays so it can be cleared"
        );
    }
}

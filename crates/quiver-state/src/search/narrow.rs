//! Narrowing a result set by host or tag.
//!
//! `compatible_os` is deliberately not a facet: the list is a projection of the last compile and
//! install-time re-resolution is authoritative, so filtering on it would hide arrows that would in fact
//! install. It is shown on the card and never offered as a control.

use crate::domain::SearchEntry;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FacetKind {
    Host,
    Tag,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Facet {
    pub value: String,
    pub count: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Selection {
    pub host: Vec<String>,
    pub tag: Vec<String>,
}

impl Selection {
    fn of(
        &self,
        kind: FacetKind,
    ) -> &Vec<String> {
        match kind {
            FacetKind::Host => &self.host,
            FacetKind::Tag => &self.tag,
        }
    }
}

pub fn is_narrowed(selection: &Selection) -> bool {
    !selection.host.is_empty() || !selection.tag.is_empty()
}

/// `github.com/PaperMC/Paper` -> `github.com`.
pub fn host_of(entry: &SearchEntry) -> &str {
    entry.namespace.split('/').next().unwrap_or("")
}

fn values_of(
    entry: &SearchEntry,
    kind: FacetKind,
) -> Vec<&str> {
    match kind {
        FacetKind::Host => vec![host_of(entry)],
        FacetKind::Tag => entry.tags.iter().map(String::as_str).collect(),
    }
}

/// Counted off the results themselves, so a facet can never name something the list does not contain.
/// A tag carried by all but one result is dropped: it is the query restated, and selecting it would
/// remove a single card while looking like a filter.
pub fn facets_for(
    entries: &[SearchEntry],
    kind: FacetKind,
    take: usize,
) -> Vec<Facet> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut first_seen: Vec<&str> = Vec::new();
    for entry in entries {
        for value in values_of(entry, kind) {
            if !counts.contains_key(value) {
                first_seen.push(value);
            }
            *counts.entry(value).or_default() += 1;
        }
    }
    let mut facets: Vec<Facet> = first_seen
        .into_iter()
        .map(|value| Facet {
            value: value.to_owned(),
            count: counts[value],
        })
        .filter(|f| kind == FacetKind::Host || f.count + 2 <= entries.len())
        .collect();
    facets.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.value.to_lowercase().cmp(&b.value.to_lowercase()))
    });
    facets.truncate(take);
    facets
}

/// Selecting inside one facet widens (OR); selecting across facets narrows (AND).
pub fn apply_selection(
    entries: &[SearchEntry],
    selection: &Selection,
) -> Vec<SearchEntry> {
    entries
        .iter()
        .filter(|entry| {
            [FacetKind::Host, FacetKind::Tag].into_iter().all(|kind| {
                let wanted = selection.of(kind);
                wanted.is_empty()
                    || values_of(entry, kind)
                        .iter()
                        .any(|v| wanted.iter().any(|w| w == v))
            })
        })
        .cloned()
        .collect()
}

pub fn toggle(
    selection: &Selection,
    kind: FacetKind,
    value: &str,
) -> Selection {
    let mut next = selection.clone();
    let chosen = match kind {
        FacetKind::Host => &mut next.host,
        FacetKind::Tag => &mut next.tag,
    };
    if let Some(at) = chosen.iter().position(|v| v == value) {
        chosen.remove(at);
    } else {
        chosen.push(value.to_owned());
    }
    next
}

#[cfg(test)]
mod tests {
    use super::super::relevance::fixtures::entry;
    use super::*;

    fn with_tags(
        namespace: &str,
        tags: &[&str],
    ) -> SearchEntry {
        SearchEntry {
            tags: tags.iter().map(|t| (*t).to_owned()).collect(),
            ..entry(namespace, namespace)
        }
    }

    fn entries() -> Vec<SearchEntry> {
        vec![
            with_tags("github.com/a/one", &["server", "java"]),
            with_tags("github.com/b/two", &["server", "proxy"]),
            with_tags("github.com/c/three", &["server"]),
            with_tags("codeberg.org/d/four", &["proxy"]),
        ]
    }

    fn facet(
        value: &str,
        count: usize,
    ) -> Facet {
        Facet {
            value: value.into(),
            count,
        }
    }

    #[test]
    fn hosts_are_counted_off_the_results() {
        assert_eq!(
            facets_for(&entries(), FacetKind::Host, 6),
            [facet("github.com", 3), facet("codeberg.org", 1)]
        );
    }

    #[test]
    fn a_tag_all_but_one_result_carries_is_the_query_restated() {
        // `server` is on 3 of 4: selecting it would remove a single card while looking like a filter.
        let values: Vec<_> = facets_for(&entries(), FacetKind::Tag, 8)
            .into_iter()
            .map(|f| f.value)
            .collect();
        assert_eq!(values, ["proxy", "java"]);
    }

    #[test]
    fn facets_order_by_count_then_alphabetically_and_respect_take() {
        assert_eq!(
            facets_for(&entries(), FacetKind::Tag, 8),
            [facet("proxy", 2), facet("java", 1)]
        );
        assert_eq!(facets_for(&entries(), FacetKind::Tag, 1).len(), 1);
    }

    #[test]
    fn nothing_selected_keeps_everything() {
        assert_eq!(apply_selection(&entries(), &Selection::default()).len(), 4);
    }

    #[test]
    fn inside_one_facet_widens() {
        let selection = Selection {
            tag: vec!["java".into(), "proxy".into()],
            ..Selection::default()
        };
        let hosts: Vec<_> = apply_selection(&entries(), &selection)
            .iter()
            .map(|e| host_of(e).to_owned())
            .collect();
        assert_eq!(hosts, ["github.com", "github.com", "codeberg.org"]);
    }

    #[test]
    fn across_facets_narrows_and_can_narrow_to_nothing() {
        let selection = Selection {
            host: vec!["github.com".into()],
            tag: vec!["proxy".into()],
        };
        let found = apply_selection(&entries(), &selection);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].namespace, "github.com/b/two");
        let nothing = Selection {
            host: vec!["codeberg.org".into()],
            tag: vec!["java".into()],
        };
        assert!(apply_selection(&entries(), &nothing).is_empty());
    }

    #[test]
    fn toggle_adds_then_removes_leaving_the_other_facet_alone() {
        let on = toggle(&Selection::default(), FacetKind::Host, "github.com");
        assert_eq!(on.host, ["github.com"]);
        assert!(on.tag.is_empty());
        assert!(is_narrowed(&on));
        let off = toggle(&on, FacetKind::Host, "github.com");
        assert!(off.host.is_empty());
        assert!(!is_narrowed(&off));
    }
}

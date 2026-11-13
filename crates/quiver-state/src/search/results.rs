//! The results screen model: what to show, in which shelf, and what to say when there is nothing.
//! The sort and narrowing are view-level (they belong to the mount, and a new query is a new answer
//! whose facets are not the old one's).

use super::narrow::{apply_selection, Selection};
use super::relevance::relevant_local;
use super::sort::{sort_entries, SortKey};
use crate::domain::{is_held, is_quiver_own_component, DiscoverySummary, SearchEntry};
use crate::i18n::{en::design, en::search, Key, Params, Translator};
use crate::store::SearchPhase;

pub struct ResultsInput<'a> {
    pub query: &'a str,
    pub phase: SearchPhase,
    /// Lane A, ranked.
    pub local: &'a [SearchEntry],
    /// Lane B, arrival order.
    pub streamed: &'a [SearchEntry],
    pub summary: Option<&'a DiscoverySummary>,
    pub local_error: bool,
    pub pass_failed: bool,
    pub show_self_components: bool,
    pub selection: &'a Selection,
    pub sort: SortKey,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EmptyState {
    /// Core is not reachable.
    Unreachable,
    /// Nothing has been asked for.
    Idle,
    /// The pass ended with refusals: says what happened without claiming every host answered.
    WithRefusal,
    EmptyEverywhere,
}

/// "Searched 2 sources · 1 unavailable" (plain words; the old "2 hosts · 1 refused").
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceLine {
    pub sources: usize,
    pub unavailable: usize,
}

impl SourceLine {
    pub fn text(
        &self,
        tr: &Translator,
    ) -> String {
        match self.unavailable_part(tr) {
            None => self.sources_part(tr),
            Some(unavailable) => format!("{} · {unavailable}", self.sources_part(tr)),
        }
    }

    /// "Searched 2 sources": the pill's emphasised part (the old `font-medium text-foreground` span).
    pub fn sources_part(
        &self,
        tr: &Translator,
    ) -> String {
        tr.t_count(design::SEARCH_SOURCES, self.sources)
    }

    /// "1 unavailable", only when a source refused.
    pub fn unavailable_part(
        &self,
        tr: &Translator,
    ) -> Option<String> {
        (self.unavailable > 0)
            .then(|| tr.t_with(design::SEARCH_UNAVAILABLE, &Params::count(self.unavailable)))
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ResultsModel {
    /// Shown count in the header ("2 results").
    pub count: usize,
    /// Shelf "In your vault": held arrows, ranked rows first.
    pub vault: Vec<SearchEntry>,
    /// Shelf "From the network".
    pub network: Vec<SearchEntry>,
    /// Network shelf still filling: shows skeleton tiles and "N so far".
    pub discovering: bool,
    /// The size of the whole answer before narrowing. The column cap is measured against this so
    /// selecting a facet never resizes every tile under the cursor.
    pub total: usize,
    pub narrowed_to_nothing: bool,
    /// The header is hidden for a blank query.
    pub show_header: bool,
    pub searching: bool,
    pub pass_failed: bool,
    /// Reordering a list that is still arriving would move cards out from under the cursor.
    pub sortable: bool,
    pub source_line: Option<SourceLine>,
    pub empty: Option<EmptyState>,
    /// The facet row is built from this: the whole answer, not the narrowed one (a facet that
    /// disappears when selected cannot be unselected).
    pub answer: Vec<SearchEntry>,
}

pub fn build_results(input: &ResultsInput<'_>) -> ResultsModel {
    // The local lane is narrowed to the query; a streamed result is a network hit and stays whole. One the
    // local lane also holds shows only when that local copy was narrowed away, so an arrow is never on
    // screen twice or lost.
    let relevant = relevant_local(input.local, input.query);
    let shown_namespaces: Vec<&str> = relevant.iter().map(|e| e.namespace.as_str()).collect();
    let visible =
        |e: &SearchEntry| input.show_self_components || !is_quiver_own_component(&e.namespace);
    let answer: Vec<SearchEntry> = relevant
        .iter()
        .cloned()
        .chain(
            input
                .streamed
                .iter()
                .filter(|e| !shown_namespaces.contains(&e.namespace.as_str()))
                .cloned(),
        )
        .filter(visible)
        .collect();
    let shown = sort_entries(&apply_selection(&answer, input.selection), input.sort);

    // Keyed on the entry, never on the lane: a discovered arrow the catalog already holds streams with
    // `installed: true`, so the streamed band is not "things you lack".
    let (vault, network): (Vec<_>, Vec<_>) = shown.iter().cloned().partition(is_held);
    let refused = input
        .summary
        .map_or(0, |s| s.providers.iter().filter(|p| !p.ok).count());
    let discovering = input.phase == SearchPhase::Discovering;

    ResultsModel {
        count: shown.len(),
        vault,
        network,
        discovering,
        total: answer.len(),
        narrowed_to_nothing: shown.is_empty() && !answer.is_empty(),
        show_header: !input.query.is_empty(),
        searching: discovering,
        pass_failed: input.pass_failed,
        sortable: matches!(input.phase, SearchPhase::Settled | SearchPhase::Local),
        source_line: input.summary.map(|s| SourceLine {
            sources: s.providers.len(),
            unavailable: refused,
        }),
        empty: empty_state(input, !answer.is_empty(), refused),
        answer,
    }
}

fn empty_state(
    input: &ResultsInput<'_>,
    has_results: bool,
    refused: usize,
) -> Option<EmptyState> {
    if has_results {
        return None;
    }
    if input.local_error {
        return Some(EmptyState::Unreachable);
    }
    // Nothing asked for: gated on the query rather than on `Idle`, because `setQuery` empties the store
    // before Lane A answers and a real query passes through `Idle` on its way in.
    if input.query.trim().is_empty() {
        return Some(EmptyState::Idle);
    }
    // The local window: Lane A answered but no host has been asked yet; "every host answered" would
    // claim a network pass that has not started.
    if matches!(
        input.phase,
        SearchPhase::Idle | SearchPhase::Local | SearchPhase::Discovering
    ) {
        return None;
    }
    // A timeout is a claim the network never got to make (spec 10.2); the header already states the failure.
    if input.pass_failed {
        return None;
    }
    if refused > 0 {
        Some(EmptyState::WithRefusal)
    } else {
        Some(EmptyState::EmptyEverywhere)
    }
}

impl EmptyState {
    pub fn text(&self) -> Key {
        match self {
            Self::Unreachable => search::RESULTS_UNREACHABLE,
            Self::Idle => search::RESULTS_IDLE,
            Self::WithRefusal => search::RESULTS_EMPTY_WITH_REFUSAL,
            Self::EmptyEverywhere => search::RESULTS_EMPTY_EVERYWHERE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::relevance::fixtures::entry;
    use super::*;
    use crate::domain::DiscoveryProvider;

    fn held(
        namespace: &str,
        name: &str,
    ) -> SearchEntry {
        SearchEntry {
            installed: true,
            ..entry(namespace, name)
        }
    }

    fn net(
        namespace: &str,
        name: &str,
    ) -> SearchEntry {
        SearchEntry {
            provenance: None,
            ..entry(namespace, name)
        }
    }

    fn summary(ok: &[bool]) -> DiscoverySummary {
        DiscoverySummary {
            job_id: "j".into(),
            query: "q".into(),
            found: 0,
            verified: 0,
            skipped: 0,
            providers: ok
                .iter()
                .map(|ok| DiscoveryProvider {
                    host: "h".into(),
                    ok: *ok,
                    returned: 0,
                    reason: None,
                    retry_after: None,
                })
                .collect(),
        }
    }

    fn input<'a>(
        local: &'a [SearchEntry],
        streamed: &'a [SearchEntry],
        selection: &'a Selection,
    ) -> ResultsInput<'a> {
        ResultsInput {
            query: "rust",
            phase: SearchPhase::Settled,
            local,
            streamed,
            summary: None,
            local_error: false,
            pass_failed: false,
            show_self_components: true,
            selection,
            sort: SortKey::Relevance,
        }
    }

    #[test]
    fn held_arrows_go_to_the_vault_shelf_whichever_lane_they_came_from() {
        let local = [held("github.com/a/rust-one", "rust one")];
        let streamed = [
            held("github.com/b/rust-two", "rust two"),
            net("github.com/c/rust-three", "rust three"),
        ];
        let none = Selection::default();
        let model = build_results(&input(&local, &streamed, &none));
        assert_eq!(model.vault.len(), 2);
        assert_eq!(model.network.len(), 1);
        assert_eq!(model.count, 3);
    }

    #[test]
    fn the_local_lane_is_narrowed_to_the_query_but_the_network_stays_whole() {
        let local = [
            held("github.com/a/rust-one", "rust one"),
            held("github.com/a/unrelated", "unrelated"),
        ];
        let streamed = [net("github.com/c/other", "other")];
        let none = Selection::default();
        let model = build_results(&input(&local, &streamed, &none));
        assert_eq!(model.count, 2);
        assert_eq!(model.vault[0].name, "rust one");
        assert_eq!(model.network[0].name, "other");
    }

    #[test]
    fn an_arrow_is_never_on_screen_twice() {
        let a = held("github.com/a/rust-one", "rust one");
        let local = [a.clone()];
        let streamed = [a];
        let none = Selection::default();
        assert_eq!(build_results(&input(&local, &streamed, &none)).count, 1);
    }

    #[test]
    fn a_streamed_copy_shows_when_the_local_one_was_narrowed_away() {
        let local = [held("github.com/a/unrelated", "unrelated")];
        let streamed = [held("github.com/a/unrelated", "unrelated")];
        let none = Selection::default();
        assert_eq!(build_results(&input(&local, &streamed, &none)).count, 1);
    }

    #[test]
    fn quivers_own_components_hide_when_asked() {
        let streamed = [
            net("github.com/rabbytesoftware/quiver.core", "rust core"),
            net("github.com/x/rust", "rust"),
        ];
        let none = Selection::default();
        let mut i = input(&[], &streamed, &none);
        assert_eq!(build_results(&i).count, 2);
        i.show_self_components = false;
        assert_eq!(build_results(&i).count, 1);
    }

    #[test]
    fn narrowing_to_nothing_is_said_not_silently_ignored_and_total_ignores_the_selection() {
        let streamed = [net("github.com/x/rust", "rust")];
        let selection = Selection {
            host: vec!["codeberg.org".into()],
            tag: vec![],
        };
        let model = build_results(&input(&[], &streamed, &selection));
        assert!(model.narrowed_to_nothing);
        assert_eq!((model.count, model.total), (0, 1));
        assert!(model.empty.is_none());
    }

    #[test]
    fn the_header_hides_for_a_blank_query_and_the_idle_line_shows() {
        let none = Selection::default();
        let mut i = input(&[], &[], &none);
        i.query = "";
        let model = build_results(&i);
        assert!(!model.show_header);
        assert_eq!(model.empty, Some(EmptyState::Idle));
    }

    #[test]
    fn empty_states_follow_the_phase_and_the_refusals() {
        let none = Selection::default();
        let mut i = input(&[], &[], &none);
        i.local_error = true;
        assert_eq!(build_results(&i).empty, Some(EmptyState::Unreachable));
        i.local_error = false;
        for early in [
            SearchPhase::Idle,
            SearchPhase::Local,
            SearchPhase::Discovering,
        ] {
            i.phase = early;
            assert_eq!(build_results(&i).empty, None, "{early:?}");
        }
        i.phase = SearchPhase::Settled;
        assert_eq!(build_results(&i).empty, Some(EmptyState::EmptyEverywhere));
        let refused = summary(&[true, false]);
        i.summary = Some(&refused);
        assert_eq!(build_results(&i).empty, Some(EmptyState::WithRefusal));
        i.pass_failed = true;
        assert_eq!(build_results(&i).empty, None);
    }

    #[test]
    fn sorting_waits_for_the_answer_to_stop_moving() {
        let none = Selection::default();
        let mut i = input(&[], &[], &none);
        for (phase, sortable) in [
            (SearchPhase::Settled, true),
            (SearchPhase::Local, true),
            (SearchPhase::Discovering, false),
            (SearchPhase::Settling, false),
        ] {
            i.phase = phase;
            assert_eq!(build_results(&i).sortable, sortable, "{phase:?}");
        }
        i.phase = SearchPhase::Discovering;
        let model = build_results(&i);
        assert!(model.discovering && model.searching);
    }

    #[test]
    fn the_source_line_says_it_in_plain_words() {
        let tr = Translator::default();
        assert_eq!(
            SourceLine {
                sources: 2,
                unavailable: 1
            }
            .text(&tr),
            "Searched 2 sources · 1 unavailable"
        );
        assert_eq!(
            SourceLine {
                sources: 1,
                unavailable: 0
            }
            .text(&tr),
            "Searched 1 source"
        );
        let s = summary(&[true, true, false]);
        let none = Selection::default();
        let mut i = input(&[], &[], &none);
        i.summary = Some(&s);
        assert_eq!(
            build_results(&i).source_line,
            Some(SourceLine {
                sources: 3,
                unavailable: 1
            })
        );
    }
}

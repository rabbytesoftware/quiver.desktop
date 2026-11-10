use crate::domain::{DiscoverySummary, SearchEntry};
use crate::observable::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchPhase {
    Idle,
    Local,
    Discovering,
    Settling,
    Settled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchJob {
    pub id: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchState {
    pub query: String,
    pub phase: SearchPhase,
    /// Lane A, ranked.
    pub local: Vec<SearchEntry>,
    /// Lane B, arrival order -- these have no score to rank by.
    pub streamed: Vec<SearchEntry>,
    pub job: Option<SearchJob>,
    /// Held in memory: the job 404s 30s after the pass ends.
    pub summary: Option<DiscoverySummary>,
    pub local_error: bool,
    pub pass_failed: bool,
    /// The query Enter was pressed on. The search field and the mounted pass controller are
    /// siblings, not parent/child, so this is how Enter reaches the controller (spec 2.2). It
    /// carries the exact committed value rather than relying on the URL, which lags behind an
    /// in-flight navigation.
    pub submit_query: Option<String>,
    /// The query the field reopened rather than asked for. Reopening restores a screen; it is not
    /// a new request, so Lane B stays put and Lane A rebuilds from the vault the last pass already
    /// filled. Carries the value, not a flag, so a query typed between the request and the mount is
    /// still treated as the new ask it is.
    pub restore_query: Option<String>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            phase: SearchPhase::Idle,
            local: vec![],
            streamed: vec![],
            job: None,
            summary: None,
            local_error: false,
            pass_failed: false,
            submit_query: None,
            restore_query: None,
        }
    }
}

impl SearchState {
    /// Everything but the query and the two messages between components.
    fn clear_results(&mut self) {
        self.phase = SearchPhase::Idle;
        self.local.clear();
        self.streamed.clear();
        self.job = None;
        self.summary = None;
        self.local_error = false;
        self.pass_failed = false;
    }
}

/// Lane A resolves the platform before it asks, so a slow answer can land after the pass has
/// begun -- it must fill the band, not rewind the phase.
fn phase_after_local(phase: SearchPhase) -> SearchPhase {
    match phase {
        SearchPhase::Discovering | SearchPhase::Settling | SearchPhase::Settled => phase,
        _ => SearchPhase::Local,
    }
}

#[derive(Clone, Default)]
pub struct SearchStore {
    store: Store<SearchState>,
}

impl SearchStore {
    pub fn store(&self) -> &Store<SearchState> {
        &self.store
    }

    pub fn snapshot(&self) -> SearchState {
        self.store.snapshot()
    }

    pub fn phase(&self) -> SearchPhase {
        self.store.read(|s| s.phase)
    }

    pub fn set_query(
        &self,
        query: &str,
    ) {
        self.store.update(|s| {
            s.query = query.to_string();
            s.clear_results();
        });
    }

    pub fn set_local(
        &self,
        local: Vec<SearchEntry>,
    ) {
        self.store.update(|s| {
            s.local = local;
            s.phase = phase_after_local(s.phase);
            s.local_error = false;
        });
    }

    pub fn set_local_error(&self) {
        self.store.update(|s| {
            s.local_error = true;
            s.phase = phase_after_local(s.phase);
        });
    }

    pub fn begin_pass(
        &self,
        job: SearchJob,
    ) {
        self.store.update(|s| {
            s.job = Some(job);
            s.phase = SearchPhase::Discovering;
            s.pass_failed = false;
        });
    }

    /// Dedup on the bare namespace within the streamed band only. A namespace `local` also holds is
    /// kept: whether the local copy is on screen is the screen's call (it filters that lane to the
    /// query), and dropping the streamed copy here would lose the arrow when it is not.
    pub fn receive(
        &self,
        entry: SearchEntry,
    ) {
        self.store.update(|s| {
            if s.phase != SearchPhase::Discovering
                || s.streamed.iter().any(|e| e.namespace == entry.namespace)
            {
                return;
            }
            s.streamed.push(entry);
        });
    }

    pub fn end_pass(
        &self,
        summary: DiscoverySummary,
    ) {
        self.store.update(|s| {
            s.summary = Some(summary);
            s.phase = SearchPhase::Settling;
        });
    }

    /// Local is a replacement; `streamed` is left whole (arrival order). Some probed results are
    /// deliberately never indexed, so the re-query alone would erase them, and one it does return
    /// may be filtered out of the local band by the screen -- which then still needs the streamed copy.
    pub fn settle(
        &self,
        local: Vec<SearchEntry>,
    ) {
        self.store.update(|s| {
            s.local = local;
            s.phase = SearchPhase::Settled;
        });
    }

    /// Holds the phase and keeps both bands: clearing them would delete results the user can see,
    /// to recover from a failed re-query.
    pub fn settle_failed(&self) {
        self.store.update(|s| {
            s.phase = SearchPhase::Settling;
            s.pass_failed = true;
        });
    }

    pub fn request_submit(
        &self,
        query: &str,
    ) {
        self.store
            .update(|s| s.submit_query = Some(query.to_string()));
    }

    pub fn clear_submit(&self) {
        self.store.update(|s| s.submit_query = None);
    }

    pub fn request_restore(
        &self,
        query: &str,
    ) {
        self.store
            .update(|s| s.restore_query = Some(query.to_string()));
    }

    pub fn clear_restore(&self) {
        self.store.update(|s| s.restore_query = None);
    }

    /// The screen went away, so its results go with it: a query and the results attached to it
    /// belong to one mount, not to the app. Deliberately not `reset` -- `submit_query` is a message
    /// from the always-mounted field, and dropping it here would swallow an Enter that navigated to
    /// a fresh screen.
    pub fn clear(&self) {
        self.store.update(|s| {
            s.query.clear();
            s.clear_results();
        });
    }

    pub fn reset(&self) {
        self.store.set(SearchState::default());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn entry(namespace: &str) -> SearchEntry {
        SearchEntry {
            namespace: namespace.into(),
            name: namespace.rsplit('/').next().unwrap_or(namespace).into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            versions: vec!["v1.0.0".into()],
            compatible_os: vec![],
            provenance: Some(crate::domain::SearchProvenance::Seen),
            installed: false,
            known: false,
            stars: 0,
            source: Some("github.com".into()),
            origin: None,
            confidence: None,
        }
    }

    fn held(namespace: &str) -> SearchEntry {
        SearchEntry {
            installed: true,
            known: true,
            ..entry(namespace)
        }
    }

    fn job() -> SearchJob {
        SearchJob {
            id: "job-1".into(),
            expires_at: "2026-08-18T00:00:30.000Z".into(),
        }
    }

    fn summary(
        found: u32,
        verified: u32,
    ) -> DiscoverySummary {
        DiscoverySummary {
            job_id: "job-1".into(),
            query: "q".into(),
            found,
            verified,
            skipped: 0,
            providers: vec![],
        }
    }

    fn streamed(store: &SearchStore) -> Vec<String> {
        store
            .snapshot()
            .streamed
            .into_iter()
            .map(|e| e.namespace)
            .collect()
    }

    #[test]
    fn starts_idle_with_nothing_in_either_band() {
        let s = SearchStore::default().snapshot();
        assert_eq!(s.phase, SearchPhase::Idle);
        assert!(s.local.is_empty() && s.streamed.is_empty());
    }

    #[test]
    fn moves_to_local_when_lane_a_answers_even_with_no_hits() {
        let s = SearchStore::default();
        s.set_local(vec![]);
        assert_eq!(s.phase(), SearchPhase::Local);
    }

    #[test]
    fn moves_through_discovering_and_settling_to_settled() {
        let s = SearchStore::default();
        s.set_local(vec![held("github.com/a/one")]);
        s.begin_pass(job());
        assert_eq!(s.phase(), SearchPhase::Discovering);
        s.end_pass(summary(0, 0));
        assert_eq!(s.phase(), SearchPhase::Settling);
        s.settle(vec![held("github.com/a/one")]);
        assert_eq!(s.phase(), SearchPhase::Settled);
    }

    #[test]
    fn receive_appends_in_arrival_order_dedups_and_leaves_local_untouched() {
        let s = SearchStore::default();
        s.set_local(vec![SearchEntry {
            provenance: Some(crate::domain::SearchProvenance::Installed),
            ..held("github.com/a/one")
        }]);
        s.begin_pass(job());
        s.receive(entry("github.com/a/one"));
        s.receive(entry("github.com/a/two"));
        s.receive(entry("github.com/a/two"));
        assert_eq!(streamed(&s), vec!["github.com/a/one", "github.com/a/two"]);
        assert_eq!(
            s.snapshot().local[0].provenance,
            Some(crate::domain::SearchProvenance::Installed)
        );
    }

    #[test]
    fn ignores_results_that_arrive_when_no_pass_is_running() {
        let s = SearchStore::default();
        s.set_local(vec![]);
        s.receive(entry("github.com/a/one"));
        assert!(streamed(&s).is_empty());
    }

    #[test]
    fn settle_keeps_the_streamed_band_whole_whether_or_not_the_requery_returns_it() {
        let s = SearchStore::default();
        s.set_local(vec![held("github.com/a/base")]);
        s.begin_pass(job());
        s.receive(entry("github.com/a/fletcher-1"));
        s.receive(entry("github.com/a/fletcher-2"));
        s.end_pass(summary(2, 0));
        s.settle(vec![
            held("github.com/a/base"),
            entry("github.com/a/fletcher-1"),
        ]);
        assert_eq!(
            streamed(&s),
            vec!["github.com/a/fletcher-1", "github.com/a/fletcher-2"]
        );
        assert_eq!(s.snapshot().local.len(), 2);
        assert_eq!(s.phase(), SearchPhase::Settled);
    }

    #[test]
    fn settle_failed_holds_the_phase_and_keeps_both_bands() {
        let s = SearchStore::default();
        s.set_local(vec![held("github.com/a/one")]);
        s.begin_pass(job());
        s.receive(entry("github.com/a/two"));
        s.end_pass(summary(1, 1));
        s.settle_failed();
        let snap = s.snapshot();
        assert_eq!(
            (snap.phase, snap.streamed.len(), snap.local.len()),
            (SearchPhase::Settling, 1, 1)
        );
        assert!(snap.pass_failed);
    }

    #[test]
    fn pass_failed_is_cleared_by_begin_pass_so_a_retry_starts_clean() {
        let s = SearchStore::default();
        s.set_local(vec![]);
        s.begin_pass(job());
        s.settle_failed();
        assert!(s.snapshot().pass_failed);
        s.begin_pass(job());
        assert!(!s.snapshot().pass_failed);
    }

    #[test]
    fn the_summary_outlives_the_job() {
        let s = SearchStore::default();
        s.set_local(vec![]);
        s.begin_pass(job());
        s.end_pass(summary(7, 5));
        s.settle(vec![]);
        assert_eq!(s.snapshot().summary.map(|x| x.verified), Some(5));
    }

    #[test]
    fn set_query_clears_both_bands_and_the_job_and_empty_goes_back_to_idle() {
        let s = SearchStore::default();
        s.set_local(vec![entry("github.com/a/one")]);
        s.begin_pass(job());
        s.receive(entry("github.com/a/two"));
        s.set_query("other");
        let snap = s.snapshot();
        assert!(
            snap.local.is_empty()
                && snap.streamed.is_empty()
                && snap.job.is_none()
                && snap.summary.is_none()
        );
        assert_eq!(snap.query, "other");
        s.set_local(vec![]);
        s.set_query("");
        assert_eq!(s.phase(), SearchPhase::Idle);
    }

    #[test]
    fn a_late_lane_a_answer_or_failure_keeps_the_pass_phase() {
        for phase in [
            SearchPhase::Discovering,
            SearchPhase::Settling,
            SearchPhase::Settled,
        ] {
            let s = SearchStore::default();
            s.store().update(|st| st.phase = phase);
            s.set_local(vec![]);
            assert_eq!(s.phase(), phase);
            s.set_local_error();
            assert_eq!(s.phase(), phase);
        }
    }

    #[test]
    fn clear_keeps_the_submit_message_and_reset_drops_everything() {
        let s = SearchStore::default();
        s.set_query("q");
        s.request_submit("q");
        s.request_restore("q");
        s.clear();
        let snap = s.snapshot();
        assert_eq!(
            (snap.query.as_str(), snap.submit_query.as_deref()),
            ("", Some("q"))
        );
        s.reset();
        let snap = s.snapshot();
        assert!(snap.submit_query.is_none() && snap.restore_query.is_none());
        s.request_submit("x");
        s.clear_submit();
        s.request_restore("x");
        s.clear_restore();
        let snap = s.snapshot();
        assert!(snap.submit_query.is_none() && snap.restore_query.is_none());
    }
}

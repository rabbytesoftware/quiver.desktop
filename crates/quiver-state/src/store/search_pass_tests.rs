use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Notify;

use super::search::{SearchPhase, SearchStore};
use super::search_pass::{PlatformSource, SearchController, IDLE_BEFORE_PASS, PASS_DEADLINE};
use crate::core_api::{FakeCoreApi, FakeStreamHandle, Method};

struct Linux;

#[async_trait]
impl PlatformSource for Linux {
    async fn platform(&self) -> String {
        "linux/amd64".into()
    }
}

struct Rig {
    api: FakeCoreApi,
    store: SearchStore,
    controller: SearchController,
}

const LANE_A: &str = "/v0/search?q=server&limit=100&os=linux%2Famd64";
const DISCOVER: &str = "/v0/search/discover";
const JOB: &str = "/v0/search/discover/job-1";
/// The stream carries the same `os` filter as both Lane A calls.
const STREAM: &str = "/v0/search/discover/job-1?os=linux%2Famd64";

fn result(namespace: &str) -> serde_json::Value {
    json!({"namespace": namespace, "name": "n", "description": "", "tags": [], "installed": false, "known": true})
}

fn rig() -> Rig {
    let api = FakeCoreApi::new();
    api.route(Method::Get, LANE_A, json!([result("github.com/a/local")]));
    api.route(
        Method::Post,
        DISCOVER,
        json!({"job_id": "job-1", "query": "server", "expires_at": "2026-08-18T00:00:30Z"}),
    );
    api.route(
        Method::Get,
        JOB,
        json!({"job_id": "job-1", "status": "completed", "query": "server", "found": 2, "verified": 1, "skipped": 0, "providers": []}),
    );
    let store = SearchStore::default();
    let controller = SearchController::new(Arc::new(api.clone()), store.clone(), Arc::new(Linux));
    Rig {
        api,
        store,
        controller,
    }
}

impl Rig {
    async fn wait_for(&self, what: impl Fn(&super::search::SearchState) -> bool) {
        let mut rx = self.store.store().subscribe();
        loop {
            if self.store.store().read(&what) {
                return;
            }
            rx.changed().await.unwrap();
        }
    }

    async fn wait_phase(&self, phase: SearchPhase) {
        self.wait_for(|s| s.phase == phase).await;
    }

    /// Commits "server" and waits until the pass's stream is open.
    async fn start_pass(&self) -> FakeStreamHandle {
        self.controller.set_query("server", true);
        self.wait_phase(SearchPhase::Discovering).await;
        self.api.wait_stream(STREAM, 0).await
    }
}

#[tokio::test(start_paused = true)]
async fn lane_a_answers_without_waiting_for_the_idle_timer() {
    let rig = rig();
    rig.controller.set_query("server", true);
    rig.wait_phase(SearchPhase::Local).await;
    assert_eq!(rig.store.snapshot().local.len(), 1);
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 0);
}

#[tokio::test(start_paused = true)]
async fn blank_and_whitespace_queries_never_reach_the_network() {
    let rig = rig();
    rig.controller.set_query("", true);
    rig.controller.set_query("   ", true);
    rig.controller.submit();
    tokio::time::sleep(IDLE_BEFORE_PASS * 3).await;
    assert!(rig.api.calls().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_pass_starts_after_the_idle_delay_and_not_before() {
    let rig = rig();
    rig.controller.set_query("server", true);
    rig.wait_phase(SearchPhase::Local).await;
    tokio::time::sleep(IDLE_BEFORE_PASS - Duration::from_millis(1)).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 0);
    rig.wait_phase(SearchPhase::Discovering).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 1);
}

#[tokio::test(start_paused = true)]
async fn retyping_re_arms_the_timer_so_a_pass_never_fires_mid_typing() {
    let rig = rig();
    rig.api.route(
        Method::Get,
        "/v0/search?q=serve&limit=100&os=linux%2Famd64",
        json!([]),
    );
    rig.controller.set_query("serve", true);
    tokio::time::sleep(IDLE_BEFORE_PASS - Duration::from_millis(100)).await;
    rig.controller.set_query("server", true);
    tokio::time::sleep(IDLE_BEFORE_PASS - Duration::from_millis(100)).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 0);
    rig.wait_phase(SearchPhase::Discovering).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 1);
}

#[tokio::test(start_paused = true)]
async fn submit_fires_immediately_and_clears_the_timer() {
    let rig = rig();
    rig.controller.set_query("server", true);
    rig.controller.submit();
    rig.wait_phase(SearchPhase::Discovering).await;
    tokio::time::sleep(IDLE_BEFORE_PASS * 2).await;
    assert_eq!(
        rig.api.calls_to(Method::Post, DISCOVER),
        1,
        "the idle timer was cleared by submit"
    );
}

#[tokio::test(start_paused = true)]
async fn streams_results_then_settles_on_the_close_frame_reading_the_summary_once() {
    let rig = rig();
    let stream = rig.start_pass().await;
    stream.send_json(result("github.com/a/probed"));
    rig.wait_for(|s| s.streamed.len() == 1).await;
    assert_eq!(
        rig.api.calls_to(Method::Get, JOB),
        0,
        "the job is never polled while the stream is open"
    );
    stream.close(Some((1000, "completed")));
    rig.wait_phase(SearchPhase::Settled).await;
    let snap = rig.store.snapshot();
    assert_eq!(
        snap.summary.map(|s| s.verified),
        Some(1),
        "the summary survives the settle"
    );
    assert_eq!(snap.streamed.len(), 1);
    assert_eq!(rig.api.calls_to(Method::Get, JOB), 1);
    assert_eq!(
        rig.api.calls_to(Method::Get, LANE_A),
        2,
        "initial Lane A plus the re-query"
    );
}

#[tokio::test(start_paused = true)]
async fn the_stream_is_opened_with_the_same_platform_as_lane_a() {
    let rig = rig();
    rig.start_pass().await;
    assert_eq!(rig.api.open_count(STREAM), 1);
}

#[tokio::test(start_paused = true)]
async fn still_settles_when_the_summary_cannot_be_read_after_a_clean_close() {
    let rig = rig();
    rig.api.route_error(Method::Get, JOB, 500, "gone");
    let stream = rig.start_pass().await;
    stream.close(Some((1000, "completed")));
    rig.wait_phase(SearchPhase::Settled).await;
    assert!(rig.store.snapshot().summary.is_none());
}

#[tokio::test(start_paused = true)]
async fn reconnects_once_on_an_abnormal_close_without_duplicating_replayed_namespaces() {
    let rig = rig();
    let first = rig.start_pass().await;
    first.send_json(result("github.com/a/b"));
    rig.wait_for(|s| s.streamed.len() == 1).await;
    first.close(Some((1013, "slow consumer")));
    let second = rig.api.wait_stream(STREAM, 1).await;
    assert_eq!(rig.store.phase(), SearchPhase::Discovering);
    second.send_json(result("github.com/a/b"));
    second.close(Some((1000, "completed")));
    rig.wait_phase(SearchPhase::Settled).await;
    assert_eq!(rig.store.snapshot().streamed.len(), 1);
    assert!(!rig.store.snapshot().pass_failed);
}

#[tokio::test(start_paused = true)]
async fn fails_the_pass_when_the_second_stream_breaks_too_keeping_what_it_has() {
    let rig = rig();
    let first = rig.start_pass().await;
    first.send_json(result("github.com/a/b"));
    first.close(None);
    let second = rig.api.wait_stream(STREAM, 1).await;
    second.close(Some((1006, "")));
    rig.wait_for(|s| s.pass_failed).await;
    let snap = rig.store.snapshot();
    assert_eq!(
        (snap.phase, snap.streamed.len()),
        (SearchPhase::Settling, 1)
    );
    assert_eq!(rig.api.open_count(STREAM), 2);
}

#[tokio::test(start_paused = true)]
async fn gives_up_after_the_safety_deadline_if_a_reconnected_stream_never_closes() {
    let rig = rig();
    let first = rig.start_pass().await;
    first.close(Some((1013, "slow consumer")));
    rig.api.wait_stream(STREAM, 1).await;
    let before = tokio::time::Instant::now();
    rig.wait_for(|s| s.pass_failed).await;
    assert!(before.elapsed() >= PASS_DEADLINE - Duration::from_millis(1));
    assert_eq!(rig.store.phase(), SearchPhase::Settling);
}

#[tokio::test(start_paused = true)]
async fn a_pass_that_completed_inside_the_safety_window_is_not_failed() {
    let rig = rig();
    let first = rig.start_pass().await;
    first.close(Some((1013, "slow consumer")));
    let second = rig.api.wait_stream(STREAM, 1).await;
    second.close(Some((1000, "completed")));
    rig.wait_phase(SearchPhase::Settled).await;
    tokio::time::sleep(PASS_DEADLINE * 2).await;
    assert_eq!(rig.store.phase(), SearchPhase::Settled);
    assert!(!rig.store.snapshot().pass_failed);
}

#[tokio::test(start_paused = true)]
async fn a_new_query_cancels_the_in_flight_pass_and_clears_its_results() {
    let rig = rig();
    let stream = rig.start_pass().await;
    rig.api.route(
        Method::Get,
        "/v0/search?q=other&limit=100&os=linux%2Famd64",
        json!([]),
    );
    rig.controller.set_query("other", false);
    rig.wait_phase(SearchPhase::Local).await;
    assert!(stream.is_dropped() || rig.api.open_count(STREAM) == 1);
    stream.send_json(result("github.com/a/late"));
    tokio::time::sleep(IDLE_BEFORE_PASS * 2).await;
    let snap = rig.store.snapshot();
    assert!(
        snap.streamed.is_empty(),
        "a frame from the replaced pass is dropped"
    );
    assert_eq!(
        rig.api.calls_to(Method::Post, DISCOVER),
        1,
        "restoring (discover=false) starts no new pass"
    );
}

#[tokio::test(start_paused = true)]
async fn restoring_a_query_runs_lane_a_but_never_arms_a_pass() {
    let rig = rig();
    rig.controller.set_query("server", false);
    rig.wait_phase(SearchPhase::Local).await;
    tokio::time::sleep(IDLE_BEFORE_PASS * 2).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 0);
    rig.controller.submit();
    rig.wait_phase(SearchPhase::Discovering).await;
}

#[tokio::test(start_paused = true)]
async fn dispose_leaves_nothing_behind_and_ignores_later_calls() {
    let rig = rig();
    rig.start_pass().await;
    rig.controller.dispose();
    assert_eq!(rig.store.snapshot().phase, SearchPhase::Idle);
    rig.controller.set_query("server", true);
    rig.controller.submit();
    tokio::time::sleep(IDLE_BEFORE_PASS * 3).await;
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 1);
    assert!(rig.store.snapshot().query.is_empty());
}

#[tokio::test(start_paused = true)]
async fn dispose_drops_a_lane_a_answer_that_lands_after_it() {
    let rig = rig();
    let release = Arc::new(Notify::new());
    let gate = release.clone();
    rig.api.route_fn(Method::Get, LANE_A, move |_| {
        let gate = gate.clone();
        async move {
            gate.notified().await;
            Ok(crate::core_api::ApiResult {
                status: 200,
                data: Some(json!([])),
            })
        }
    });
    rig.controller.set_query("server", false);
    tokio::task::yield_now().await;
    rig.controller.dispose();
    release.notify_waiters();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(rig.store.phase(), SearchPhase::Idle);
}

#[tokio::test(start_paused = true)]
async fn a_failed_lane_a_sets_the_error_unless_the_query_was_replaced() {
    let rig = rig();
    rig.api.route_error(Method::Get, LANE_A, 500, "boom");
    rig.controller.set_query("server", false);
    rig.wait_for(|s| s.local_error).await;
    assert_eq!(rig.store.phase(), SearchPhase::Local);
}

#[tokio::test(start_paused = true)]
async fn a_pass_whose_post_never_returned_a_ticket_does_not_begin() {
    let rig = rig();
    rig.api.route_error(Method::Post, DISCOVER, 500, "no");
    rig.controller.set_query("server", true);
    rig.wait_phase(SearchPhase::Local).await;
    tokio::time::sleep(IDLE_BEFORE_PASS * 2).await;
    assert_eq!(rig.store.phase(), SearchPhase::Local);
    assert_eq!(rig.api.open_count(STREAM), 0);
}

#[tokio::test(start_paused = true)]
async fn an_unchanged_query_is_ignored() {
    let rig = rig();
    rig.controller.set_query("server", false);
    rig.wait_phase(SearchPhase::Local).await;
    rig.controller.set_query("server", true);
    tokio::time::sleep(IDLE_BEFORE_PASS * 2).await;
    assert_eq!(rig.api.calls_to(Method::Get, LANE_A), 1);
    assert_eq!(rig.api.calls_to(Method::Post, DISCOVER), 0);
}

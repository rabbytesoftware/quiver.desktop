//! The adapters that put `quiver-state` and the arrow host on the active connection, against a fake
//! daemon on the local socket and a wiremock one standing in for a remote.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use quiver_app::services::{CoreUpstream, NativeBackend, TransportCoreApi};
use quiver_arrow_host::{ArrowRequest, ArrowUpstream};
use quiver_state::connection::Backend;
use quiver_state::core_api::{CoreApi, Method, StreamEvent};
use quiver_state::transport::api::{RetryConfig, Sleeper};
use support::{accept_everything, remote_daemon, FakeDaemon, LocalSetup, Reply};

struct RecordingSleeper(Mutex<Vec<Duration>>);

#[async_trait]
impl Sleeper for RecordingSleeper {
    async fn sleep(
        &self,
        duration: Duration,
    ) {
        self.0.lock().unwrap().push(duration);
    }
}

fn retries(attempts: u32) -> (RetryConfig, Arc<RecordingSleeper>) {
    let sleeper = Arc::new(RecordingSleeper(Mutex::default()));
    let config = RetryConfig {
        attempts,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_millis(1000),
        sleeper: sleeper.clone(),
    };
    (config, sleeper)
}

#[tokio::test]
async fn the_envelope_is_unwrapped_and_an_error_carries_its_message_and_status() {
    let setup = LocalSetup::new();
    let _daemon = FakeDaemon::bind(
        &setup.socket,
        Arc::new(|r| match r.path.as_str() {
            "/v0/ok" => Reply::Json(200, r#"{"success":true,"data":{"a":1}}"#.into()),
            _ => Reply::Json(400, r#"{"success":false,"error":"bad"}"#.into()),
        }),
    );
    let (manager, _) = setup.manager();
    let api = TransportCoreApi::new(manager);

    let ok = api.request(Method::Get, "/v0/ok", None).await.unwrap();
    assert_eq!(ok.data, Some(serde_json::json!({"a": 1})));
    let err = api
        .request(Method::Post, "/v0/nope", Some(serde_json::json!({"x": 1})))
        .await
        .unwrap_err();
    assert_eq!((err.status, err.message.as_str()), (400, "bad"));
}

#[tokio::test]
async fn an_unreachable_daemon_is_a_proxy_marked_502_so_reads_retry_and_writes_do_not() {
    let setup = LocalSetup::new(); // nothing listens on the socket
    let (manager, _) = setup.manager();
    let (config, sleeper) = retries(3);
    let api = TransportCoreApi::new(manager).with_retry(config);

    let read = api
        .request(Method::Get, "/v0/arrow", None)
        .await
        .unwrap_err();
    assert_eq!(read.status, 502);
    assert_eq!(sleeper.0.lock().unwrap().len(), 2, "a read is retried");

    sleeper.0.lock().unwrap().clear();
    let write = api
        .request(Method::Delete, "/v0/arrow/x", None)
        .await
        .unwrap_err();
    assert_eq!(write.status, 502);
    assert!(
        sleeper.0.lock().unwrap().is_empty(),
        "a write is never retried"
    );

    assert!(!api.is_reachable().await);
}

#[tokio::test(start_paused = true)]
async fn a_daemon_that_never_answers_is_a_marked_504_after_the_timeout() {
    let setup = LocalSetup::new();
    let _daemon = FakeDaemon::bind(&setup.socket, Arc::new(|_| Reply::Stall));
    let (manager, _) = setup.manager();
    let (config, _) = retries(1);
    let api = TransportCoreApi::new(manager)
        .with_retry(config)
        .with_timeout(Duration::from_secs(5));

    let err = api
        .request(Method::Get, "/v0/arrow", None)
        .await
        .unwrap_err();
    assert_eq!(err.status, 504);
}

#[tokio::test]
async fn reachability_is_the_health_status_only() {
    let setup = LocalSetup::new();
    let _daemon = FakeDaemon::bind(
        &setup.socket,
        Arc::new(|_| Reply::Json(200, "not json at all".into())),
    );
    let (manager, _) = setup.manager();
    assert!(TransportCoreApi::new(manager).is_reachable().await);
}

#[tokio::test]
async fn the_transport_is_resolved_per_call_so_a_switch_reaches_the_new_daemon() {
    let setup = LocalSetup::new();
    let daemon = FakeDaemon::bind(&setup.socket, accept_everything());
    let remote = remote_daemon().await;
    let (manager, _) = setup.manager();
    let api = TransportCoreApi::new(manager.clone());

    api.request(Method::Get, "/v0/where", None).await.unwrap();
    assert_eq!(
        daemon.paths(),
        vec![("GET".to_string(), "/v0/where".to_string())]
    );

    let paired = manager
        .add_connection("R".into(), remote.uri(), "code".into())
        .await
        .unwrap();
    manager.switch_connection(&paired.id).await.unwrap();
    api.request(Method::Get, "/v0/where", None).await.unwrap();

    let on_remote = remote.received_requests().await.unwrap();
    assert!(on_remote.iter().any(|r| r.url.path() == "/v0/where"));
    assert_eq!(
        daemon.paths().len(),
        1,
        "the local daemon saw nothing after the switch"
    );
}

#[tokio::test]
async fn a_stream_yields_text_frames_then_the_close_the_daemon_sent() {
    let setup = LocalSetup::new();
    let _daemon = FakeDaemon::bind(
        &setup.socket,
        Arc::new(|_| Reply::WebSocket(vec!["one".into(), "two".into()])),
    );
    let (manager, _) = setup.manager();
    let api = TransportCoreApi::new(manager);

    let mut stream = api.open_stream("/v0/arrow").await.unwrap();
    assert_eq!(stream.next().await, StreamEvent::Message("one".into()));
    assert_eq!(stream.next().await, StreamEvent::Message("two".into()));
    let StreamEvent::Closed(Some(info)) = stream.next().await else {
        panic!("expected the daemon's close");
    };
    assert_eq!((info.code, info.reason.as_str()), (1000, "completed"));
    assert_eq!(stream.next().await, StreamEvent::Closed(None));
}

#[tokio::test]
async fn opening_a_stream_on_a_dead_daemon_is_a_transport_failure() {
    let setup = LocalSetup::new();
    let (manager, _) = setup.manager();
    let err = TransportCoreApi::new(manager)
        .open_stream("/v0/arrow")
        .await
        .err()
        .unwrap();
    assert_eq!(err.status, 0);
}

#[tokio::test]
async fn arrow_pages_reach_the_daemon_ui_route_and_get_its_status_headers_and_body() {
    let setup = LocalSetup::new();
    let daemon = FakeDaemon::bind(
        &setup.socket,
        Arc::new(|_| Reply::Json(200, "<html></html>".into())),
    );
    let (manager, _) = setup.manager();
    let upstream = CoreUpstream::new(manager);

    let resp = upstream
        .request(ArrowRequest::get(
            "/v0/ui/github.com%2Fa%2Fb/index.html?x=1",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"<html></html>");
    assert_eq!(resp.headers.get("content-type"), Some("application/json"));
    assert_eq!(
        daemon.paths(),
        vec![(
            "GET".to_string(),
            "/v0/ui/github.com%2Fa%2Fb/index.html?x=1".to_string()
        )]
    );
}

#[tokio::test]
async fn a_dead_daemon_is_an_upstream_error_the_arrow_host_turns_into_a_502() {
    let setup = LocalSetup::new();
    let (manager, _) = setup.manager();
    let err = CoreUpstream::new(manager)
        .request(ArrowRequest::get("/v0/ui/ns/"))
        .await
        .unwrap_err();
    assert!(matches!(err, quiver_arrow_host::Error::Upstream(_)));
}

#[tokio::test]
async fn the_backend_lists_the_local_connection_forwards_events_and_refuses_what_core_refuses() {
    let setup = LocalSetup::new();
    let (manager, events) = setup.manager();
    let backend = NativeBackend::new(manager, events, "linux/amd64".into());

    let snapshot = backend.get_connections().await.unwrap();
    assert_eq!(snapshot.active_id, "local");
    assert_eq!(snapshot.connections.len(), 1);
    assert_eq!(backend.get_platform().await.unwrap(), "linux/amd64");

    let mut changes = backend.on_connections_changed();
    let err = backend.remove_connection("local").await.unwrap_err();
    assert!(err.to_string().contains("cannot remove local"), "{err}");
    let err = backend
        .rename_connection("local", "Home")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("cannot rename local"), "{err}");
    assert!(
        changes.try_recv().is_err(),
        "a refused change announces nothing"
    );

    assert!(backend.switch_connection("missing").await.is_err());
    assert!(backend
        .check_remote_health("http://127.0.0.1:1")
        .await
        .is_err());
}

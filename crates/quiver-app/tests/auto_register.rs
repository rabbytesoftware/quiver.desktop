//! The fix of docs/decision-log.md "desktop self-registers on remote connections", proven at the
//! services level against fake daemons: the desktop registers itself in the LOCAL daemon's catalog
//! and never in a remote one's.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use quiver_app::services::AppServices;
use support::{accept_everything, remote_daemon, FakeDaemon, LocalSetup, Recorded};

fn is_catalog_write(r: &Recorded) -> bool {
    r.method != "GET" && r.path.starts_with("/v0/arrow/")
}

#[tokio::test]
async fn a_remote_connection_that_becomes_ready_registers_nothing_but_the_local_one_still_does() {
    let setup = LocalSetup::new();
    let mut local = FakeDaemon::bind(&setup.socket, accept_everything());
    let remote = remote_daemon().await;

    // The local daemon is "already running", so the app adopts it and announces to it.
    let services = AppServices::build(setup.config()).await;
    let registration = local.wait_for(is_catalog_write).await;
    assert_eq!(registration.method, "POST");
    assert!(
        registration.path.contains("quiver.desktop"),
        "{}",
        registration.path
    );

    // Pair with the remote and switch to it: it becomes ready and the switch completes.
    let paired = services
        .backend
        .add_connection("Remote", &remote.uri(), "pairing-code")
        .await
        .unwrap();
    services
        .backend
        .switch_connection(&paired.id)
        .await
        .unwrap();
    assert_eq!(
        services.backend.get_connections().await.unwrap().active_id,
        paired.id
    );

    // A real round trip over the new connection (it also yields to anything the switch spawned),
    // so a would-be announce has had every chance to reach the remote before we look.
    for _ in 0..3 {
        services
            .api
            .request(quiver_state::core_api::Method::Get, "/v0/whoami", None)
            .await
            .unwrap();
    }
    let received = remote.received_requests().await.unwrap();
    // Reading the remote's catalog is the library view doing its job; WRITING to it (register,
    // adopt, delete) is what the desktop must never do there.
    let catalog_writes: Vec<String> = received
        .iter()
        .filter(|r| r.method.as_str() != "GET" && r.url.path().starts_with("/v0/arrow"))
        .map(|r| format!("{} {}", r.method, r.url.path()))
        .collect();
    assert!(
        catalog_writes.is_empty(),
        "the remote daemon's catalog was written to: {catalog_writes:?}"
    );
    // The remote was reached with its own bearer token, and only for the probe and our calls.
    assert!(received.iter().any(|r| r.url.path() == "/v0/whoami"
        && r.headers
            .get("authorization")
            .is_some_and(|v| v == "Bearer remote-token")));

    // Back on the local connection the desktop announces again.
    services.backend.switch_connection("local").await.unwrap();
    local.wait_for(is_catalog_write).await;

    services.shutdown_async().await;
}

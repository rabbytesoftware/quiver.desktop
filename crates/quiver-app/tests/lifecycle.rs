//! `AppServices` end to end against a fake local daemon.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use quiver_app::services::AppServices;
use quiver_state::domain::ConnectionStatus;
use support::{accept_everything, FakeDaemon, LocalSetup};

async fn wait_for_status(
    services: &AppServices,
    wanted: ConnectionStatus,
) {
    let mut changes = services.status.store().subscribe();
    while services.status.status() != wanted {
        changes.changed().await.unwrap();
    }
}

#[tokio::test]
async fn the_local_daemon_is_adopted_the_stores_fill_and_shutdown_is_idempotent() {
    let setup = LocalSetup::new();
    let daemon = FakeDaemon::bind(&setup.socket, accept_everything());

    let services = AppServices::build(setup.config()).await;
    wait_for_status(&services, ConnectionStatus::Ready).await;

    let state = services.connections.snapshot();
    assert_eq!(state.active_id, "local");
    assert_eq!(state.connections.len(), 1);

    services.shutdown_async().await;
    services.shutdown_async().await;
    assert!(services.arrow_windows.open_apps().is_empty());

    // The daemon was found running, not started by this app, so quitting leaves it alone.
    assert!(
        tokio::net::UnixStream::connect(&setup.socket).await.is_ok(),
        "an adopted daemon must survive the app"
    );
    drop(daemon);
}

#[tokio::test]
async fn opening_an_arrow_app_without_a_window_to_show_it_in_fails_cleanly() {
    let setup = LocalSetup::new();
    let _daemon = FakeDaemon::bind(&setup.socket, accept_everything());
    let services = AppServices::build(setup.config()).await;

    assert!(services
        .arrow_windows
        .show("github.com/user/chat", "Chat")
        .await
        .is_err());
    assert!(services.arrow_windows.open_apps().is_empty());
    services.shutdown_async().await;
}

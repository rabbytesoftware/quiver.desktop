//! The Remote screen's add flow at the services level: pairing with a remote daemon files the
//! connection in the list (and announces the change), switching makes it the active one, and a
//! secret store that cannot keep the token fails the add instead of half-adding it.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;

use quiver_app::services::{AppServices, KeyringSecrets};
use quiver_state::domain::ConnectionKind;
use quiver_state::remote::flows::{add, connect};
use quiver_state::remote::{AddSubmission, ConnectionOutcome};
use support::{accept_everything, remote_daemon, FakeDaemon, LocalSetup};

fn submission(url: &str) -> AddSubmission {
    AddSubmission {
        name: "Lab".into(),
        url: url.into(),
        code: "123456".into(),
    }
}

#[tokio::test]
async fn adding_a_remote_lists_it_announces_the_change_and_can_then_be_switched_to() {
    let setup = LocalSetup::new();
    let _local = FakeDaemon::bind(&setup.socket, accept_everything());
    let remote = remote_daemon().await;
    let services = AppServices::build(setup.config()).await;
    let mut changes = services.backend.on_connections_changed();

    let outcome = add(services.backend.as_ref(), &submission(&remote.uri())).await;
    assert_eq!(outcome, ConnectionOutcome::Added);

    let announced = changes.recv().await.unwrap();
    let listed = services.backend.get_connections().await.unwrap();
    for snapshot in [&announced, &listed] {
        assert_eq!(snapshot.active_id, "local", "adding must not switch");
        let lab = snapshot
            .connections
            .iter()
            .find(|c| c.name == "Lab")
            .expect("the paired remote is in the list");
        assert_eq!(lab.kind, ConnectionKind::Remote);
    }

    let id = listed
        .connections
        .iter()
        .find(|c| c.name == "Lab")
        .unwrap()
        .id
        .clone();
    let outcome = connect(
        services.backend.as_ref(),
        &services.connections,
        &id,
        || services.status.status(),
    )
    .await;
    assert!(matches!(outcome, ConnectionOutcome::Connected { .. }));
    assert_eq!(services.connections.snapshot().active_id, id);
    assert_eq!(
        services.backend.get_connections().await.unwrap().active_id,
        id
    );
    services.shutdown_async().await;
}

/// A host with no usable secret store (a Linux session without a Secret Service) must not end in a
/// connection that is listed but cannot authenticate: the add fails and the list stays as it was.
#[tokio::test]
async fn a_secret_store_that_refuses_the_token_fails_the_add_and_lists_nothing() {
    struct Refusing;
    impl quiver_platform::SecretStore for Refusing {
        fn set(
            &self,
            _: &str,
            _: &str,
        ) -> quiver_platform::Result<()> {
            Err(quiver_platform::Error::Secret("no secret service".into()))
        }
        fn get(
            &self,
            _: &str,
        ) -> quiver_platform::Result<Option<String>> {
            Ok(None)
        }
        fn delete(
            &self,
            _: &str,
        ) -> quiver_platform::Result<()> {
            Ok(())
        }
    }
    let setup = LocalSetup::new();
    let _local = FakeDaemon::bind(&setup.socket, accept_everything());
    let remote = remote_daemon().await;
    let mut config = setup.config();
    config.secrets = Arc::new(KeyringSecrets::new(Arc::new(Refusing)));
    let services = AppServices::build(config).await;

    let outcome = add(services.backend.as_ref(), &submission(&remote.uri())).await;
    assert_eq!(outcome, ConnectionOutcome::AddFailed);
    let listed = services.backend.get_connections().await.unwrap();
    assert!(listed.connections.iter().all(|c| c.name != "Lab"));
    services.shutdown_async().await;
}

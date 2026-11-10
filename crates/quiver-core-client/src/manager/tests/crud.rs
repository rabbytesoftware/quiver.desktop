use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn remote(
    id: &str,
    name: &str,
) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: name.into(),
        kind: ConnectionKind::Remote,
        url: Some(format!("http://{id}.example")),
        api_version: "v0".into(),
    }
}

fn local_harness() -> Harness {
    harness(
        StubConnection::boxed("local", Arc::new(AtomicBool::new(false))),
        None,
    )
}

fn save(
    h: &Harness,
    configs: &[ConnectionConfig],
) {
    save_remote_configs(h.settings.as_ref(), configs).unwrap();
}

#[tokio::test]
async fn the_local_connection_cannot_be_removed_or_renamed() {
    let h = local_harness();
    assert!(matches!(
        h.manager.remove_connection("local").await,
        Err(Error::CannotRemoveLocal)
    ));
    assert!(matches!(
        h.manager.rename_connection("local", "x".into()).await,
        Err(Error::CannotRenameLocal)
    ));
}

#[tokio::test]
async fn removing_a_saved_connection_forgets_it_and_its_token_and_tells_the_host() {
    let h = local_harness();
    save(&h, &[remote("a", "A"), remote("b", "B")]);
    h.secrets.set("a", "tok").unwrap();

    h.manager.remove_connection("a").await.unwrap();

    let state = h.manager.get_connections().await;
    let ids: Vec<&str> = state.connections.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["local", "b"]);
    assert!(h.secrets.get("a").is_err());
    assert_eq!(h.events.changes.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn renaming_a_saved_connection_persists_and_tells_the_host() {
    let h = local_harness();
    save(&h, &[remote("a", "A")]);

    h.manager
        .rename_connection("a", "Renamed".into())
        .await
        .unwrap();

    let state = h.manager.get_connections().await;
    assert_eq!(state.connections[1].name, "Renamed");
    assert_eq!(h.events.changes.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn renaming_an_unknown_connection_is_an_error() {
    let h = local_harness();
    let err = h
        .manager
        .rename_connection("nope", "x".into())
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "connection nope not found");
}

#[tokio::test]
async fn switching_to_an_unknown_connection_changes_nothing() {
    let h = local_harness();
    let err = h.manager.switch_connection("nope").await.unwrap_err();
    assert_eq!(err.to_string(), "connection nope not found");
    assert_eq!(active_id(&h.manager).await, "local");
}

/// Pairing a new remote: redeem the code, keep the token in the secret store
/// (never in the saved config), save the connection and announce the change.
#[tokio::test]
async fn adding_a_connection_redeems_the_code_and_stores_the_token_apart() {
    let _serialised = crate::FD_TESTS.lock().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v0/auth/pairing/redeem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true, "error": null, "data": { "token": "secret-token" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let h = local_harness();

    let config = h
        .manager
        .add_connection("Lab".into(), server.uri(), "123456".into())
        .await
        .unwrap();

    assert_eq!(config.kind, ConnectionKind::Remote);
    assert_eq!(h.secrets.get(&config.id).unwrap(), "secret-token");
    let saved = load_remote_configs(h.settings.as_ref());
    assert_eq!(saved, vec![config]);
    assert!(!serde_json::to_string(&saved)
        .unwrap()
        .contains("secret-token"));
    assert_eq!(h.events.changes.lock().unwrap().len(), 1);
    // The same device id is reused by every later pairing.
    assert_eq!(
        device_id(h.settings.as_ref()).unwrap(),
        device_id(h.settings.as_ref()).unwrap()
    );
}

#[tokio::test]
async fn a_refused_pairing_code_saves_nothing() {
    let _serialised = crate::FD_TESTS.lock().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let h = local_harness();

    let err = h
        .manager
        .add_connection("Lab".into(), server.uri(), "bad".into())
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Pairing(_)), "got {err:?}");
    assert!(load_remote_configs(h.settings.as_ref()).is_empty());
    assert!(h.events.changes.lock().unwrap().is_empty());
}

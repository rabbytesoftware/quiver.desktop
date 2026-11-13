//! SCENARIO (new) -- quiver.core's own subsystems are settings on the Developer tab, next to Fletcher, and every
//! change lands in the real daemon's configuration: a duration typed and entered, a value core refuses (its
//! reason under the row, the field back to the held value), a switch, and a reset to the daemon's default.

use serde_json::{json, Value};

use crate::support::app::App;
use crate::support::daemon::Daemon;
use crate::support::env::Env;
use crate::support::home::Home;
use crate::support::wait::{eventually, LONG};

/// The configured value at `path` in the daemon's `GET /v0/config`.
async fn configured(
    core: &Daemon,
    path: &[&str],
) -> Value {
    let reply = core.get("/v0/config").await.expect("GET /v0/config");
    let body = reply.json();
    path.iter()
        .fold(&body["configured"], |v, key| &v[*key])
        .clone()
}

async fn until_configured(
    core: &Daemon,
    path: &[&str],
    want: Value,
) {
    eventually(&format!("{} to be {want}", path.join(".")), LONG, || {
        let want = want.clone();
        async move { (configured(core, path).await == want).then_some(()) }
    })
    .await;
}

/// Replaces the field's text and presses enter.
async fn retype(
    app: &mut App,
    id: &str,
    text: &str,
) {
    app.click(id).await;
    app.matching(json!({"id": id, "state_contains": "[focused]"}))
        .await;
    app.bridge.key("ctrl-a").await.expect("select all");
    app.bridge.type_text(None, text).await.expect("type");
    app.bridge.key("enter").await.expect("enter");
}

#[tokio::test]
async fn core_subsystems_are_settings_and_land_in_the_daemon() {
    let Some(env) = Env::load("core_settings") else {
        return;
    };
    let home = Home::fresh(&env, "core_settings");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    app.click("sidebar.tab.settings").await;
    app.click("settings.tab.developer").await;
    // Fletcher's row and its neighbours.
    app.node("settings.autoregister").await;
    let check = app.node("settings.config.version-check-interval").await;
    let held = configured(&core, &["arrows", "version_check_interval"]).await;
    assert!(
        check
            .state
            .contains(&format!("value={}", held.as_str().unwrap_or_default())),
        "{check:?} against {held}"
    );
    for id in ["fetch-timeout", "auto-retry", "retries", "port-forwarding"] {
        app.node(&format!("settings.config.{id}")).await;
    }
    app.shot("developer-core-settings").await;

    // A duration: typed, entered, saved by core.
    let interval = ["arrows", "version_check_interval"];
    retype(&mut app, "settings.config.version-check-interval", "12h").await;
    until_configured(&core, &interval, json!("12h")).await;
    app.matching(
        json!({"id": "settings.config.version-check-interval", "state_contains": "value=12h"}),
    )
    .await;

    // A value core refuses: its reason under the row, and the daemon keeps 12h.
    retype(&mut app, "settings.config.version-check-interval", "soon").await;
    app.matching(json!({"id": "settings.config.version-check-interval.description", "label_contains": "duration"}))
        .await;
    app.shot("developer-core-refused").await;
    assert_eq!(configured(&core, &interval).await, json!("12h"));

    // A count.
    retype(&mut app, "settings.config.retries", "5").await;
    until_configured(&core, &["arrows", "auto_retry", "retries"], json!(5)).await;

    // A switch.
    let retry = ["arrows", "auto_retry", "enabled"];
    let before = configured(&core, &retry).await;
    app.click("settings.config.auto-retry").await;
    until_configured(&core, &retry, json!(!before.as_bool().unwrap_or(true))).await;

    // Reset: back to the daemon's default.
    app.click("settings.config.version-check-interval.reset")
        .await;
    let defaults = core.get("/v0/config").await.expect("config").json()["defaults"]["arrows"]
        ["version_check_interval"]
        .clone();
    until_configured(&core, &interval, defaults).await;
}

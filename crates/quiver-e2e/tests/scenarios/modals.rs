//! The arrow page's two modals: Settings (one row per declared variable, validated as you type) and Methods
//! (the manifest's lifecycle methods, each expandable to its steps). Both open from the action row, close with
//! Escape and, for Settings, with Done.
//!
//! NOT checked: the error a value outside its bounds shows (`arrow.settings.<var>.error`, "At most 10"). The
//! bridge cannot type into the NumberField (it reports no focus and a click + key/type leaves `value=5`), so the
//! rule stays covered by the unit tests of `variable_form::validate`.
//!
//! Fixtures: e2e-form declares a bounded number (1..10) and a select, e2e-updatable has install, update and
//! uninstall methods. (A text variable has no validation: the daemon applies defaults to empty values.)

use serde_json::json;

use crate::support::app::{App, WAIT_MS};
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::lifecycle::register;

#[tokio::test]
async fn settings_validates_and_methods_expand_and_both_close() {
    let Some(env) = Env::load("modals") else {
        return;
    };
    let form = "github.com/rabbytesoftware/e2e-form";
    let updatable = required("QUIVER_E2E_UPDATABLE_NS");
    let home = Home::fresh(&env, "modals");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    register(&core, form).await;
    register(&core, &updatable).await;

    // Settings: one row per declared variable, seeded with the manifest defaults and error-free; Done closes.
    app.click(&format!("sidebar.row.{form}")).await;
    app.node("screen.arrow").await;
    assert!(app.find("arrow.settings").await.is_none());
    app.click("arrow.settings.open").await;
    app.node("arrow.settings").await;
    assert_eq!(app.node("arrow.settings.e2e_port").await.label, "E2E_PORT");
    app.matching(json!({"id": "arrow.settings.e2e_port", "state_contains": "value=5"}))
        .await;
    assert!(app.find("arrow.settings.e2e_port.error").await.is_none());
    app.node("arrow.settings.e2e_mode").await;
    app.matching(json!({"id": "arrow.settings.e2e_mode", "state_contains": "value=fast"}))
        .await;
    assert!(app.find("arrow.settings.e2e_port.error").await.is_none());
    app.shot("settings-defaults").await;
    app.click("arrow.settings.done").await;
    app.gone("arrow.settings").await;

    // Escape closes it as well.
    app.click("arrow.settings.open").await;
    app.node("arrow.settings").await;
    app.bridge.key("escape").await.expect("press escape");
    app.gone("arrow.settings").await;

    // Methods: the lifecycle methods the manifest declares, expandable one by one.
    app.click(&format!("sidebar.row.{updatable}")).await;
    app.matching(json!({"id": "arrow.title", "label_contains": "E2E Updatable"}))
        .await;
    app.click("arrow.methods.open").await;
    app.node("arrow.methods").await;
    for method in ["install", "update", "uninstall"] {
        let row = app.node(&format!("arrow.methods.{method}")).await;
        assert!(!row.state.contains("expanded"), "{method} starts collapsed");
    }
    app.click("arrow.methods.update.inspect").await;
    app.matching(json!({"id": "arrow.methods.update", "state_contains": "expanded"}))
        .await;
    app.shot("methods-expanded").await;
    app.click("arrow.methods.update.inspect").await;
    app.bridge
        .wait_matching(
            json!({"id": "arrow.methods.update", "state_contains": "expanded"}),
            false,
            WAIT_MS,
        )
        .await
        .expect("the row collapses again");
    app.bridge.key("escape").await.expect("press escape");
    app.gone("arrow.methods").await;
}

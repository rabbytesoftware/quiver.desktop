//! Driving an arrow's lifecycle on the daemon directly (not through the app), for the scenarios whose subject is
//! the app's reaction to it.

use serde_json::Value;

use super::app::App;
use super::arrows::{runtime, wait_arrow};
use super::daemon::{enc, Daemon};
use super::wait::{eventually, LONG};

/// `POST /v0/arrow/:ns`, then waits until the catalog knows it. Returns the catalog identity (`ns@selector`).
pub async fn register(core: &Daemon, ns: &str) -> String {
    let reply = core
        .post(&format!("/v0/arrow/{}", enc(ns)), None)
        .await
        .expect("add arrow");
    assert!(
        [200, 201].contains(&reply.status),
        "adding {ns} answered {}: {}",
        reply.status,
        reply.body
    );
    wait_arrow(core, ns, "in the catalog", |_| true)
        .await
        .namespace
}

/// [`register`], install, and waits for `ready`.
pub async fn install(core: &Daemon, ns: &str) -> String {
    let identity = register(core, ns).await;
    let status = runtime(core, &identity, "install").await;
    assert!(
        [200, 202].contains(&status),
        "installing {ns} answered {status}"
    );
    wait_arrow(core, &identity, "installed", |a| a.state == "ready").await;
    identity
}

/// Executes an installed arrow (it answers 202 and runs in the background).
pub async fn execute(core: &Daemon, identity: &str) {
    assert_eq!(runtime(core, identity, "execute").await, 202);
}

/// `GET /v0/runtime/:ns`, raw: `state`, `active_run` (with its `surface`), `last_return`.
pub async fn runtime_of(core: &Daemon, identity: &str) -> Value {
    core.get(&format!("/v0/runtime/{}", enc(identity)))
        .await
        .expect("runtime read")
        .json()
}

/// The interface the arrow's run opened: `{title, mode, path, ready}`, or null when it has none.
pub async fn surface(core: &Daemon, identity: &str) -> Value {
    runtime_of(core, identity).await["active_run"]["surface"].clone()
}

/// Waits for the surface to be `ready` (or not), and returns it.
pub async fn wait_surface(core: &Daemon, identity: &str, ready: bool) -> Value {
    eventually(
        &format!("{identity}: a surface with ready={ready}"),
        LONG,
        || async {
            let s = surface(core, identity).await;
            (s["ready"] == ready).then_some(s)
        },
    )
    .await
}

/// Waits for the run's surface to be gone.
pub async fn wait_no_surface(core: &Daemon, identity: &str) {
    eventually(&format!("{identity}: no surface"), LONG, || async {
        surface(core, identity).await.is_null().then_some(())
    })
    .await;
}

/// Installs through the daemon, then does what a person does: opens the arrow's page and presses Start. Because
/// the run opens an interface the page becomes the app pane (docs/arrow-apps.md).
pub async fn open_in_app(app: &mut App, core: &Daemon, ns: &str) -> String {
    let identity = install(core, ns).await;
    start_from_page(app, ns).await;
    identity
}

pub async fn start_from_page(app: &mut App, ns: &str) {
    app.click(&format!("sidebar.row.{ns}")).await;
    app.click("arrow.action.start").await;
    app.node("screen.arrow-app").await;
}

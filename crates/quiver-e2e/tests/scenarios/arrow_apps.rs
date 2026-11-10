//! ARROW APPS -- an arrow that opens an interface (a `ui:` on a run step) is served by the daemon on a socket it
//! provisions, proxied at `/v0/ui/<ns>`, and shown by the app in a webview embedded in its main window
//! (docs/decision-log.md). Driven with the repository's own fixture apps (docker/e2e/fixtures/arrows): e2e-echo-app
//! (binds its socket 5 s late and echoes request headers), e2e-static-app-1..3 (static files, run for a long
//! time) and e2e-short-app (its run ends by itself after 12 s). quiver.chat's own test needs its checkout and
//! skips with a message without it.
//!
//! The page's DOM is not reachable from the bridge, so what is asserted is what the box can see around it:
//!   * the daemon: the surface (`mode`, `ready`), the run directory (0700, one socket named from the namespace),
//!     no new TCP listener, `/v0/ui` stripping Authorization/Cookie, 503 after the run ended;
//!   * the app: `state.dump` > `arrowWindows` (open apps, remount keys, and each `arrow-app://` request the
//!     page made with the status and CSP of the answer), the pane header (`arrow-app.*` ids) and the route;
//!   * the OS: the page is a mapped child window of the main window, exactly over the pane body, and the
//!     main window is the only top-level one (xwininfo).
//!
//! NOT checked, because it needs the DOM of the page (the old spec ran it in the webview): the sandbox attribute
//! of the iframe (there is none: the page is a native webview), "no Tauri globals" in the page, a page navigating
//! itself to `quiver://`, WebSocket frames crossing the shim, sessionStorage/localStorage per origin. The handler
//! rules behind those (host allow-list, CSP, header stripping, ws scoping) are unit-tested in quiver-arrow-host.
//! Also not ported: the 15-cycle execute/stop stress (A6, a daemon claim) and the tcp bearer checks (A4).

use serde_json::{json, Value};

use crate::support::app::App;
use crate::support::arrow_windows::{
    open_apps, requests, until, wait_embedded, wait_open, wait_pages, ARROW_CSP,
};
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::lifecycle::{open_in_app, runtime_of, surface, wait_no_surface, wait_surface};
use crate::support::procs::alive;
use crate::support::sandbox::{run_dir, socket_name, tcp_listeners, ui_request};

fn statics() -> Vec<String> {
    required("QUIVER_E2E_STATIC_NS")
        .split_whitespace()
        .map(String::from)
        .collect()
}

#[path = "arrow_apps_chat.rs"]
mod chat;
#[tokio::test]
async fn a_late_binding_app_is_isolated_and_its_window_opens_only_when_ready() {
    let Some(env) = Env::load("arrow_apps") else {
        return;
    };
    let echo = required("QUIVER_E2E_ECHO_NS");
    let home = Home::fresh(&env, "arrow_apps_echo");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let listeners = tcp_listeners();

    let identity = open_in_app(&mut app, &core, &echo).await;

    // The interface is announced before it answers; the app opens no window for it yet.
    let starting = wait_surface(&core, &identity, false).await;
    assert_eq!(
        starting,
        json!({"title": "E2E Echo App", "mode": "listen", "path": "/", "ready": false})
    );
    assert!(
        open_apps(&mut app).await.is_empty(),
        "a window opened before the interface was ready"
    );
    wait_embedded(&mut app, 0).await;
    app.shot("echo-starting").await;

    wait_surface(&core, &identity, true).await;
    wait_open(&mut app, &[&echo]).await;
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;
    app.shot("echo-open").await;

    // Details opens the arrow's own page in a dialog over the app, as in Tauri (docs/arrow-apps.md). The page
    // stays alive but steps aside while the dialog is open, and is back when it closes.
    app.click("arrow-app.details").await;
    app.node("screen.arrow").await;
    wait_embedded(&mut app, 0).await;
    assert_eq!(
        open_apps(&mut app).await,
        vec![echo.clone()],
        "the dialog must not close the app"
    );
    app.shot("echo-details-dialog").await;
    app.bridge.key("escape").await.expect("press escape");
    app.gone("screen.arrow").await;
    wait_embedded(&mut app, 1).await;

    // The window's first page load went through the host's handler: 200, HTML, the CSP, nothing sensitive sent.
    let loaded = until(&mut app, "the page was requested", |d| {
        d["protocol"]
            .as_array()
            .is_some_and(|a| a.iter().any(|e| e["status"] == 200))
    })
    .await;
    assert!(loaded["protocol"].is_array());
    let page = requests(&mut app, &echo).await;
    let first = page
        .iter()
        .find(|e| e["status"] == 200)
        .expect("a served request");
    assert_eq!(first["csp"], ARROW_CSP);
    assert!(
        first["contentType"]
            .as_str()
            .is_some_and(|c| c.starts_with("text/html")),
        "{first}"
    );
    for exchange in &page {
        let sent = exchange["requestHeaders"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for secret in ["authorization", "cookie"] {
            assert!(
                !sent.iter().any(|h| h == secret),
                "the page sent {secret}: {exchange}"
            );
        }
        assert!(
            exchange["csp"].is_string(),
            "an answer without a CSP: {exchange}"
        );
    }

    // Isolation: no TCP listener (the interface is a unix socket), and the run directory is private with one
    // socket named from the namespace.
    assert_eq!(
        tcp_listeners(),
        listeners,
        "running the app opened a TCP listener"
    );
    let (mode, sockets) = run_dir(&home);
    assert_eq!(mode, 0o700);
    assert_eq!(sockets, vec![socket_name(&identity)]);

    // The daemon never forwards Authorization or Cookie to the arrow and keeps ordinary headers.
    let (status, body) = ui_request(
        &home.socket(),
        &identity,
        "/headers?via=daemon",
        &[
            ("Authorization", "Bearer leaked-token"),
            ("Cookie", "session=leaked"),
            ("X-E2E-Kept", "yes"),
        ],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let seen: Value = serde_json::from_str(&body).expect("the echo's JSON");
    assert_eq!(seen["headers"]["x-e2e-kept"], "yes");
    assert!(seen["headers"]["authorization"].is_null(), "{seen}");
    assert!(seen["headers"]["cookie"].is_null(), "{seen}");
    assert_eq!(seen["path"], "/headers?via=daemon");
}

#[tokio::test]
async fn the_window_cap_is_four_and_the_oldest_hidden_app_makes_room() {
    let Some(env) = Env::load("arrow_apps") else {
        return;
    };
    let (echo, statics) = (required("QUIVER_E2E_ECHO_NS"), statics());
    let home = Home::fresh(&env, "arrow_apps_cap");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    // Four apps open, each in a window of its own with its own origin.
    let mut identities = Vec::new();
    let mut opened: Vec<&str> = Vec::new();
    for ns in statics[..3]
        .iter()
        .map(String::as_str)
        .chain([echo.as_str()])
    {
        identities.push(open_in_app(&mut app, &core, ns).await);
        let id = identities.last().expect("identity").clone();
        wait_surface(&core, &id, true).await;
        opened.push(ns);
        wait_open(&mut app, &opened).await;
        wait_pages(&mut app, opened.len()).await;
        // Only the app on screen has a visible page; the others are kept alive hidden.
        wait_embedded(&mut app, 1).await;
    }
    let hosts: std::collections::BTreeSet<String> = {
        let dump = until(&mut app, "all four loaded", |d| {
            opened.iter().all(|ns| {
                d["protocol"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|e| e["namespace"] == *ns))
            })
        })
        .await;
        dump["protocol"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|e| e["namespace"].as_str().map(String::from))
            .collect()
    };
    assert_eq!(hosts.len(), 4, "every app loaded its own page: {hosts:?}");
    app.shot("four-open").await;

    // A fifth evicts the least recently used one and tells the person.
    open_in_app(&mut app, &core, &statics[3]).await;
    let order = wait_open(&mut app, &[&statics[1], &statics[2], &echo, &statics[3]]).await;
    assert!(!order.contains(&statics[0]), "{order:?}");
    let notice = app.node("arrow-app.notice").await;
    assert!(notice.label.contains("E2E Static App 1"), "{notice:?}");
    app.shot("fifth-evicts-first").await;
    wait_pages(&mut app, 4).await;

    // Closing the page did not stop the app: its run is still going.
    let evicted = runtime_of(&core, &identities[0]).await;
    assert_eq!(evicted["state"], "running");
    assert!(!surface(&core, &identities[0]).await.is_null());
}

#[tokio::test]
async fn reload_remounts_the_page_and_stop_ends_the_window_and_the_process() {
    let Some(env) = Env::load("arrow_apps") else {
        return;
    };
    let ns = statics().remove(0);
    let home = Home::fresh(&env, "arrow_apps_reload_stop");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let identity = open_in_app(&mut app, &core, &ns).await;
    wait_surface(&core, &identity, true).await;
    wait_open(&mut app, &[&ns]).await;
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;
    until(&mut app, "the first page load", |d| {
        d["protocol"]
            .as_array()
            .is_some_and(|a| a.iter().any(|e| e["namespace"] == ns))
    })
    .await;
    let before = requests(&mut app, &ns).await.len();
    assert_eq!(
        until(&mut app, "dump", |_| true).await["reloadKeys"][&ns],
        0
    );

    // Reload: the app's remount key moves, the helper is told, and the page is requested again.
    app.click("arrow-app.reload").await;
    until(&mut app, "the remount key to move", |d| {
        d["reloadKeys"][&ns] == 1
    })
    .await;
    let reloaded = until(&mut app, "the page requested again", |d| {
        d["protocol"]
            .as_array()
            .is_some_and(|a| a.iter().filter(|e| e["namespace"] == ns).count() > before)
    })
    .await;
    assert!(reloaded["open"].as_array().is_some_and(|a| a.len() == 1));
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;
    app.shot("after-reload").await;

    // Stop: the window and the process go, the surface is gone, /v0/ui answers 503, the page is the arrow's again.
    let pid = runtime_of(&core, &identity).await["active_run"]["pid"]
        .as_u64()
        .expect("the run's pid") as u32;
    app.click("arrow-app.stop").await;
    app.node("screen.arrow").await;
    wait_open(&mut app, &[]).await;
    wait_pages(&mut app, 0).await;
    wait_embedded(&mut app, 0).await;
    wait_no_surface(&core, &identity).await;
    crate::support::wait::eventually(
        "the run's process to end",
        crate::support::wait::SHORT,
        || async { (!alive(pid)).then_some(()) },
    )
    .await;
    let (status, _) = ui_request(&home.socket(), &identity, "/", &[]).await;
    assert_eq!(status, 503);
    app.shot("after-stop").await;
}

#[path = "arrow_apps_ended.rs"]
mod ended;

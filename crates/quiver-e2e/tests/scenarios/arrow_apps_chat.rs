//! quiver.chat in its window: the real app, sandboxed, opened and stopped like any other arrow app.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrow_windows::{wait_embedded, wait_open, wait_pages};
use crate::support::daemon::Daemon;
use crate::support::env::{optional, Env};
use crate::support::home::Home;
use crate::support::lifecycle::{open_in_app, wait_no_surface, wait_surface};
use crate::support::sandbox::{run_dir, socket_name, tcp_listeners, ui_request};

/// quiver.chat itself. Needs its checkout (mounted at /workspace/quiver.chat, docker/e2e/README.md): without it
/// this test says why and passes, so the other arrow-app tests still count. NOT run in the environment this was
/// written in (no checkout); the steps mirror the fixture test above.
#[tokio::test]
async fn quiver_chat_runs_sandboxed_in_its_window_and_stops() {
    let Some(env) = Env::load("arrow_apps") else {
        return;
    };
    let manifest =
        optional("QUIVER_E2E_CHAT_MANIFEST").filter(|m| std::path::Path::new(m).is_file());
    let (Some(_), Some(chat)) = (manifest, optional("QUIVER_E2E_CHAT_NS")) else {
        eprintln!("SKIP arrow_apps quiver.chat: no quiver.chat checkout (QUIVER_E2E_CHAT_MANIFEST is not a file); the fixture apps above cover the same claims");
        return;
    };
    let home = Home::fresh(&env, "arrow_apps_chat");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let listeners = tcp_listeners();
    let identity = open_in_app(&mut app, &core, &chat).await;
    let s = wait_surface(&core, &identity, true).await;
    assert_eq!(
        s,
        json!({"title": "Quiver Chat", "mode": "listen", "path": "/", "ready": true})
    );
    assert_eq!(tcp_listeners(), listeners);
    let (mode, sockets) = run_dir(&home);
    assert_eq!((mode, sockets), (0o700, vec![socket_name(&identity)]));
    wait_open(&mut app, &[&chat]).await;
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;
    app.click("arrow-app.stop").await;
    wait_open(&mut app, &[]).await;
    wait_pages(&mut app, 0).await;
    wait_no_surface(&core, &identity).await;
    let (status, _) = ui_request(&home.socket(), &identity, "/", &[]).await;
    assert_eq!(status, 503);
}

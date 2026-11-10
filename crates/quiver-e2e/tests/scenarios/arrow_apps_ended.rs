//! The run that serves the interface ends by itself: the pane goes back to the arrow's page, and Start brings it back.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrow_windows::{wait_embedded, wait_open, wait_pages};
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::lifecycle::{open_in_app, start_from_page, wait_no_surface, wait_surface};
use crate::support::sandbox::ui_request;

#[tokio::test]
async fn when_the_run_serving_the_interface_ends_the_pane_returns_to_the_arrow_page() {
    let Some(env) = Env::load("arrow_apps") else {
        return;
    };
    let ns = required("QUIVER_E2E_SHORT_NS");
    let home = Home::fresh(&env, "arrow_apps_short");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let identity = open_in_app(&mut app, &core, &ns).await;
    let surface = wait_surface(&core, &identity, true).await;
    assert_eq!(
        surface,
        json!({"title": "E2E Short App", "mode": "static", "path": "/", "ready": true})
    );
    wait_open(&mut app, &[&ns]).await;
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;

    // The run is `sleep 12`: nothing stops it.
    wait_no_surface(&core, &identity).await;
    app.node("screen.arrow").await;
    wait_open(&mut app, &[]).await;
    wait_pages(&mut app, 0).await;
    wait_embedded(&mut app, 0).await;
    let (status, _) = ui_request(&home.socket(), &identity, "/", &[]).await;
    assert_eq!(status, 503);
    app.shot("short-ended").await;

    // And it starts again.
    start_from_page(&mut app, &ns).await;
    wait_surface(&core, &identity, true).await;
    wait_open(&mut app, &[&ns]).await;
    wait_pages(&mut app, 1).await;
    wait_embedded(&mut app, 1).await;
}

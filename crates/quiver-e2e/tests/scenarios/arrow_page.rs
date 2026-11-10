//! The arrow page's action row follows the arrow's state: which buttons exist before install, while an install
//! runs (the ProgressButton, whose "Steps" popover lists the manifest's steps as done / running / pending), once
//! installed, and while running. Driven for real: the install of the e2e-slow fixture (three steps, the second
//! waits ~20 s) runs on the real quiver.core, started by clicking the page's Install button.
//!
//! Where a claim has no native check: pixel details of the button (colours, the fill of the progress bar) are not
//! asserted; the fill is reported as `fraction=<0..1>` in the progress element's state and checked to be a number
//! between 0 and 1.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrows::wait_arrow;
use crate::support::daemon::Daemon;
use crate::support::env::Env;
use crate::support::home::Home;
use crate::support::lifecycle::register;

const SLOW: &str = "github.com/rabbytesoftware/e2e-slow";

async fn actions(app: &mut App) -> Vec<String> {
    let mut names: Vec<String> = app
        .with_prefix("arrow.action.")
        .await
        .into_iter()
        .filter(|n| !n.id.ends_with(".info"))
        .map(|n| n.id.trim_start_matches("arrow.action.").to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn the_action_row_follows_the_arrow_through_install_start_and_stop() {
    let Some(env) = Env::load("arrow_page") else {
        return;
    };
    let home = Home::fresh(&env, "arrow_page");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let slow = register(&core, SLOW).await;

    app.click(&format!("sidebar.row.{SLOW}")).await;
    app.node("screen.arrow").await;

    // Not installed: Install and the way out of the library, nothing that needs an installation.
    app.node("arrow.action.install").await;
    let before = actions(&mut app).await;
    assert!(before.contains(&"install".to_string()), "{before:?}");
    for absent in ["start", "stop", "uninstall", "update"] {
        assert!(
            !before.contains(&absent.to_string()),
            "{absent} in {before:?}"
        );
    }
    assert!(app.find("arrow.progress").await.is_none());
    app.shot("page-not-installed").await;

    // Installing: the Install button turns into the ProgressButton; its Steps popover lists the manifest's steps.
    app.click("arrow.action.install").await;
    let progress = app.node("arrow.progress").await;
    assert!(progress.state.contains("fraction="), "{progress:?}");
    app.gone("arrow.action.install").await;
    app.click("arrow.steps").await;
    app.node("arrow.steps.popover").await;
    // Core adds its own first step ("Resolve dependencies"), so the manifest's steps are found by title.
    app.matching(
        json!({"prefix": "arrow.steps.", "label_contains": "Prepare", "state_contains": "done"}),
    )
    .await;
    app.matching(
        json!({"prefix": "arrow.steps.", "label_contains": "Settle", "state_contains": "running"}),
    )
    .await;
    app.matching(
        json!({"prefix": "arrow.steps.", "label_contains": "Finish", "state_contains": "pending"}),
    )
    .await;
    let fraction: f32 = app
        .node("arrow.progress")
        .await
        .state
        .split("fraction=")
        .nth(1)
        .and_then(|f| f.split_whitespace().next()?.parse().ok())
        .expect("a fraction");
    assert!((0.0..=1.0).contains(&fraction), "fraction {fraction}");
    app.shot("page-installing-steps").await;

    // Installed: the progress button is gone, Start and Uninstall are there.
    wait_arrow(&core, &slow, "installed", |a| a.state == "ready").await;
    app.gone("arrow.progress").await;
    app.node("arrow.action.start").await;
    app.node("arrow.action.uninstall").await;
    assert!(app.find("arrow.action.install").await.is_none());
    app.shot("page-installed").await;

    // Running: Stop (and Restart) replace Start.
    app.click("arrow.action.start").await;
    app.node("arrow.action.stop").await;
    app.gone("arrow.action.start").await;
    wait_arrow(&core, &slow, "running", |a| a.state == "running").await;
    let running = actions(&mut app).await;
    assert!(running.contains(&"stop".to_string()), "{running:?}");
    app.shot("page-running").await;

    // The ⓘ next to an action opens its explanation, and closes again.
    if app.find("arrow.action.stop.info").await.is_some() {
        app.click("arrow.action.stop.info").await;
        app.node("arrow.info.popover").await;
        app.click("arrow.action.stop.info").await;
        app.gone("arrow.info.popover").await;
    }

    // Stopped: back to Start.
    app.click("arrow.action.stop").await;
    app.node("arrow.action.start").await;
    wait_arrow(&core, &slow, "stopped", |a| a.state == "ready").await;
}

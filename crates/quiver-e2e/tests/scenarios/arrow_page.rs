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
    // The installing row (progress, ⌄, a disabled Remove from Library) wraps rather than run under the banner.
    if let Some(banner) = app.find("arrow.banner").await {
        let mut row = app.with_prefix("arrow.action.").await;
        row.push(app.node("arrow.progress").await);
        for node in row {
            assert!(
                node.x + node.w <= banner.x + 0.5,
                "{node:?} runs under the banner {banner:?}"
            );
        }
    }
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

const UPDATABLE: &str = "github.com/rabbytesoftware/e2e-updatable";

/// The page follows the AFTER board, not the old Hero: the version is the header's chip (the way to switch what the
/// row follows), with no selector line, no Switch… button and no "last run" block of the old Activity tab. The
/// "At a glance" rows keep their links inside the card.
#[tokio::test]
async fn the_page_follows_the_board_and_the_version_chip_switches() {
    let Some(env) = Env::load("arrow_page_board") else {
        return;
    };
    let home = Home::fresh(&env, "arrow_page_board");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    register(&core, UPDATABLE).await;

    app.click(&format!("sidebar.row.{UPDATABLE}")).await;
    app.node("screen.arrow").await;
    app.node("arrow.title").await;
    app.node("arrow.version").await;
    for gone in [
        "arrow.switch",
        "arrow.activity",
        "arrow.pick.channel",
        "arrow.pick.version",
    ] {
        assert!(app.find(gone).await.is_none(), "{gone} is on the page");
    }

    // Every "At a glance" row sits inside the card the rows make up.
    let rows = app.with_prefix("arrow.glance.").await;
    if let Some(card) = rows.iter().map(|r| r.x + r.w).reduce(f32::max) {
        for row in &rows {
            assert!(row.x + row.w <= card + 0.5, "{row:?} runs past the card");
        }
    }

    // The repository publishes channels (a branch and a tag): the chip opens the Switch dialog.
    app.matching(json!({"id": "arrow.version", "state_contains": "switch=enabled"}))
        .await;
    app.shot("page-board").await;
    app.click("arrow.version").await;
    // In the library but not installed: switching re-registers, nothing is uninstalled, and the dialog says so.
    app.matching(json!({"id": "arrow.switch-dialog", "state_contains": "mode=switch-idle"}))
        .await;
    app.shot("switch-dialog").await;
    // A dropdown inside a dialog: both are layers over the page. GPUI aborts the app when one deferred layer
    // draws another ("cannot call defer_draw during deferred drawing"), which is how the playground crashed.
    app.click("arrow.switch-pick.channel").await;
    app.matching(json!({"id": "arrow.switch-pick.channel", "state_contains": "open"}))
        .await;
    let option = app
        .matching(json!({"prefix": "arrow.switch-pick.channel.option."}))
        .await;
    app.shot("switch-dialog-select-open").await;
    app.click(&option.id).await;
    app.gone_matching(json!({"id": "arrow.switch-pick.channel", "state_contains": "open"}))
        .await;
    app.node("arrow.switch-dialog").await;
    app.bridge.key("escape").await.expect("escape");
    app.gone("arrow.switch-dialog").await;

    // Not in the library: the very same chip (`namespace@selector · version ⌄`), no pickers on the page, and the
    // same dialog, which only chooses what Install / Add to Library will use.
    // Reached the way the old app's search bar allowed: Enter on a full namespace goes straight to its page.
    app.enter_text("sidebar.search", "github.com/rabbytesoftware/e2e-required")
        .await;
    app.matching(json!({"id": "arrow.namespace", "label_contains": "e2e-required"}))
        .await;
    app.matching(json!({"id": "arrow.version", "state_contains": "switch=enabled"}))
        .await;
    // The chip names what the pick resolves to (once crowbar's said nothing: the channels had landed before
    // the selector, and the picker was never seeded again).
    let chip = app.node("arrow.version").await;
    assert!(
        !chip.label.trim().is_empty(),
        "the version chip is empty: {chip:?}"
    );
    for gone in ["arrow.pick.channel", "arrow.pick.version"] {
        assert!(app.find(gone).await.is_none(), "{gone} is on the page");
    }
    let namespace = app.node("arrow.namespace").await;
    assert!(
        namespace.label.contains('@'),
        "the namespace line does not say what it follows: {namespace:?}"
    );
    app.shot("page-not-in-library").await;
    app.click("arrow.version").await;
    app.matching(json!({"id": "arrow.switch-dialog", "state_contains": "mode=choose"}))
        .await;
    app.shot("choose-dialog").await;
    app.bridge.key("escape").await.expect("escape");
    app.gone("arrow.switch-dialog").await;
}

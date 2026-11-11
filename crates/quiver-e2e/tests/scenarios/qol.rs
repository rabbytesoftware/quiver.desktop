//! The pre-playtest QOL fixes, in the real built app against a real daemon:
//!
//!   1. a screen starts at the top rather than inheriting the last one's scroll
//!   2. focusing the search field does not navigate; typing does
//!   3. when an update finishes, the Update button goes away on its own, with no navigation
//!
//! Substituted: the old spec observed `scrollTop` of one shared scroller and a `window` marker that proves "no
//! reload". Natively every route is its own view with its own scroll state, so (1) is checked as: scroll the
//! arrow page (its title moves up the window), leave for Settings and come back, and the title is back at its
//! original y. For (3), "no reload / no navigation" is the absence of a new `route:` signal target: the screen
//! element stays `screen.arrow` for the whole update and the button disappears from the same page.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrows::{add, check_now, runtime, wait_arrow};
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::upstream::Upstream;

const UPDATABLE_REPO: &str = "rabbytesoftware/e2e-updatable";

#[tokio::test]
async fn the_pre_playtest_qol_fixes() {
    let Some(env) = Env::load("qol") else { return };
    let upstream = Upstream::load();
    let updatable = required("QUIVER_E2E_UPDATABLE_NS");
    let home = Home::fresh(&env, "qol");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    assert!([200, 201].contains(&add(&core, &updatable).await));
    let identity = wait_arrow(&core, &updatable, "in the catalog", |_| true)
        .await
        .namespace;
    assert_eq!(runtime(&core, &identity, "install").await, 202);
    wait_arrow(&core, &identity, "the updatable arrow installed", |a| {
        a.state == "ready"
    })
    .await;

    // 1. A screen starts at the top. A short window makes the arrow page scroll at all.
    app.bridge
        .call("window.resize", json!({"width": 1000, "height": 520}))
        .await
        .expect("resize");
    app.click(&format!("sidebar.row.{updatable}")).await;
    app.node("screen.arrow").await;
    // Too narrow for the Details rail (the old page's 860px rule): the README keeps a readable column instead of
    // being squeezed beside the card, and an arrow whose manifest has no details shows no empty card.
    let readme = app.node("arrow.readme").await;
    assert!(readme.w >= 500., "the README column is {}px wide", readme.w);
    app.gone("arrow.details").await;
    let title = app.node("arrow.title").await;
    app.bridge
        .scroll("arrow.title", 0.0, 300.0)
        .await
        .expect("scroll");
    let scrolled = app
        .matching(json!({"id": "arrow.title", "y_lt": title.y - 20.0}))
        .await;
    app.shot("01-scrolled").await;
    assert!(scrolled.y < title.y);
    app.click("sidebar.tab.settings").await;
    app.route("/settings").await;
    app.node("screen.settings").await;
    app.click(&format!("sidebar.row.{updatable}")).await;
    let back = app
        .matching(json!({"id": "arrow.title", "y_gt": title.y - 2.0, "y_lt": title.y + 2.0}))
        .await;
    app.shot("02-next-screen-at-top").await;
    assert!((back.y - title.y).abs() < 2.0);

    // 2. The search field: focusing it does not navigate...
    app.click("sidebar.tab.settings").await;
    app.node("screen.settings").await;
    app.click("sidebar.search").await;
    app.matching(json!({"id": "sidebar.search", "state_contains": "[focused]"}))
        .await;
    assert!(
        app.find("screen.settings").await.is_some(),
        "focusing the search field left Settings"
    );
    assert!(
        app.find("screen.search").await.is_none(),
        "focusing the search field opened the results"
    );
    app.shot("03-search-focused").await;
    // ...typing does.
    app.type_into("sidebar.search", "redis").await;
    app.route("/search").await;
    app.node("screen.search").await;
    app.matching(json!({"id": "sidebar.search", "state_contains": "redis"}))
        .await;
    app.shot("04-search-typed").await;

    // 3. An update finishing drops its button on its own.
    app.click(&format!("sidebar.row.{updatable}")).await;
    app.node("screen.arrow").await;
    // Leaving the results takes the keyboard from the search field: typing now must not go into it.
    let search = app.node("sidebar.search").await;
    assert!(
        !search.state.contains("[focused]"),
        "the search field kept the focus on the arrow page: {search:?}"
    );
    assert!(
        app.find("arrow.action.update").await.is_none(),
        "Update shown before a newer release exists"
    );
    upstream.publish_repo_tag(UPDATABLE_REPO, "stable-1.1");
    check_now(&core, &identity).await;
    wait_arrow(&core, &identity, "the updatable arrow outdated", |a| {
        a.state == "outdated"
    })
    .await;
    app.node("arrow.action.update").await;
    app.shot("05-update-available").await;

    app.click("arrow.action.update").await;
    wait_arrow(&core, &identity, "the update committed in core", |a| {
        a.resolved_ref.as_deref() == Some("stable-1.1")
            && a.available.is_none()
            && a.state == "ready"
            && a.active_run.is_none()
    })
    .await;
    app.gone("arrow.action.update").await;
    assert!(
        app.find("screen.arrow").await.is_some(),
        "the update navigated away from the arrow page"
    );
    app.shot("06-update-done").await;
}

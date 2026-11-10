//! The status glyph on a sidebar row says what state its arrow is in: busy while a method runs, running,
//! update available, and needs attention (the supervised process outlived a daemon swap). Driven through the real
//! daemon; the app is asserted through `sidebar.row.<ns>.glyph`, whose label is the glyph kind.
//!
//! Substituted claim: the old app had no such check (the glyph was SVG with no accessible name); natively the
//! kind is what the view decided, read from the id, not pixels. That the dots are drawn is not asserted.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrows::{check_now, runtime, wait_arrow};
use crate::support::core_update::Setup;
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::lifecycle::{execute, install, register};
use crate::support::upstream::{ReleaseOptions, Upstream};

const SLOW: &str = "github.com/rabbytesoftware/e2e-slow";

async fn glyph(
    app: &mut App,
    ns: &str,
    kind: &str,
) {
    app.matching(json!({"id": format!("sidebar.row.{ns}.glyph"), "label_contains": kind}))
        .await;
}

#[tokio::test]
async fn sidebar_rows_show_the_glyph_of_their_state() {
    let Some(env) = Env::load("glyphs") else {
        return;
    };
    let upstream = Upstream::load();
    let updatable = required("QUIVER_E2E_UPDATABLE_NS");
    let home = Home::fresh(&env, "glyphs");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    // Busy: the install of the slow arrow sits in its second step for ~20 s.
    let slow = register(&core, SLOW).await;
    app.node(&format!("sidebar.row.{SLOW}")).await;
    assert!(app
        .find(&format!("sidebar.row.{SLOW}.glyph"))
        .await
        .is_none());
    assert_eq!(runtime(&core, &slow, "install").await, 202);
    glyph(&mut app, SLOW, "busy").await;
    app.shot("glyph-busy").await;
    wait_arrow(&core, &slow, "installed", |a| a.state == "ready").await;
    app.gone(&format!("sidebar.row.{SLOW}.glyph")).await;

    // Running.
    execute(&core, &slow).await;
    glyph(&mut app, SLOW, "running").await;
    app.shot("glyph-running").await;

    // Update available: an ordinary arrow with a newer release ahead.
    let identity = install(&core, &updatable).await;
    upstream.publish_repo_tag("rabbytesoftware/e2e-updatable", "stable-1.1");
    check_now(&core, &identity).await;
    glyph(&mut app, &updatable, "update-available").await;
    app.shot("glyph-update-available").await;
}

/// The daemon replaces itself while the supervised arrow runs, which leaves its process detached; the row must then
/// say "needs attention".
///
/// The reconnect after the swap re-reads the list over the `running` overlay the live frames painted
/// (`ArrowStore::forget_live_state`), so the glyph follows the daemon's `detached`.
#[tokio::test]
async fn a_detached_arrow_after_a_daemon_swap_shows_needs_attention() {
    let Some(mut s) = Setup::start("glyphs_detached").await else {
        return;
    };
    let fixture = required("QUIVER_E2E_FIXTURE_NS");
    let v2 = s.tags[1].clone();
    glyph(&mut s.app, &fixture, "running").await;
    s.offer(&v2, ReleaseOptions::default()).await;
    s.app.click("settings.core.update").await;
    s.run().swapped_to(&v2).await;
    let core = s.core.clone();
    wait_arrow(&core, &fixture, "detached by the swap", |a| {
        a.state == "detached"
    })
    .await;
    glyph(&mut s.app, &fixture, "needs-attention").await;
    s.app.shot("glyph-needs-attention").await;
}

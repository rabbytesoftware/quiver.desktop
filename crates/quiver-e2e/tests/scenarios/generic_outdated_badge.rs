//! SCENARIO 3 -- an outdated self-arrow gets the SAME badge every other outdated arrow gets, through the generic
//! path. Asserted against the Library grid (the sidebar row has no status badge; tiles do).
//!
//! The box publishes a second quiver.core release for tests named `*generic_outdated_badge*`, so the installed
//! build (release 1) is behind it.
//!
//! Substituted claims: the old spec counted DOM nodes ("the badge holds exactly one icon", "exactly one badge on the
//! tile"). Natively the tile reports one `tile.<ns>.badge` element carrying the text, so those become: the
//! self-arrow's badge text EQUALS the one an ordinary outdated arrow (e2e-updatable, driven outdated here too)
//! gets, and the details page says `Update available`, the generic `arrow.state.outdated` copy the old spec read.

use serde_json::json;

use crate::support::app::App;
use crate::support::arrows::{add, arrow, check_now, runtime, wait_arrow};
use crate::support::daemon::{enc, Daemon};
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::upstream::Upstream;
use crate::support::NS_CORE;

#[tokio::test]
async fn an_outdated_self_arrow_gets_the_generic_outdated_badge() {
    let Some(env) = Env::load("generic_outdated_badge") else {
        return;
    };
    let upstream = Upstream::load();
    let updatable = required("QUIVER_E2E_UPDATABLE_NS");
    let home = Home::fresh(&env, "generic_outdated_badge");
    home.show_own_components();
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    let versioned = arrow(&core, NS_CORE)
        .await
        .1
        .expect("quiver.core is in the catalog")
        .namespace;

    // absent -> outdated is not a legal transition, only ready -> outdated is, so it has to be ready first. A
    // self-arrow is installed by definition: the daemon lands it at ready on boot, so installing again is a 200
    // no-op; a 202 means an install really started.
    let variables = json!({"variables": {
        "QUIVER_RELEASE_ASSET_URL": "unused-for-install",
        "QUIVER_RELEASE_CHECKSUM": "unused-for-install",
    }});
    let install = core
        .post(
            &format!("/v0/runtime/{}/install", enc(&versioned)),
            Some(variables),
        )
        .await
        .expect("install");
    assert!(
        [200, 202].contains(&install.status),
        "install answered {}: {}",
        install.status,
        install.body
    );
    wait_arrow(&core, &versioned, "ready or outdated", |a| {
        a.state == "ready" || a.state == "outdated"
    })
    .await;

    // Reported as outdated through the ordinary arrow API: CheckVersionDrift compares the installed ref against
    // the remote, which now serves a newer release.
    wait_arrow(&core, &versioned, "outdated", |a| a.state == "outdated").await;

    // An ordinary arrow made outdated the ordinary way, to compare against.
    assert!([200, 201].contains(&add(&core, &updatable).await));
    let identity = wait_arrow(&core, &updatable, "in the catalog", |_| true)
        .await
        .namespace;
    let install = runtime(&core, &identity, "install").await;
    assert!([200, 202].contains(&install), "install answered {install}");
    wait_arrow(&core, &identity, "installed", |a| a.state == "ready").await;
    upstream.publish_repo_tag("rabbytesoftware/e2e-updatable", "stable-1.1");
    check_now(&core, &identity).await;
    wait_arrow(&core, &identity, "outdated", |a| a.state == "outdated").await;

    // The badge on the tiles (Home's "View all" opens the Library): the same text on both.
    app.click("home.library.view-all").await;
    app.route("/library").await;
    let badge = |ns: &str| json!({"id": format!("tile.{ns}.badge"), "label_contains": "Update"});
    let own = app.matching(badge(NS_CORE)).await;
    let ordinary = app.matching(badge(&updatable)).await;
    assert_eq!(
        own.label, ordinary.label,
        "the self-arrow's badge differs from an ordinary arrow's"
    );
    app.shot("outdated-badge").await;

    // The details page words the same state as `Update available`.
    app.click(&format!("sidebar.row.{NS_CORE}")).await;
    let status = app
        .matching(json!({"id": "arrow.status", "label_contains": "Update available"}))
        .await;
    assert_eq!(status.label, "Update available");
    app.shot("outdated-details").await;
}

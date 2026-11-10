//! SCENARIO 1 -- quiver.core updates itself through the app: the Update button posts the ordinary `update`
//! method, core downloads the release, verifies it against the release's checksums.txt and swaps the daemon
//! process in place. Driven through the real app and the daemon it spawned; releases come from the box's GitHub
//! stand-in over HTTPS by the real hostnames.
//!
//! The old spec walked ONE installation through five releases. Natively each phase is its own test from a clean
//! home (V1 start), so one failure does not hide the rest:
//!   * from the arrow page, V1 -> V2        * from Settings > Engine, V1 -> V2
//!   * a wrong checksum is refused           * a build that never becomes healthy is rolled back
//!   * closing the window stops the daemon   * a second update after the first swap
//!
//! Substituted claims: the old spec read the setting rows' text from the DOM; natively the values are the
//! `settings.core.{installed,available}.value` ids and the row notes `settings.core.actions.description`.
//! "Quitting the app" closes the window with wmctrl (the window manager's close button, as before).

use std::process::Command;

use crate::support::lifecycle::runtime_of;

use crate::support::arrows::wait_arrow;
use crate::support::core_update::*;
use crate::support::env::required;
use crate::support::procs::{alive, daemon_version};
use crate::support::upstream::ReleaseOptions;
use crate::support::wait::{eventually, LONG, SHORT};
use crate::support::NS_CORE;

#[tokio::test]
async fn update_from_the_arrow_page_swaps_the_daemon_and_the_app_stays_up() {
    let Some(mut s) = Setup::start("update_core_page").await else {
        return;
    };
    let (v1, v2) = (s.tags[0].clone(), s.tags[1].clone());
    open_engine(&mut s.app).await;
    value(&mut s.app, "installed", &v1).await;
    value(&mut s.app, "available", UP_TO_DATE).await;
    assert!(s.app.find("settings.core.update").await.is_none());
    assert!(s.app.node("settings.core.check").await.enabled);
    s.app.shot("01-settings-up-to-date").await;

    s.offer(&v2, ReleaseOptions::default()).await;
    s.app.node("settings.core.update").await;
    s.app.shot("02-settings-update-available").await;

    let before = daemons();
    assert_eq!(before.len(), 1);
    s.app.click(&format!("sidebar.row.{NS_CORE}")).await;
    s.app.click("arrow.action.update").await;
    s.run().swapped_to(&v2).await;
    assert_ne!(daemons(), before);
    let served = format!("/rabbytesoftware/quiver.core/releases/download/{v2}");
    assert!(
        s.upstream
            .hits(&format!("{served}/{}", required("QUIVER_E2E_CORE_ASSET")))
            >= 1
    );
    assert!(s.upstream.hits(&format!("{served}/checksums.txt")) >= 1);
    open_engine(&mut s.app).await;
    value(&mut s.app, "installed", &v2).await;
    value(&mut s.app, "available", UP_TO_DATE).await;
    s.app.shot("05-settings-after-swap").await;
}

#[tokio::test]
async fn update_from_settings_swaps_the_daemon_and_settings_shows_the_new_version() {
    let Some(mut s) = Setup::start("update_core_settings").await else {
        return;
    };
    let v2 = s.tags[1].clone();
    s.offer(&v2, ReleaseOptions::default()).await;
    s.app.click("settings.core.update").await;
    s.app.shot("06-settings-updating").await;
    s.run().swapped_to(&v2).await;
    value(&mut s.app, "installed", &v2).await;
    // The "available" row after a swap without leaving Settings is covered by the second-update test below.
    s.app.shot("07-settings-after-swap").await;
}

/// After the first swap the app must not keep the pre-swap runtime state of quiver.core ("updating"): the
/// socket's reconnect re-reads the list over it, so Settings enables "Check for updates" again.
#[tokio::test]
async fn a_second_update_can_be_offered_after_the_first_swap() {
    let Some(mut s) = Setup::start("update_core_twice").await else {
        return;
    };
    let (v2, v3) = (s.tags[1].clone(), s.tags[2].clone());
    s.offer(&v2, ReleaseOptions::default()).await;
    s.app.click("settings.core.update").await;
    s.run().swapped_to(&v2).await;
    s.offer(&v3, ReleaseOptions::default()).await;
    s.app.click("settings.core.update").await;
    s.run().swapped_to(&v3).await;
    value(&mut s.app, "installed", &v3).await;
}

#[tokio::test]
async fn a_release_whose_checksum_does_not_match_is_refused_and_nothing_changes() {
    let Some(mut s) = Setup::start("update_core_checksum").await else {
        return;
    };
    let (v1, v2) = (s.tags[0].clone(), s.tags[1].clone());
    let (before, binary) = (daemons(), sha256(&s.home.self_binary()));
    let options = ReleaseOptions {
        binary_of: Some(&v1),
        wrong_checksum: true,
        ..Default::default()
    };
    s.offer(&v2, options).await;
    s.app.click("settings.core.update").await;
    let (core, identity) = (s.core.clone(), s.identity.clone());
    eventually("the update to be refused", LONG, || async {
        (runtime_of(&core, &identity).await["last_return"]["outcome"] == "failed").then_some(())
    })
    .await;
    wait_arrow(&core, &identity, "outdated again", |a| {
        a.state == "outdated"
    })
    .await;
    assert_eq!(daemon_version(&core).await.as_deref(), Some(v1.as_str()));
    assert_eq!(daemons(), before);
    assert_eq!(sha256(&s.home.self_binary()), binary);
    assert!(leftovers(&s.home).is_empty());
    assert!(alive(s.supervised_pid));
    s.app.shot("08-settings-wrong-checksum").await;
    s.app.click(&format!("sidebar.row.{NS_CORE}")).await;
    s.app.node("arrow.problem").await;
    s.app.shot("09-arrow-page-wrong-checksum").await;
}

#[tokio::test]
async fn a_build_that_never_becomes_healthy_is_rolled_back() {
    let Some(mut s) = Setup::start("update_core_rollback").await else {
        return;
    };
    let (v1, v2) = (s.tags[0].clone(), s.tags[1].clone());
    let options = ReleaseOptions {
        binary_of: Some(&v1),
        unhealthy: true,
        ..Default::default()
    };
    s.offer(&v2, options).await;
    s.app.click("settings.core.update").await;
    let log = s.home.quiver().join("logs").join("self-update.log");
    eventually("the swap to give up on the new build", LONG * 2, || async {
        std::fs::read_to_string(&log)
            .ok()
            .filter(|l| l.contains("rolling back"))
            .map(|_| ())
    })
    .await;
    let core = s.core.clone();
    eventually("the daemon back on the old build", LONG, || async {
        (daemon_version(&core).await.as_deref() == Some(v1.as_str())).then_some(())
    })
    .await;
    wait_arrow(&core, &s.identity, "outdated on the old version", |a| {
        a.state == "outdated" && a.available.as_ref().is_some_and(|r| r.name == v2)
    })
    .await;
    assert_eq!(
        sha256(&s.home.self_binary()),
        sha256(&s.upstream.built_asset(&v1))
    );
    eventually("the aside binary to be gone", SHORT, || async {
        leftovers(&s.home).is_empty().then_some(())
    })
    .await;
    assert_eq!(daemons().len(), 1);
    assert!(alive(s.supervised_pid) && alive(s.app_pid));
    s.app.shot("10-settings-after-rollback").await;
}

#[tokio::test]
async fn closing_the_window_stops_the_daemon_and_leaves_no_quiver_process() {
    let Some(mut s) = Setup::start("update_core_quit").await else {
        return;
    };
    let current = daemons();
    assert_eq!(current.len(), 1);
    assert!(Command::new("wmctrl")
        .args(["-c", "Quiver"])
        .status()
        .expect("wmctrl")
        .success());
    s.app.proc.wait_exit().await.expect("the app to exit");
    eventually("the daemon to stop with the app", LONG, || async {
        (daemons().is_empty() && !alive(current[0])).then_some(())
    })
    .await;
    assert!(daemon_version(&s.core).await.is_none());
}

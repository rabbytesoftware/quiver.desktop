//! SCENARIO 2 -- a process quiver.core supervises is not killed by quiver.core's own self-update, driven through
//! the real app and a real swap of the daemon process (quiver.core's own swap_test.go proves the same without
//! the app). Runs in the E2E box: the new release is published to its GitHub stand-in and the daemon resolves it
//! itself.

use crate::support::app::App;
use crate::support::arrows::{add, check_now, runtime, wait_arrow};
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::procs::{alive, daemon_version, kill};
use crate::support::upstream::Upstream;
use crate::support::wait::{eventually, LONG};
use crate::support::NS_CORE;

#[tokio::test]
async fn a_supervised_process_survives_the_daemon_replacing_itself() {
    let Some(env) = Env::load("self_update_while_running") else {
        return;
    };
    let upstream = Upstream::load();
    let tags = Upstream::core_tags();
    let (old, new) = (&tags[0], &tags[1]);
    let fixture = required("QUIVER_E2E_FIXTURE_NS");
    let home = Home::fresh(&env, "self_update_while_running");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    let self_ns = wait_arrow(&core, NS_CORE, "a settled quiver.core", |a| {
        a.state == "ready" || a.state == "outdated"
    })
    .await
    .namespace;
    assert!(self_ns.contains('@'), "{self_ns}");
    assert_eq!(daemon_version(&core).await.as_deref(), Some(old.as_str()));

    // A supervised arrow to protect.
    assert!([200, 201].contains(&add(&core, &fixture).await));
    let fixture_ns = wait_arrow(&core, &fixture, "in the catalog", |_| true)
        .await
        .namespace;
    let install = runtime(&core, &fixture_ns, "install").await;
    assert!([200, 202].contains(&install), "install answered {install}");
    wait_arrow(&core, &fixture_ns, "the supervised arrow installed", |a| {
        a.state == "ready"
    })
    .await;
    assert_eq!(runtime(&core, &fixture_ns, "execute").await, 202);
    let running = wait_arrow(&core, &fixture_ns, "the supervised arrow running", |a| {
        a.state == "running" && a.active_run.as_ref().and_then(|r| r.pid).unwrap_or(0) > 0
    })
    .await;
    let pid = running.active_run.and_then(|r| r.pid).expect("pid");

    // The daemon replaces itself while it runs.
    upstream.publish_core_release(new);
    assert_eq!(check_now(&core, &self_ns).await, 200);
    wait_arrow(&core, &self_ns, &format!("{new} found ahead"), |a| {
        a.available.as_ref().is_some_and(|r| &r.name == new)
    })
    .await;
    assert_eq!(runtime(&core, &self_ns, "update").await, 202);
    eventually(&format!("the daemon to report {new}"), LONG * 2, || async {
        (daemon_version(&core).await.as_deref() == Some(new.as_str())).then_some(())
    })
    .await;
    eventually("the new daemon to answer", LONG, || async {
        core.healthy().await.then_some(())
    })
    .await;
    wait_arrow(&core, &self_ns, "the new build settled", |a| {
        a.resolved_ref.as_deref() == Some(new.as_str()) && a.available.is_none()
    })
    .await;

    // RecordDetached clears Execution, so there is no active_run.pid left in the read model once Detached: ask
    // the OS about the process, then the read model about its state.
    let alive_after = alive(pid);
    if !alive_after {
        kill(pid);
    }
    assert!(alive_after, "supervised pid {pid} died with the old daemon");
    let after = wait_arrow(&core, &fixture_ns, "still known", |_| true).await;
    assert_eq!(after.state, "detached");
    kill(pid);

    // The window is still usable afterwards: same process, sidebar populated, the daemon reconnected.
    assert!(
        app.proc.pid().is_some_and(alive),
        "the app died with the old daemon"
    );
    app.node("sidebar.list").await;
    app.shot("after-self-update").await;
}

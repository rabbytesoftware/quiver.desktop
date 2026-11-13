//! SCENARIO 1 -- bootstrap from a genuinely empty QUIVER_HOME, driving the real shipped artifact rather than the
//! per-repo unit tests that only exercise pieces of this in isolation.

use std::os::unix::fs::PermissionsExt;

use crate::support::app::App;
use crate::support::arrows::{arrow, channels, library, wait_arrow};
use crate::support::daemon::Daemon;
use crate::support::env::{optional, Env};
use crate::support::home::Home;
use crate::support::wait::{eventually, LONG};
use crate::support::{NS_CORE, NS_DESKTOP};

#[tokio::test]
async fn a_clean_home_comes_up_self_installed() {
    let Some(env) = Env::load("bootstrap") else {
        return;
    };
    let home = Home::fresh(&env, "bootstrap");

    // The precondition, recorded BEFORE the app starts: by the first assertion the daemon has already installed.
    assert!(
        !home.self_binary().exists(),
        "the home must start without a self-installed Core"
    );

    home.show_own_components();
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    app.shot("bootstrap-ready").await;
    let core = Daemon::unix(home.socket());

    // quiver.core leaves a binary at <QUIVER_HOME>/self/quiver.
    let binary = home.self_binary();
    let meta = eventually("quiver.core to self-install", LONG, || async {
        std::fs::metadata(&binary).ok()
    })
    .await;
    assert!(
        meta.is_file() && meta.len() > 0,
        "{} is not a non-empty file",
        binary.display()
    );
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "{} is not executable",
        binary.display()
    );

    // quiver.core is its own user-installed arrow, filed under a selector.
    let (status, detail) = arrow(&core, NS_CORE).await;
    let detail = detail.expect("quiver.core is in the catalog");
    assert_eq!(status, 200);
    assert!(
        detail.namespace.starts_with(&format!("{NS_CORE}@")),
        "identity {}",
        detail.namespace
    );
    assert!(detail.user_installed);
    assert!(detail.resolved_ref.is_some_and(|r| !r.is_empty()));

    // quiver.desktop is filed under a channel of its repository, never a pin of its version.
    let names = channels(&core, NS_DESKTOP).await;
    assert!(
        !names.is_empty(),
        "quiver.desktop's repository lists no channel"
    );
    let channel = optional("QUIVER_E2E_BUILD_CHANNEL").unwrap_or_else(|| names[0].clone());
    let identity = format!("{NS_DESKTOP}@{channel}");
    // The announce is fire-and-forget on connect, so the row lands (and is adopted into the library) a moment after
    // the window is ready.
    let desktop = wait_arrow(
        &core,
        &identity,
        "to be announced and user-installed",
        |a| a.user_installed,
    )
    .await;
    assert_eq!(desktop.namespace, identity);
    assert_eq!(desktop.selector_kind.as_deref(), Some("channel"));
    assert!(desktop.user_installed);
    assert!(desktop.resolved_ref.is_some_and(|r| !r.is_empty()));
    let versions = library(&core)
        .await
        .into_iter()
        .find(|(ns, _)| ns == NS_DESKTOP)
        .map(|(_, v)| v);
    assert_eq!(versions, Some(vec![channel.clone()]));

    // The sidebar the user actually sees lists both self-arrows, by the name in each arrow's manifest.
    // Rows are keyed by the bare namespace: the catalog identity's selector is not part of the id.
    for (ns, name) in [(NS_CORE, "Quiver Core"), (NS_DESKTOP, "Quiver Desktop")] {
        let row = app.node(&format!("sidebar.row.{ns}")).await;
        assert_eq!(row.label, name);
    }
    app.shot("bootstrap-sidebar").await;
}

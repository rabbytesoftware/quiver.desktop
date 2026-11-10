//! SCENARIO (new) -- the desktop registers ITSELF in the catalog of its LOCAL daemon only. With a real
//! quiver.core: the local connection files quiver.desktop; then a REMOTE connection (a second daemon this test
//! starts on a TCP port) is added through the app's own Remote screen, and neither the remote daemon's catalog
//! nor its log shows a registration. Exactly one announcer exists in the app (`services/announce.rs`); this is
//! the proof end to end that a remote daemon is never made to know this desktop.

use std::process::Stdio;

use serde_json::json;
use tokio::process::{Child, Command};

use crate::support::app::App;
use crate::support::arrows::{arrow, library};
use crate::support::daemon::Daemon;
use crate::support::env::{optional, Env};
use crate::support::home::Home;
use crate::support::wait::{eventually, LONG};
use crate::support::NS_DESKTOP;

struct RemoteDaemon {
    _child: Child,
    api: Daemon,
    home: Home,
}

/// A second quiver.core on loopback TCP with a home of its own (the self-installed binary of the app's daemon).
async fn start_remote(env: &Env, binary: &std::path::Path, port: u16) -> RemoteDaemon {
    let home = Home::fresh(env, "auto_register_remote");
    let log = std::fs::File::create(home.root.join("daemon.out")).expect("daemon log");
    let child = Command::new(binary)
        .args(["daemon", "--host", &format!("tcp://127.0.0.1:{port}")])
        .envs(home.env())
        .stdout(Stdio::from(log.try_clone().expect("clone")))
        .stderr(Stdio::from(log))
        .kill_on_drop(true)
        .spawn()
        .expect("start the remote daemon");
    let api = Daemon::tcp(port);
    eventually("the remote daemon to answer", LONG, || async {
        api.healthy().await.then_some(())
    })
    .await;
    RemoteDaemon {
        _child: child,
        api,
        home,
    }
}

/// A one-time pairing code from a daemon's own loopback.
async fn mint_code(api: &Daemon) -> String {
    let reply = api
        .post("/v0/auth/pairing", None)
        .await
        .expect("mint a pairing code");
    reply.json()["code"]
        .as_str()
        .unwrap_or_else(|| panic!("no code in {}", reply.body))
        .to_string()
}

/// Redeems a code for a bearer token of a device named `device`.
async fn redeem(api: &Daemon, code: &str, device: &str) -> String {
    let body = json!({"code": code, "device_id": device, "label": "auto_register e2e"});
    let reply = api
        .post("/v0/auth/pairing/redeem", Some(body))
        .await
        .expect("redeem");
    reply.json()["token"]
        .as_str()
        .unwrap_or_else(|| panic!("no token in {}", reply.body))
        .to_string()
}

#[tokio::test]
async fn only_the_local_daemon_learns_about_the_desktop() {
    let Some(env) = Env::load("auto_register") else {
        return;
    };
    let port: u16 = optional("QUIVER_E2E_TCP_PORT")
        .and_then(|p| p.parse().ok())
        .unwrap_or(40299);
    let home = Home::fresh(&env, "auto_register");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let local = Daemon::unix(home.socket());

    // The LOCAL connection registered quiver.desktop (under a channel of its repository).
    let registered = eventually("the local daemon to list quiver.desktop", LONG, || async {
        library(&local)
            .await
            .into_iter()
            .find(|(ns, _)| ns == NS_DESKTOP)
    })
    .await;
    assert!(
        !registered.1.is_empty(),
        "no selector filed for quiver.desktop"
    );
    let identity = format!("{NS_DESKTOP}@{}", registered.1[0]);
    assert!(arrow(&local, &identity)
        .await
        .1
        .is_some_and(|a| a.user_installed));

    // A second daemon, remote to the app: its own home, TCP only.
    let binary = eventually("the self-installed core", LONG, || async {
        home.self_binary().exists().then(|| home.self_binary())
    })
    .await;
    let remote = start_remote(&env, &binary, port).await;
    let before = remote.api.get("/v0/arrow?user_installed=true").await;
    assert!(
        before.is_ok_and(|r| r.status == 401 || r.status == 200),
        "the remote daemon should answer"
    );

    // Add it as a remote connection through the Remote screen, pairing like a person would.
    app.click("sidebar.tab.remote").await;
    app.route("/remote").await;
    app.click("remote.add").await;
    app.node("remote.add.name").await;
    app.type_into("remote.add.name", "Lab").await;
    app.type_into("remote.add.url", &format!("http://127.0.0.1:{port}"))
        .await;
    app.click("remote.add.continue").await;
    app.node("remote.add.code").await;
    let ui_code = mint_code(&remote.api).await;
    app.type_into("remote.add.code", &ui_code).await;
    app.click("remote.add.submit").await;
    if app
        .bridge
        .wait_node("remote.add.code", false, 20_000)
        .await
        .is_err()
    {
        let out = std::fs::read_to_string(remote.home.root.join("daemon.out")).unwrap_or_default();
        let tail: Vec<&str> = out
            .lines()
            .rev()
            .filter(|l| !l.contains("GIN-debug"))
            .take(15)
            .collect();
        eprintln!("remote daemon output (newest first):\n{}", tail.join("\n"));
    }
    app.gone("remote.add.code").await;
    let row = app
        .matching(json!({"prefix": "remote.row.", "label_contains": "Lab"}))
        .await;
    app.shot("remote-added").await;

    // Make it the active connection and wait until the app is connected to it: the moment an announce, if any,
    // would have been fired.
    let id = row.id.trim_start_matches("remote.row.").to_string();
    assert!(
        !row.state.contains("selected"),
        "adding a connection must not switch to it"
    );
    app.click(&format!("remote.row.{id}.menu")).await;
    app.click(&format!("remote.row.{id}.switch")).await;
    app.signal(&format!("connection.ready:{id}")).await;
    app.shot("remote-active").await;

    // The remote daemon's catalog has no quiver.desktop: asked with a bearer token of a device of its own.
    let token = redeem(
        &remote.api,
        &mint_code(&remote.api).await,
        "auto-register-checker",
    )
    .await;
    let authed = remote.api.clone().with_bearer(&token);
    let catalog = authed.get("/v0/arrow").await.expect("remote catalog");
    assert_eq!(catalog.status, 200, "{}", catalog.body);
    assert!(
        !catalog.body.contains("quiver.desktop"),
        "the remote catalog lists the desktop: {}",
        catalog.body
    );
    assert!(library(&authed)
        .await
        .iter()
        .all(|(ns, _)| ns != NS_DESKTOP));

    // ...and neither does its request log (every request the daemon served is one JSON line on its stdout; a read of
    // the desktop's arrow is not a registration, a POST/PUT/DELETE naming it is). It must hold the pairing the app did, so an empty log cannot pass for a clean one.
    let served = std::fs::read_to_string(remote.home.root.join("daemon.out")).unwrap_or_default();
    assert!(
        served.contains("/v0/auth/pairing/redeem"),
        "the remote daemon's log does not show the app's pairing: {served}"
    );
    let writes: Vec<&str> = served
        .lines()
        .filter(|l| l.contains("quiver.desktop") && !l.contains(r#""method":"GET""#))
        .collect();
    assert!(
        writes.is_empty(),
        "the remote daemon was told about the desktop: {writes:?}"
    );

    // A detail read resolves from upstream whether or not the daemon holds the arrow; what it must not be is a
    // row the user installed.
    assert!(
        arrow(&authed, &identity)
            .await
            .1
            .is_none_or(|a| !a.user_installed),
        "the remote daemon filed {identity} as installed"
    );
}

//! SCENARIO -- the build indicator and the daemon console, driven through the real built app and the real
//! quiver.core it spawned. No mock: what is asserted is the wire contract of the console spec as the shipped
//! daemon honours it, and the window a person actually sees.
//!
//! The daemon under test must have the console (`features` has `console.v1`): build the box with a quiver.core
//! that includes it (QUIVER_CORE_DEV_PATH).
//!
//! Claims that cannot be checked natively, and what stands in:
//! * "the console is `inert` until opened": the native dock is not drawn at all while closed (`screen.console` is
//!   absent), which is the stronger form; asserted as absence.
//! * "expands a line to its raw JSON on click": the native row has no expanded view yet, so only the typed row is
//!   checked (its `log:<level>` kind and its text); see the report.
//! * the indicator's `title` tooltip: not exposed, the `sidebar.build` state (the lines it draws) is checked
//!   against the version the daemon reports instead.

use serde_json::json;

use crate::support::app::App;
use crate::support::daemon::Daemon;
use crate::support::env::Env;
use crate::support::home::Home;
use crate::support::wait::{eventually, LONG};

const LINE: &str = "console.line.";

/// Types a command the way a person does; returns once its echo (`> <line>`) is in the log pane.
async fn run(
    app: &mut App,
    line: &str,
) {
    app.enter_text("console.input", line).await;
    app.matching(
        json!({"prefix": LINE, "state_contains": "cmd", "label_contains": format!("> {line}")}),
    )
    .await;
}

async fn note(
    app: &mut App,
    text_part: &str,
) -> String {
    app.matching(
        json!({"prefix": LINE, "state_contains": "note:error", "label_contains": text_part}),
    )
    .await
    .label
}

/// How many times the daemon's own log file says it ran `command` to `code`: the pane only holds what streamed,
/// the file does not depend on that.
fn audits(
    home: &Home,
    command: &str,
    code: i64,
) -> usize {
    std::fs::read_to_string(home.daemon_log())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|r| {
            r["msg"] == "exec"
                && r["command"]
                    .as_str()
                    .is_some_and(|c| c.trim_start_matches("quiver ") == command)
                && r["code"] == code
        })
        .count()
}

async fn audited(
    home: &Home,
    command: &str,
    code: i64,
) {
    eventually(
        &format!("the daemon to audit `{command}` at code {code}"),
        LONG,
        || async { (audits(home, command, code) > 0).then_some(()) },
    )
    .await;
}

#[tokio::test]
async fn the_build_indicator_and_the_daemon_console() {
    let Some(env) = Env::load("console") else {
        return;
    };
    let home = Home::fresh(&env, "console");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());

    // The daemon advertises the console.
    let versions = core.get("/versions").await.expect("versions");
    assert_eq!(versions.status, 200);
    let features = versions.json()["features"].clone();
    assert!(
        features.to_string().contains("console.v1"),
        "features: {features}"
    );

    // Both builds in the rail, once the daemon has answered; the core line names the version the daemon reports.
    let build = app
        .matching(json!({"id": "sidebar.build", "state_contains": "core "}))
        .await;
    assert!(build.state.contains("app "), "indicator: {}", build.state);
    if let Some(version) = versions.json()["version"].as_str() {
        // The rail writes `stable-26.5.90` as the channel and the number: `stable 26.5.90`.
        let shown = version.replacen('-', " ", 1);
        assert!(
            build.state.contains(&shown),
            "indicator {:?} lacks the daemon's version {shown}",
            build.state
        );
    }
    app.shot("console-indicator").await;

    // Unreachable until opened: not drawn at all.
    assert!(
        app.find("screen.console").await.is_none(),
        "the console is drawn while closed"
    );

    // Opens from the indicator and shows the daemon's own log, typed.
    app.click("sidebar.build").await;
    app.node("screen.console").await;
    app.matching(json!({"prefix": LINE, "state_contains": "log:"}))
        .await;
    let rows = app.with_prefix(LINE).await;
    let levels: Vec<_> = rows
        .iter()
        .filter_map(|n| n.state.strip_prefix("log:"))
        .collect();
    assert!(!levels.is_empty());
    for level in levels.iter().take(20) {
        assert!(
            ["debug", "info", "warn", "error"].contains(level),
            "level {level}"
        );
    }
    app.shot("console-open").await;

    // Runs a command on the daemon and prints what it said.
    run(&mut app, "version").await;
    app.matching(
        json!({"prefix": LINE, "state_contains": "out:", "label_contains": "daemon stable"}),
    )
    .await;

    // Runs a read command to a clean exit; the daemon records the run in the log the console shows.
    run(&mut app, "list").await;
    audited(&home, "list", 0).await;
    app.matching(
        json!({"prefix": LINE, "state_contains": "log:", "label_contains": "quiver list"}),
    )
    .await;
    assert!(
        app.with_prefix(LINE)
            .await
            .iter()
            .all(|n| n.state != "note:error"),
        "a clean exit must say nothing extra"
    );

    // Refuses what the daemon does not offer, in the daemon's own words.
    run(&mut app, "daemon").await;
    let refused = note(&mut app, "Refused (403)").await;
    assert_eq!(
        refused,
        "Refused (403): command \"daemon\" is not available in the console"
    );
    run(&mut app, "add github.com/char2cs/crowbar").await;
    let refused = note(&mut app, "command \"add\"").await;
    assert_eq!(
        refused,
        "Refused (403): command \"add\" is not available in the console"
    );
    app.shot("console-refused").await;

    // The console does no quoting of its own: the daemon says so.
    run(&mut app, "install \"github.com/char2cs/crowbar\"").await;
    let refused = note(&mut app, "Refused (400)").await;
    assert!(
        refused.starts_with("Refused (400): line must be 1 to 1024 bytes")
            && refused.ends_with("quoting is not supported"),
        "{refused}"
    );

    // --server is accepted and runs on this daemon all the same: honoured, it would dial 127.0.0.1:1 and fail.
    let before = audits(&home, "version", 0);
    run(&mut app, "version --server tcp://127.0.0.1:1").await;
    eventually(
        "the daemon to audit the --server run of `version` at code 0",
        LONG,
        || async { (audits(&home, "version", 0) == before + 1).then_some(()) },
    )
    .await;

    // Never waits on a confirmation: a destructive command without --yes refuses at once.
    run(&mut app, "arrow remove github.com/example/not-there").await;
    app.matching(json!({"prefix": LINE, "label_contains": "requires --yes"}))
        .await;
    note(&mut app, "Exited with code 2").await;

    // The daemon logs each command it ran.
    audited(&home, "version", 0).await;
    audited(&home, "arrow remove", 2).await;

    // On the wire: what the console must never run, and what it rejects.
    for line in ["daemon", "self-update /tmp/x", "context list"] {
        let r = core
            .post("/v0/console/exec", Some(json!({ "line": line })))
            .await
            .expect("exec");
        assert_eq!(r.status, 403, "{line}");
    }
    let too_long = "x".repeat(1025);
    for line in [
        "install \"a b\"",
        "install 'a'",
        "list; id",
        "list | cat",
        "install $(id)",
        "   ",
        too_long.as_str(),
    ] {
        let r = core
            .post("/v0/console/exec", Some(json!({ "line": line })))
            .await
            .expect("exec");
        assert_eq!(r.status, 400, "{}", &line[..line.len().min(40)]);
    }

    // Lists the commands it will run, with no bare add.
    let commands = core.get("/v0/console/commands").await.expect("commands");
    assert_eq!(commands.status, 200);
    let names: Vec<String> = commands.json()["commands"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|c| {
                    c["path"]
                        .as_array()
                        .map(|p| {
                            p.iter()
                                .filter_map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        names.contains(&"install".to_string()) && names.contains(&"arrow add".to_string()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"add".to_string()) && !names.contains(&"daemon".to_string()),
        "{names:?}"
    );

    // Closes from the indicator.
    app.click("sidebar.build").await;
    app.gone("screen.console").await;
}

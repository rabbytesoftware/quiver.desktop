//! What the update_core scenarios share: the offered-version helpers and `Setup`, a clean home with quiver.core v1,
//! a supervised process running and the app up.

use std::process::Command;

use serde_json::json;

use crate::support::app::App;
use crate::support::arrows::wait_arrow;
use crate::support::daemon::Daemon;
use crate::support::env::{required, Env};
use crate::support::home::Home;
use crate::support::lifecycle::{execute, install};
use crate::support::procs::{alive, daemon_version, kill, pgrep};
use crate::support::upstream::{ReleaseOptions, Upstream};
use crate::support::wait::{eventually, LONG, SHORT};
use crate::support::NS_CORE;

pub const UP_TO_DATE: &str = "Up to date";

pub fn daemons() -> Vec<u32> {
    pgrep(&["-f", "quiver[^ ]* daemon"])
}

pub fn sha256(file: &std::path::Path) -> String {
    let out = Command::new("sha256sum")
        .arg(file)
        .output()
        .expect("sha256sum");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

pub fn leftovers(home: &Home) -> Vec<String> {
    std::fs::read_dir(home.self_binary().parent().expect("self dir"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("quiver.old-"))
        .collect()
}

pub async fn value(app: &mut App, id: &str, text: &str) {
    app.matching(json!({"id": format!("settings.core.{id}.value"), "label_contains": text}))
        .await;
}

pub async fn open_engine(app: &mut App) {
    app.click("sidebar.tab.settings").await;
    app.click("settings.tab.engine").await;
    app.node("settings.core.actions.row").await;
}

pub struct Run<'a> {
    pub home: &'a Home,
    pub core: &'a Daemon,
    pub upstream: &'a Upstream,
    pub identity: String,
    pub app_pid: u32,
    pub supervised_pid: u32,
}

impl Run<'_> {
    /// One daemon, on `tag`, whose binary is at the self path with nothing left beside it; the app and the
    /// supervised process lived through it.
    pub async fn swapped_to(&self, tag: &str) {
        eventually(&format!("the daemon to report {tag}"), LONG * 2, || async {
            (daemon_version(self.core).await.as_deref() == Some(tag)).then_some(())
        })
        .await;
        eventually("the new daemon to answer", LONG, || async {
            self.core.healthy().await.then_some(())
        })
        .await;
        wait_arrow(self.core, &self.identity, &format!("{tag} settled"), |a| {
            a.resolved_ref.as_deref() == Some(tag) && a.available.is_none() && a.state == "ready"
        })
        .await;
        assert_eq!(daemons().len(), 1, "daemons: {:?}", daemons());
        assert_eq!(
            sha256(&self.home.self_binary()),
            sha256(&self.upstream.built_asset(tag))
        );
        eventually("the previous binary to be removed", SHORT, || async {
            leftovers(self.home).is_empty().then_some(())
        })
        .await;
        assert!(self.home.socket().exists());
        assert!(
            alive(self.app_pid),
            "the app died with the daemon it replaced"
        );
        assert!(alive(self.supervised_pid), "the supervised process died");
    }
}

pub struct Setup {
    pub app: App,
    pub core: Daemon,
    pub home: Home,
    pub upstream: Upstream,
    pub tags: Vec<String>,
    pub identity: String,
    pub app_pid: u32,
    pub supervised_pid: u32,
}

impl Setup {
    /// The app on a clean home with quiver.core v1, a supervised process running, and the Engine settings open.
    pub async fn start(name: &str) -> Option<Self> {
        let env = Env::load("update_core")?;
        let home = Home::fresh(&env, name);
        home.show_own_components();
        let mut app = App::launch(&env, &home).await;
        app.wait_ready().await;
        let core = Daemon::unix(home.socket());
        let tags = Upstream::core_tags();
        let identity = wait_arrow(&core, NS_CORE, "a settled quiver.core", |a| {
            a.state == "ready" || a.state == "outdated"
        })
        .await
        .namespace;
        assert_eq!(
            daemon_version(&core).await.as_deref(),
            Some(tags[0].as_str())
        );
        // A supervised process the swaps must not kill.
        let supervised = install(&core, &required("QUIVER_E2E_FIXTURE_NS")).await;
        execute(&core, &supervised).await;
        let running = wait_arrow(&core, &supervised, "running", |a| {
            a.state == "running" && a.active_run.as_ref().and_then(|r| r.pid).unwrap_or(0) > 0
        })
        .await;
        let app_pid = app.proc.pid().expect("app pid");
        Some(Self {
            app,
            core,
            home,
            upstream: Upstream::load(),
            tags,
            identity,
            app_pid,
            supervised_pid: running.active_run.and_then(|r| r.pid).expect("pid"),
        })
    }

    pub fn run(&self) -> Run<'_> {
        Run {
            home: &self.home,
            core: &self.core,
            upstream: &self.upstream,
            identity: self.identity.clone(),
            app_pid: self.app_pid,
            supervised_pid: self.supervised_pid,
        }
    }

    /// Settings > Engine with the update rows loaded; publishes `tag`, checks and waits for it to be offered.
    pub async fn offer(&mut self, tag: &str, options: ReleaseOptions<'_>) {
        self.upstream.publish_core_release_with(tag, options);
        open_engine(&mut self.app).await;
        self.app.click("settings.core.check").await;
        value(&mut self.app, "available", tag).await;
    }
}

impl Drop for Setup {
    fn drop(&mut self) {
        kill(self.supervised_pid);
    }
}

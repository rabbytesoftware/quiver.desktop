//! The app under test, launched with the bridge on a wiped home, and the waits a scenario builds on.
//! Every wait is on a named signal or on an element appearing/disappearing: nothing here sleeps.

use std::path::PathBuf;

use quiver_e2e::app::AppProcess;
use quiver_e2e::client::Bridge;
use quiver_e2e::protocol::Node;

use super::env::Env;
use super::home::Home;

/// Generous: a cold daemon boot plus the first frames in software rendering.
pub const WAIT_MS: u64 = 90_000;

pub struct App {
    pub proc: AppProcess,
    pub bridge: Bridge,
    evidence: Option<PathBuf>,
}

impl App {
    /// Starts the app against `home` and connects the bridge.
    pub async fn launch(env: &Env, home: &Home) -> Self {
        let proc = AppProcess::launch(&env.app_binary, &home.env())
            .await
            .expect("launch the app");
        let bridge = proc.bridge().await.expect("connect to the bridge");
        Self {
            proc,
            bridge,
            evidence: env.evidence_dir(),
        }
    }

    /// The window is up, the daemon answered and the sidebar has settled out of its loading state.
    pub async fn wait_ready(&mut self) {
        self.signal("core.ready").await;
        self.node("sidebar.list").await;
    }

    pub async fn signal(&mut self, name: &str) {
        if let Err(e) = self.bridge.wait_signal(name, WAIT_MS).await {
            self.fail(&format!("signal {name}: {e}")).await;
        }
    }

    pub async fn route(&mut self, path: &str) {
        self.signal(&format!("route:{path}")).await;
    }

    /// Waits for `id` to be painted and returns it.
    pub async fn node(&mut self, id: &str) -> Node {
        match self.bridge.wait_node(id, true, WAIT_MS).await {
            Ok(node) => node,
            Err(e) => self.fail(&format!("node {id}: {e}")).await,
        }
    }

    /// Fails the test with what the app shows right now: every painted id (with its state), the app's state dump
    /// and its last log lines, which is what a timeout needs to be understood.
    pub async fn fail(&mut self, what: &str) -> ! {
        self.shot("FAILED").await;
        let tree = self
            .bridge
            .tree()
            .await
            .map(|t| t.nodes)
            .unwrap_or_default();
        let ids: Vec<String> = tree
            .iter()
            .map(|n| format!("{} [{}] {:?}", n.id, n.state, n.label))
            .collect();
        let state = self.bridge.state().await.unwrap_or_default();
        let logs = self.bridge.logs(40).await.unwrap_or_default();
        panic!(
            "{what}\npainted:\n  {}\nstate: {state}\nlast logs:\n  {}",
            ids.join("\n  "),
            logs.join("\n  ")
        );
    }

    /// Waits for a node by filter (`id` or `prefix`, `label_contains`, `state_contains`) and returns it.
    pub async fn matching(&mut self, filter: serde_json::Value) -> Node {
        match self
            .bridge
            .wait_matching(filter.clone(), true, WAIT_MS)
            .await
        {
            Ok(node) => node,
            Err(e) => self.fail(&format!("node {filter}: {e}")).await,
        }
    }

    pub async fn gone(&mut self, id: &str) {
        if let Err(e) = self.bridge.wait_node(id, false, WAIT_MS).await {
            self.fail(&format!("{id} never went away: {e}")).await;
        }
    }

    pub async fn find(&mut self, id: &str) -> Option<Node> {
        self.bridge.find(id).await.expect("ui.find")
    }

    pub async fn click(&mut self, id: &str) {
        // Enabled, not just painted: a button waits on form state that follows the typing by a frame.
        self.matching(serde_json::json!({"id": id, "enabled": true}))
            .await;
        if let Err(e) = self.bridge.click(id).await {
            self.fail(&format!("click {id}: {e}")).await;
        }
    }

    /// Every painted element whose id starts with `prefix`.
    pub async fn with_prefix(&mut self, prefix: &str) -> Vec<Node> {
        self.bridge
            .tree()
            .await
            .expect("ui.tree")
            .nodes
            .into_iter()
            .filter(|n| n.id.starts_with(prefix))
            .collect()
    }

    /// Clicks the field `id`, waits until it reports `[focused]` (the click is an OS event the app handles
    /// asynchronously: typing before it landed would go nowhere) and types `text`.
    pub async fn type_into(&mut self, id: &str, text: &str) {
        self.click(id).await;
        self.matching(serde_json::json!({"id": id, "state_contains": "[focused]"}))
            .await;
        if let Err(e) = self.bridge.type_text(None, text).await {
            self.fail(&format!("type into {id}: {e}")).await;
        }
    }

    /// [`App::type_into`], then enter.
    pub async fn enter_text(&mut self, id: &str, text: &str) {
        self.type_into(id, text).await;
        self.bridge.key("enter").await.expect("press enter");
    }

    /// Saves what the window shows next to the test's results: a green run leaves the screens its assertions were
    /// made against.
    pub async fn shot(&mut self, name: &str) {
        let Some(dir) = &self.evidence else { return };
        if let Ok(png) = self.bridge.screenshot_png().await {
            let _ = std::fs::write(dir.join(format!("{name}.png")), png);
        }
    }
}

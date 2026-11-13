//! The arrow API of the daemon, read directly (not through the app).

use serde::Deserialize;
use serde_json::{json, Value};

use super::daemon::{enc, Daemon};
use super::wait::{eventually, LONG};

/// `GET /v0/arrow/:ns` (the fields the scenarios assert on).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ArrowDetail {
    /// The catalog identity, `namespace@selector`, always.
    pub namespace: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub user_installed: bool,
    pub selector_kind: Option<String>,
    pub resolved_ref: Option<String>,
    /// The row's own record of what is ahead, set by the version check.
    pub available: Option<Ref>,
    pub active_run: Option<ActiveRun>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ref {
    #[serde(rename = "ref")]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActiveRun {
    pub pid: Option<u32>,
}

pub async fn arrow(
    d: &Daemon,
    ns: &str,
) -> (u16, Option<ArrowDetail>) {
    match d.get(&format!("/v0/arrow/{}", enc(ns))).await {
        Ok(r) => (r.status, r.data()),
        Err(_) => (0, None),
    }
}

/// Re-reads the arrow until `ok` holds; panics with the last detail seen.
pub async fn wait_arrow(
    d: &Daemon,
    ns: &str,
    what: &str,
    ok: impl Fn(&ArrowDetail) -> bool,
) -> ArrowDetail {
    eventually(&format!("{ns}: {what}"), LONG, || async {
        arrow(d, ns).await.1.filter(|a| ok(a))
    })
    .await
}

/// `GET /v0/arrow/:ns/channels`: the channel names the repository publishes, default first.
pub async fn channels(
    d: &Daemon,
    ns: &str,
) -> Vec<String> {
    let reply = d
        .get(&format!("/v0/arrow/{}/channels", enc(ns)))
        .await
        .expect("channels");
    assert_eq!(reply.status, 200, "channels of {ns}: {}", reply.body);
    reply.json()["channels"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| c["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// `GET /v0/arrow?user_installed=true`: `[(namespace, [selector])]`, one version per catalog identity.
pub async fn library(d: &Daemon) -> Vec<(String, Vec<String>)> {
    let reply = d
        .get("/v0/arrow?user_installed=true")
        .await
        .expect("library");
    let items: Vec<Value> = reply.data().unwrap_or_default();
    items
        .iter()
        .map(|i| {
            let versions = i["versions"].as_array().cloned().unwrap_or_default();
            (
                i["namespace"].as_str().unwrap_or_default().to_string(),
                versions
                    .iter()
                    .filter_map(|v| v["ref"].as_str().map(String::from))
                    .collect(),
            )
        })
        .collect()
}

/// What the Settings button runs: `PATCH /v0/arrow/:ns` re-resolves the row against a fresh snapshot.
///
/// The daemon re-resolves against upstream before it answers, and on a loaded machine it sometimes drops the
/// connection mid-request ("connection reset by peer", nothing logged for it). The call is idempotent, so a reset
/// is asked again (bounded) rather than failing the scenario on a daemon that merely was slow.
pub async fn check_now(
    d: &Daemon,
    ns: &str,
) -> u16 {
    let path = format!("/v0/arrow/{}", enc(ns));
    let mut last = None;
    for _ in 0..3 {
        match d.patch(&path, json!({})).await {
            Ok(reply) => return reply.status,
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => last = Some(e),
            Err(e) => panic!("check now: {e}"),
        }
    }
    panic!("check now: {:?}", last)
}

pub async fn add(
    d: &Daemon,
    ns: &str,
) -> u16 {
    d.post(&format!("/v0/arrow/{}", enc(ns)), None)
        .await
        .expect("add arrow")
        .status
}

/// POST `/v0/runtime/:ns/<verb>` with an empty body; the status the daemon answered.
pub async fn runtime(
    d: &Daemon,
    ns: &str,
    verb: &str,
) -> u16 {
    d.post(&format!("/v0/runtime/{}/{verb}", enc(ns)), Some(json!({})))
        .await
        .expect("runtime call")
        .status
}

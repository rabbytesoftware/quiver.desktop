//! A daemon that answers the calls an announce makes and records them.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use http::{Request, Response};

use super::{enc, LIST, NS};
use crate::transport::{Transport, WsStream};
use crate::Error;

/// How a call fails.
pub(super) enum Failure {
    /// The daemon answers with this error status.
    Status(u16),
    /// Nothing answers.
    Down,
}

/// One quiver.desktop row in the library.
pub(super) struct Row {
    pub(super) reference: String,
    pub(super) resolved: String,
}

pub(super) struct Daemon {
    rows: Vec<Row>,
    channels: Vec<String>,
    failing: HashMap<String, Failure>,
    calls: Mutex<Vec<String>>,
    bodies: Mutex<Vec<(String, serde_json::Value)>>,
}

impl Daemon {
    pub(super) fn new() -> Self {
        Self {
            rows: Vec::new(),
            channels: vec!["nightly-rolling".into()],
            failing: HashMap::new(),
            calls: Mutex::new(Vec::new()),
            bodies: Mutex::new(Vec::new()),
        }
    }

    /// A row whose resolved ref is the ref itself.
    pub(super) fn with_row(mut self, reference: &str) -> Self {
        self.rows.push(Row {
            reference: reference.into(),
            resolved: reference.into(),
        });
        self
    }

    pub(super) fn with_resolved_row(mut self, reference: &str, resolved: &str) -> Self {
        self.rows.push(Row {
            reference: reference.into(),
            resolved: resolved.into(),
        });
        self
    }

    pub(super) fn with_channels(mut self, channels: &[&str]) -> Self {
        self.channels = channels.iter().map(|c| c.to_string()).collect();
        self
    }

    pub(super) fn failing(mut self, call: String, how: Failure) -> Self {
        self.failing.insert(call, how);
        self
    }

    /// Every call, in the order it was made.
    pub(super) fn all_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// Every mutation, in the order it was made.
    pub(super) fn calls(&self) -> Vec<String> {
        self.all_calls()
            .into_iter()
            .filter(|c| !c.starts_with("GET"))
            .collect()
    }

    pub(super) fn deletes(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|c| c.starts_with("DELETE"))
            .collect()
    }

    /// The JSON body sent with the call that ended in `suffix`.
    pub(super) fn body_of(&self, suffix: &str) -> Option<serde_json::Value> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .find(|(call, _)| call.ends_with(suffix))
            .map(|(_, body)| body.clone())
    }

    fn envelope(data: serde_json::Value) -> Response<Vec<u8>> {
        let body = serde_json::json!({ "success": true, "error": null, "data": data });
        Response::builder()
            .status(200)
            .body(body.to_string().into_bytes())
            .unwrap()
    }

    fn failure(status: u16) -> Response<Vec<u8>> {
        let body = serde_json::json!({ "success": false, "error": "refused" });
        Response::builder()
            .status(status)
            .body(body.to_string().into_bytes())
            .unwrap()
    }
}

#[async_trait]
impl Transport for Daemon {
    async fn request(&self, req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        let path = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_default();
        let call = format!("{} {path}", req.method());
        self.calls.lock().unwrap().push(call.clone());
        if !req.body().is_empty() {
            let body = serde_json::from_slice(req.body()).unwrap();
            self.bodies.lock().unwrap().push((call.clone(), body));
        }
        match self.failing.get(&call) {
            Some(Failure::Status(status)) => return Ok(Self::failure(*status)),
            Some(Failure::Down) => return Err(Error::Connect("down".into())),
            None => {}
        }
        if call == format!("GET /v0/arrow/{}/channels", enc(NS)) {
            let channels: Vec<_> = self
                .channels
                .iter()
                .map(|name| serde_json::json!({ "name": name, "kind": "pointer", "latest": name }))
                .collect();
            return Ok(Self::envelope(serde_json::json!({ "channels": channels })));
        }
        if call == format!("GET {LIST}") {
            let versions: Vec<_> = self
                .rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "ref": r.reference, "resolved_ref": r.resolved, "state": "absent"
                    })
                })
                .collect();
            return Ok(Self::envelope(serde_json::json!([
                { "namespace": NS, "name": "Quiver", "description": "", "tags": null, "versions": versions },
                {
                    "namespace": "github.com/char2cs/crowbar", "name": "crowbar", "description": "",
                    "tags": [], "versions": [{ "ref": "stable-26.5", "resolved_ref": "stable-26.5", "state": "ready" }]
                }
            ])));
        }
        Ok(Response::builder().status(201).body(Vec::new()).unwrap())
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
        Err(Error::Connect("no ws".into()))
    }
}

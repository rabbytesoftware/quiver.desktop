//! Engine tab state: the daemon's config (`GET /v0/config`) and what happened to the last patch.

use crate::arrow_details::invoke::ActionFailure;
use serde_json::Value;

/// A key core refused (or corrected), with its reason.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rejection {
    pub key: String,
    pub message: String,
}

fn rejections(value: Option<&Value>) -> Vec<Rejection> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|r| {
                    Some(Rejection {
                        key: r.get("key")?.as_str()?.to_owned(),
                        message: r.get("message")?.as_str()?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The three views of the config plus the restart list. Sections are kept as raw JSON: the tab reads a
/// few, but every other section (`vault`, `search`, `api`, ...) must still survive a round trip.
#[derive(Clone, PartialEq, Debug)]
pub struct ConfigView {
    pub running: Value,
    pub configured: Value,
    pub defaults: Value,
    pub restart_required: Vec<String>,
    /// Settings the daemon already corrected on disk.
    pub corrected: Vec<Rejection>,
}

impl ConfigView {
    pub fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            running: value.get("running")?.clone(),
            configured: value.get("configured")?.clone(),
            defaults: value.get("defaults")?.clone(),
            restart_required: value
                .get("restart_required")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|k| k.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            corrected: rejections(value.get("corrected")),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PatchResult {
    pub applied: Vec<String>,
    pub rejected: Vec<Rejection>,
}

/// Core answers a fully-rejected patch with `{ success: false, error: null, data: { applied: [],
/// rejected: [...] } }` on a 422, confirmed against a real daemon. `success` reads false even though the
/// per-key rejection reasons are real data, so this endpoint reads the envelope itself: a well-formed
/// `{ applied, rejected }` payload is data regardless of `success` or HTTP status, and a failure is
/// reported only when the body is not shaped like a patch result.
pub fn parse_patch_response(
    status: u16,
    status_text: &str,
    body: Option<&Value>,
) -> Result<PatchResult, ActionFailure> {
    if let Some(data) = body.and_then(|b| b.get("data")) {
        if let (Some(applied), Some(_)) = (
            data.get("applied").and_then(Value::as_array),
            data.get("rejected").and_then(Value::as_array),
        ) {
            return Ok(PatchResult {
                applied: applied
                    .iter()
                    .filter_map(|k| k.as_str().map(str::to_owned))
                    .collect(),
                rejected: rejections(data.get("rejected")),
            });
        }
    }
    let message = body
        .and_then(|b| b.get("error"))
        .and_then(Value::as_str)
        .map_or_else(|| format!("{status} {status_text}"), str::to_owned);
    Err(ActionFailure {
        status: Some(status),
        message,
    })
}

/// A port field entry: blank or non-integer is never sent (there is nothing valid to patch) and the
/// field reverts to what the daemon actually holds.
pub fn parse_port_entry(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // `Number(trimmed)` accepts "49152.0" as the integer 49152; whole floats are integers here too.
    let n: f64 = trimmed.parse().ok()?;
    (n.is_finite() && n.fract() == 0.0 && n.abs() < 9.0e15).then_some(n as i64)
}

/// The patch for one port field.
pub fn port_patch(key: &str, port: i64) -> Value {
    serde_json::json!({ "netbridge": { key: port } })
}

#[derive(Clone, PartialEq, Debug)]
pub struct EngineState {
    pub view: Option<ConfigView>,
    pub rejected: Vec<Rejection>,
    pub loading: bool,
    /// Owned by `load` alone: a failure means there is nothing to show at all, so the whole panel is a
    /// retry screen.
    pub error: Option<String>,
    /// Owned by `patch` alone: a failure happens with a perfectly good `view` already on screen, so it
    /// renders inline. Kept separate from `error` so a momentary hiccup on one toggle cannot wipe out
    /// every row in the panel.
    pub patch_error: Option<String>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            view: None,
            rejected: Vec::new(),
            loading: true,
            error: None,
            patch_error: None,
        }
    }
}

impl EngineState {
    /// Also clears `rejected` and `patch_error`: leaving Engine and coming back re-runs the load while a
    /// rejection (or a stale patch failure) from the previous visit is still in the state. Without this a
    /// row keeps reporting a value as refused long after the state that caused it is gone.
    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
        self.rejected.clear();
        self.patch_error = None;
    }

    pub fn loaded(&mut self, result: Result<ConfigView, String>) {
        self.loading = false;
        match result {
            Ok(view) => self.view = Some(view),
            Err(message) => self.error = Some(message),
        }
    }

    pub fn begin_patch(&mut self) {
        self.loading = true;
        self.patch_error = None;
    }

    /// Re-reads after a patch rather than merging the response in: the daemon owns `restart_required`
    /// and `corrected`, and recomputing either here would be guessing at what it already told us.
    pub fn patched(&mut self, result: Result<(Vec<Rejection>, ConfigView), String>) {
        self.loading = false;
        match result {
            Ok((rejected, view)) => {
                self.rejected = rejected;
                self.view = Some(view);
            }
            Err(message) => self.patch_error = Some(message),
        }
    }

    /// Why the daemon refused `key`, if it did.
    pub fn why(&self, key: &str) -> Option<&str> {
        self.rejected
            .iter()
            .find(|r| r.key == key)
            .map(|r| r.message.as_str())
    }

    /// After a port patch: a refused value must not linger on screen looking accepted.
    pub fn port_was_rejected(&self, key: &str) -> bool {
        self.why(&format!("netbridge.{key}")).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn view() -> ConfigView {
        ConfigView::from_json(&json!({
            "running": {"logger": {"level": "info"}},
            "configured": {"logger": {"level": "info"}},
            "defaults": {"logger": {"level": "info"}},
            "restart_required": ["netbridge.ephemeral_port_start"],
            "corrected": [{"key": "logger.level", "message": "was trace"}],
        }))
        .unwrap()
    }

    #[test]
    fn the_view_keeps_all_three_configs_the_restart_list_and_corrections() {
        let v = view();
        assert_eq!(v.restart_required, ["netbridge.ephemeral_port_start"]);
        assert_eq!(
            v.corrected,
            [Rejection {
                key: "logger.level".into(),
                message: "was trace".into()
            }]
        );
        assert!(ConfigView::from_json(&json!({"running": {}})).is_none());
    }

    #[test]
    fn a_fully_rejected_patch_is_data_not_an_error() {
        let body = json!({"success": false, "error": null, "data": {"applied": [], "rejected": [{"key": "logger.level", "message": "unusable log level"}]}});
        let result = parse_patch_response(422, "Unprocessable", Some(&body)).unwrap();
        assert!(result.applied.is_empty());
        assert_eq!(result.rejected[0].key, "logger.level");
    }

    #[test]
    fn a_body_that_is_not_a_patch_result_is_an_error_with_the_envelopes_message_or_the_status_line()
    {
        let body = json!({"success": false, "error": "daemon exploded"});
        let err = parse_patch_response(500, "Server Error", Some(&body)).unwrap_err();
        assert_eq!(
            (err.status, err.message.as_str()),
            (Some(500), "daemon exploded")
        );
        let err = parse_patch_response(502, "Bad Gateway", None).unwrap_err();
        assert_eq!(err.message, "502 Bad Gateway");
        let half = json!({"data": {"applied": []}});
        assert!(parse_patch_response(200, "OK", Some(&half)).is_err());
    }

    #[test]
    fn a_patch_keeps_the_applied_keys_and_the_rejected_ones() {
        let body = json!({"success": true, "data": {"applied": ["netbridge.ephemeral_port_start"], "rejected": [{"key": "logger.level", "message": "x"}]}});
        let result = parse_patch_response(200, "OK", Some(&body)).unwrap();
        assert_eq!(result.applied, ["netbridge.ephemeral_port_start"]);
        assert_eq!(result.rejected.len(), 1);
    }

    #[test]
    fn a_load_error_and_a_patch_error_are_separate_and_the_view_survives_the_latter() {
        let mut state = EngineState::default();
        state.loaded(Ok(view()));
        state.begin_patch();
        state.patched(Err("nope".into()));
        assert_eq!(state.patch_error.as_deref(), Some("nope"));
        assert!(state.error.is_none() && state.view.is_some() && !state.loading);
        let mut fresh = EngineState::default();
        fresh.begin_load();
        fresh.loaded(Err("boom".into()));
        assert_eq!(fresh.error.as_deref(), Some("boom"));
        assert!(fresh.view.is_none());
    }

    #[test]
    fn loading_again_clears_a_stale_rejection_and_patch_error() {
        let mut state = EngineState::default();
        state.loaded(Ok(view()));
        state.patched(Ok((
            vec![Rejection {
                key: "logger.level".into(),
                message: "bad".into(),
            }],
            view(),
        )));
        state.patch_error = Some("old".into());
        assert_eq!(state.why("logger.level"), Some("bad"));
        state.begin_load();
        assert!(state.rejected.is_empty() && state.patch_error.is_none() && state.error.is_none());
    }

    #[test]
    fn a_fully_rejected_patch_populates_rejections_without_setting_an_error() {
        let mut state = EngineState::default();
        state.patched(Ok((
            vec![Rejection {
                key: "netbridge.ephemeral_port_end".into(),
                message: "too low".into(),
            }],
            view(),
        )));
        assert!(state.error.is_none() && state.patch_error.is_none());
        assert!(state.port_was_rejected("ephemeral_port_end"));
        assert!(!state.port_was_rejected("ephemeral_port_start"));
    }

    #[test]
    fn port_entries_are_integers_or_nothing() {
        assert_eq!(parse_port_entry(" 27015 "), Some(27015));
        assert_eq!(parse_port_entry("49152.0"), Some(49152));
        for bad in ["", "  ", "12.5", "abc", "NaN", "1e400"] {
            assert_eq!(parse_port_entry(bad), None, "{bad:?}");
        }
        assert_eq!(
            port_patch("ephemeral_port_start", 27015),
            json!({"netbridge": {"ephemeral_port_start": 27015}})
        );
    }
}

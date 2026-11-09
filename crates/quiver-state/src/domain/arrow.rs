use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArrowState {
    Absent,
    Installing,
    Updating,
    Ready,
    Running,
    Stopping,
    Draining,
    Detached,
    Uninstalling,
    Removed,
    Outdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// `type` stays a bare `String`: it comes off the wire from live/historical runs, and existing
/// fixtures emit values outside the manifest step kinds (`"exec"`), so callers must not assume membership.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepProgress {
    pub index: u32,
    pub title: String,
    pub status: StepStatus,
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Why a completed step did nothing (e.g. an `auto` expose that found nothing). Not a failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceMode {
    Listen,
    Static,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowSurface {
    pub mode: SurfaceMode,
    pub path: String,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveRun {
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub steps: Vec<StepProgress>,
    /// Absent on a daemon that predates arrow apps, or when the run serves no page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<ArrowSurface>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Failed,
    Cancelled,
}

/// The reactive, WS-driven shape: quiver.core's runtime-update frame omits `steps` here (a push on
/// every transition carrying full step history would be wasteful), so do not widen it to match
/// [`LastReturnDetail`], which only the one-time detail fetch can provide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastReturn {
    pub method: String,
    pub outcome: Outcome,
}

/// The detail endpoint's richer `last_return`: `variables`/`steps` let the UI show exactly *why* a
/// run failed (the failed step's own `error`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LastReturnDetail {
    pub method: String,
    pub outcome: Outcome,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub steps: Vec<StepProgress>,
}

/// Where an arrow's manifest came from. `Declared`: the repository ships an `ARROW.md` /
/// `arrow.yaml`. `Inferred`: quiver.core's Fletcher built one from the repository's release downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArrowOrigin {
    Declared,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceConfidence {
    High,
    Medium,
    Low,
}

/// An older daemon that sends none, or a newer one that sends a value this build does not know,
/// reads as a declared arrow rather than a broken one.
pub fn parse_arrow_origin(value: Option<&str>) -> ArrowOrigin {
    if value == Some("inferred") {
        ArrowOrigin::Inferred
    } else {
        ArrowOrigin::Declared
    }
}

pub fn parse_inference_confidence(value: Option<&str>) -> Option<InferenceConfidence> {
    match value? {
        "high" => Some(InferenceConfidence::High),
        "medium" => Some(InferenceConfidence::Medium),
        "low" => Some(InferenceConfidence::Low),
        _ => None,
    }
}

/// Only an explicit `Inferred` counts: anything else, including absent, is a declared manifest.
pub fn is_inferred(origin: Option<ArrowOrigin>) -> bool {
    origin == Some(ArrowOrigin::Inferred)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrowEntry {
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub version: String,
    pub state: ArrowState,
    pub active_run: Option<ActiveRun>,
    pub last_return: Option<LastReturn>,
    /// When this arrow's `execute` last completed successfully; `None` is the "never used" case.
    pub last_used_at: Option<String>,
    pub origin: Option<ArrowOrigin>,
    pub confidence: Option<InferenceConfidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeUpdate {
    pub namespace: String,
    pub state: ArrowState,
    pub active_run: Option<ActiveRun>,
    pub last_return: Option<LastReturn>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_reads_unknown_as_declared() {
        for (wire, expected) in [
            (Some("inferred"), ArrowOrigin::Inferred),
            (Some("declared"), ArrowOrigin::Declared),
            (None, ArrowOrigin::Declared),
            (Some(""), ArrowOrigin::Declared),
            (Some("something-newer"), ArrowOrigin::Declared),
        ] {
            assert_eq!(parse_arrow_origin(wire), expected);
        }
    }

    #[test]
    fn confidence_drops_unknown() {
        assert_eq!(
            parse_inference_confidence(Some("high")),
            Some(InferenceConfidence::High)
        );
        assert_eq!(
            parse_inference_confidence(Some("medium")),
            Some(InferenceConfidence::Medium)
        );
        assert_eq!(
            parse_inference_confidence(Some("low")),
            Some(InferenceConfidence::Low)
        );
        for wire in [None, Some(""), Some("certain")] {
            assert_eq!(parse_inference_confidence(wire), None);
        }
    }

    #[test]
    fn inferred_only_when_explicit() {
        assert!(is_inferred(Some(ArrowOrigin::Inferred)));
        assert!(!is_inferred(Some(ArrowOrigin::Declared)));
        assert!(!is_inferred(None));
    }
}

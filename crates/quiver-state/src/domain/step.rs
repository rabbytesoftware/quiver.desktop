use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::arrow::ArrowState;

/// A value that can vary per OS/arch -- quiver.core's `step.Overrideable[T]`. A bare scalar on the
/// wire when nothing overrides it; an object keyed by `"os/arch"` strings (plus a mandatory
/// `default`) when it does. Kept as-is rather than resolved for a platform: by the time a step is
/// inside one [`ArrowTarget`] it is already platform-scoped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Overridable<T> {
    Plain(T),
    PerPlatform(BTreeMap<String, T>),
}

/// quiver.core's `step.SignalKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalKind {
    Graceful,
    Kill,
    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepBase {
    pub title: String,
    /// Omitted on the wire for `dependencies` steps -- core's own `MarshalJSON` never writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_on_failure: Option<bool>,
}

/// A step declared on a not-yet-run method or lifecycle action: the full raw definition core's
/// manifest carries per type. Not the runtime progress shape (`StepProgress`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ArrowStepDefinition {
    Run {
        #[serde(flatten)]
        base: StepBase,
        command: Overridable<String>,
        elevated: Overridable<bool>,
        timeout: Overridable<String>,
    },
    Fetch {
        #[serde(flatten)]
        base: StepBase,
        url: Overridable<String>,
        to: Overridable<String>,
        checksum: Overridable<String>,
        timeout: Overridable<String>,
    },
    Signal {
        #[serde(flatten)]
        base: StepBase,
        signal: Overridable<SignalKind>,
        timeout: Overridable<String>,
    },
    Dependencies {
        #[serde(flatten)]
        base: StepBase,
    },
    Extract {
        #[serde(flatten)]
        base: StepBase,
    },
    Portable {
        #[serde(flatten)]
        base: StepBase,
    },
    Expose {
        #[serde(flatten)]
        base: StepBase,
    },
    Unexpose {
        #[serde(flatten)]
        base: StepBase,
    },
}

/// A manifest-declared custom method (e.g. "backup", "rcon") -- distinct from the reserved lifecycle actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrowMethod {
    pub name: String,
    pub description: String,
    pub available_in: Vec<ArrowState>,
    pub steps: Vec<ArrowStepDefinition>,
}

/// The five reserved runtime verbs, each with its own step list, separate from `methods`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrowLifecycle {
    pub install: Vec<ArrowStepDefinition>,
    pub update: Vec<ArrowStepDefinition>,
    pub execute: Vec<ArrowStepDefinition>,
    pub stop: Vec<ArrowStepDefinition>,
    pub uninstall: Vec<ArrowStepDefinition>,
}

/// Per-target hardware requirement -- quiver.core declares this once per OS, never once per arrow.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrowRequirement {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub disk_gb: f64,
}

/// One platform's build of the arrow -- requirement, lifecycle steps, and custom methods are all per-target in core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrowTarget {
    pub platform: String,
    pub requirement: ArrowRequirement,
    pub lifecycle: ArrowLifecycle,
    pub methods: Vec<ArrowMethod>,
}

/// The target whose platform matches this machine, or the first target when none matches
/// (e.g. a package with no host-specific build).
pub fn target_for_platform<'a>(
    targets: &'a [ArrowTarget],
    platform: &str,
) -> Option<&'a ArrowTarget> {
    targets
        .iter()
        .find(|t| t.platform == platform)
        .or_else(|| targets.first())
}

/// Whether this arrow genuinely publishes a build for `platform` -- an exact match, no fallback.
/// Deliberately separate from [`target_for_platform`]: its fallback to the first target exists for
/// manifests with a single universal target; this is the only place that can tell "no match" from
/// "one universal target", which is what a not-supported warning needs to know.
pub fn is_platform_supported(targets: &[ArrowTarget], platform: &str) -> bool {
    targets.iter().any(|t| t.platform == platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(platform: &str) -> ArrowTarget {
        ArrowTarget {
            platform: platform.into(),
            requirement: ArrowRequirement {
                cpu_cores: 1.0,
                memory_gb: 1.0,
                disk_gb: 1.0,
            },
            lifecycle: ArrowLifecycle::default(),
            methods: vec![],
        }
    }

    #[test]
    fn supported_on_exact_match() {
        let targets = [target("linux/amd64"), target("darwin/arm64")];
        assert!(is_platform_supported(&targets, "darwin/arm64"));
    }

    #[test]
    fn unsupported_without_match_even_though_lookup_falls_back() {
        let targets = [target("linux/amd64"), target("windows/amd64")];
        assert!(!is_platform_supported(&targets, "darwin/arm64"));
        assert_eq!(
            target_for_platform(&targets, "darwin/arm64"),
            Some(&targets[0])
        );
    }

    #[test]
    fn unsupported_with_no_targets() {
        assert!(!is_platform_supported(&[], "darwin/arm64"));
        assert!(target_for_platform(&[], "x").is_none());
    }

    #[test]
    fn single_universal_target_that_matches() {
        assert!(is_platform_supported(
            &[target("linux/amd64")],
            "linux/amd64"
        ));
    }

    #[test]
    fn steps_round_trip_with_per_platform_overrides() {
        let json = serde_json::json!({
            "type": "run", "title": "go", "command": {"default": "a", "linux/amd64": "b"},
            "elevated": false, "timeout": "5s"
        });
        let step: ArrowStepDefinition = serde_json::from_value(json.clone()).unwrap();
        assert!(
            matches!(&step, ArrowStepDefinition::Run { command: Overridable::PerPlatform(m), .. } if m.len() == 2)
        );
        assert_eq!(serde_json::to_value(&step).unwrap(), json);
        let dep: ArrowStepDefinition =
            serde_json::from_value(serde_json::json!({"type":"dependencies","title":"d"})).unwrap();
        assert!(matches!(dep, ArrowStepDefinition::Dependencies { .. }));
    }
}

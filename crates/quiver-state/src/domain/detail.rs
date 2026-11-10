use serde::{Deserialize, Serialize};

use super::arrow::{ActiveRun, ArrowOrigin, ArrowState, InferenceConfidence, LastReturnDetail};
use super::step::ArrowTarget;

/// A named credit -- quiver.core's `domain.Credit`: maintainers and credits are both arrays of this, never plain strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowCredit {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArrowMedia {
    pub icon: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    Number,
    Boolean,
    Select,
    /// Also what the daemon's empty `type` means: a manifest that declares no type is a text field.
    #[serde(other)]
    String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrowVariable {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub var_type: VariableType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

/// quiver.core's `netbridge.PortDef` -- name/protocol/default/required only. No live "currently bound" flag exists in core; don't invent one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowPort {
    pub name: String,
    pub protocol: PortProtocol,
    pub default: u32,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Tool,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Ordered,
    Pointer,
}

/// One published release channel. `Ordered` carries ranked tags (`members`, highest-precedence
/// first, `members[0]` equal to `latest`); `Pointer` is an unversioned branch/ref with nothing to
/// rank, so `count`/`members` are absent and `latest` is its only version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowChannel {
    pub name: String,
    pub kind: ChannelKind,
    pub latest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
}

/// How a catalog identity's selector is followed, as quiver.core classified it once when the row was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectorKind {
    Pin,
    Channel,
    Constraint,
    Commit,
}

/// An older core sends no kind at all, which quiver.core itself reads as a pin of the identity's ref.
pub fn parse_selector_kind(value: Option<&str>) -> SelectorKind {
    match value {
        Some("channel") => SelectorKind::Channel,
        Some("constraint") => SelectorKind::Constraint,
        Some("commit") => SelectorKind::Commit,
        _ => SelectorKind::Pin,
    }
}

/// A newer ref an installed row's selector points at, and the commit it resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableVersion {
    #[serde(rename = "ref", default)]
    pub reference: String,
    #[serde(default)]
    pub commit: String,
}

/// One resolved entry from `GET /v0/arrow/:ns/dependencies` -- namespace and ref already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrowDependency {
    pub namespace: String,
    pub dep_type: DependencyType,
}

/// The merged result of the six per-arrow endpoints -- everything the Arrow Details page needs
/// beyond the reactive `ArrowEntry`. `user_installed` is the one authoritative signal for "in the library".
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowDetail {
    pub origin: Option<ArrowOrigin>,
    pub confidence: Option<InferenceConfidence>,
    /// Machine codes from an inferred manifest's generator, e.g. `assumed_arch`; empty for a declared arrow.
    pub warnings: Vec<String>,
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub license: String,
    pub url: String,
    pub tags: Vec<String>,
    pub media: ArrowMedia,
    pub maintainers: Vec<ArrowCredit>,
    pub credits: Vec<ArrowCredit>,
    pub netbridge: Vec<ArrowPort>,
    pub variables: Vec<ArrowVariable>,
    pub targets: Vec<ArrowTarget>,
    pub state: ArrowState,
    pub user_installed: bool,
    pub installed_at: Option<String>,
    /// The part of `namespace` after its first `@`; it never changes for the life of the row.
    pub selector: String,
    pub selector_kind: SelectorKind,
    /// The ref this row resolved to. Empty until one is resolved.
    pub resolved_ref: String,
    pub installed_commit: String,
    /// What the last version check found ahead of `resolved_ref`, or `None` when current.
    pub available: Option<AvailableVersion>,
    /// True exactly when `available` is set (unless the payload says otherwise).
    pub outdated: bool,
    pub active_run: Option<ActiveRun>,
    pub last_return: Option<LastReturnDetail>,
    /// Always a list, empty when there is nothing to show.
    pub channels: Vec<ArrowChannel>,
    /// The arrow's ARROW.md, raw; `None` when the readme endpoint 404s.
    pub readme: Option<String>,
    pub dependencies: Vec<ArrowDependency>,
    /// namespace@ref of every installed arrow that declares a dependency on this one.
    pub dependents: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::VariableType;

    #[test]
    fn an_undeclared_type_reads_as_a_text_variable() {
        let read = |t: &str| serde_json::from_str::<VariableType>(&format!("\"{t}\"")).unwrap();
        assert_eq!(read(""), VariableType::String);
        assert_eq!(read("string"), VariableType::String);
        assert_eq!(read("number"), VariableType::Number);
        assert_eq!(read("select"), VariableType::Select);
    }
}

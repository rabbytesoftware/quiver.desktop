//! `GET /v0/arrow/:ns/manifest` and the other per-arrow endpoints -- everything the plain detail
//! call deliberately lacks. Two real endpoints, not one: quiver.core never returns media,
//! maintainers, credits, url, requirements, netbridge, variables or methods from the plain detail call.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::domain::{
    ArrowChannel, ArrowCredit, ArrowDependency, ArrowLifecycle, ArrowMethod, ArrowPort, ArrowRequirement,
    ArrowState, ArrowStepDefinition, ArrowTarget, ArrowVariable, DependencyType,
};

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelListDto {
    #[serde(default)]
    pub channels: Vec<ArrowChannel>,
}

pub fn to_arrow_channels(dto: ChannelListDto) -> Vec<ArrowChannel> {
    dto.channels
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ArrowMediaDto {
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
}

/// Core's wire step is already the full raw definition, nothing to rename or reshape on the way in.
pub type StepDto = ArrowStepDefinition;

#[derive(Debug, Clone, Deserialize)]
pub struct MethodDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub available_in: Vec<ArrowState>,
    #[serde(default)]
    pub steps: Vec<StepDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetManifestDto {
    pub requirements: ArrowRequirement,
    pub lifecycle: ArrowLifecycle,
    #[serde(default)]
    pub methods: Option<IndexMap<String, MethodDto>>,
}

/// The manifest's `metadata:` block as its author wrote it. Every list here can arrive as JSON
/// null rather than absent or empty (Go marshals a nil slice that way). `tags` arriving null once
/// crashed the whole arrow-details view, so every list is defaulted at this boundary.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ManifestMetadataDto {
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub maintainers: Option<Vec<ArrowCredit>>,
    #[serde(default)]
    pub credits: Option<Vec<ArrowCredit>>,
    #[serde(default)]
    pub media: Option<ArrowMediaDto>,
}

/// The manifest as its author wrote it -- the only place url/maintainers/credits/media/netbridge
/// live on the wire. Row state (selector, resolved and available refs) is never here.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ManifestContentDto {
    #[serde(default)]
    pub metadata: Option<ManifestMetadataDto>,
    #[serde(default)]
    pub variables: Option<Vec<ArrowVariable>>,
    /// Neither of Quiver's own self-manifests declares `netbridge:`, so this is null for both.
    #[serde(default)]
    pub netbridge: Option<Vec<ArrowPort>>,
    #[serde(default)]
    pub readme: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrowManifestDto {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub variables: Option<Vec<ArrowVariable>>,
    #[serde(default)]
    pub targets: Option<IndexMap<String, TargetManifestDto>>,
    /// A pointer on the Go side, so null when core had no manifest to report.
    #[serde(default)]
    pub manifest: Option<ManifestContentDto>,
}

/// `GET /v0/arrow/:ns/readme` -- a third, separate endpoint. `:ns` is the full identity; 404s when
/// the arrow has no ARROW.md prose.
#[derive(Debug, Clone, Deserialize)]
pub struct ArrowReadmeDto {
    pub readme: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrowDependencyDto {
    pub namespace: String,
    #[serde(rename = "type")]
    pub dep_type: String,
}

/// `GET /v0/arrow/:ns/dependencies` -- the resolved, topologically ordered plan. `:ns` is the full
/// `namespace@ref`: resolution is version-specific.
#[derive(Debug, Clone, Deserialize)]
pub struct ArrowDependenciesDto {
    #[serde(default)]
    pub dependencies: Option<Vec<ArrowDependencyDto>>,
}

/// `GET /v0/arrow/:ns/dependents` -- namespace@ref of every installed arrow that declares a dependency on this one.
#[derive(Debug, Clone, Deserialize)]
pub struct ArrowDependentsDto {
    #[serde(default)]
    pub dependents: Option<Vec<String>>,
}

/// `type` comes off the wire as `domain.DepType`'s two real values; anything else reads as a tool.
pub fn to_arrow_dependencies(dtos: Vec<ArrowDependencyDto>) -> Vec<ArrowDependency> {
    dtos.into_iter()
        .map(|d| ArrowDependency {
            dep_type: if d.dep_type == "service" { DependencyType::Service } else { DependencyType::Tool },
            namespace: d.namespace,
        })
        .collect()
}

pub(super) fn to_targets(dto: IndexMap<String, TargetManifestDto>) -> Vec<ArrowTarget> {
    dto.into_iter()
        .map(|(platform, target)| ArrowTarget {
            platform,
            requirement: target.requirements,
            lifecycle: target.lifecycle,
            methods: target
                .methods
                .unwrap_or_default()
                .into_values()
                .map(|m| ArrowMethod {
                    name: m.name,
                    description: m.description,
                    available_in: m.available_in,
                    steps: m.steps,
                })
                .collect(),
        })
        .collect()
}

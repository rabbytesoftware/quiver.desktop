use serde::Deserialize;

use super::manifest::{to_arrow_dependencies, to_targets, ArrowDependencyDto, ArrowManifestDto};
use super::runtime::{to_active_run, ActiveRunDto};
use super::search::MediaDto;
use crate::domain::{
    parse_arrow_origin, parse_inference_confidence, parse_selector_kind, ArrowChannel, ArrowDetail,
    ArrowMedia, ArrowState, AvailableVersion, LastReturnDetail, RuntimeUpdate,
};
use crate::namespace::selector_of;
use crate::persistence::ArrowCatalogRecord;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InferenceDto {
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub warnings: Option<Vec<String>>,
}

/// One catalog row of an arrow. `ref` is the identity selector the row is filed under (`stable`,
/// `v1.*`, `v1.2.0`, a commit) and never changes; `resolved_ref` is the ref it resolved to, empty
/// until it resolved one. Absent altogether on an older core, which reads the same as unresolved.
#[derive(Debug, Clone, Deserialize)]
pub struct InstalledVersionDto {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub resolved_ref: Option<String>,
    pub state: ArrowState,
    /// Omitted until a successful install stamps it, and again after an uninstall.
    #[serde(default)]
    pub installed_at: Option<String>,
    /// Set once quiver.core stamps it on a completed `execute`; absent until then.
    #[serde(default)]
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrowListResponseItemDto {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// NULL, not absent and not empty, for any arrow whose manifest declares no `tags:` -- which
    /// includes both of Quiver's own self-arrows. Go marshals a nil slice as JSON null. Every
    /// mapper has to default it; a consumer reading `.len()` off a null crashes the page otherwise.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub media: Option<MediaDto>,
    #[serde(default)]
    pub versions: Vec<InstalledVersionDto>,
    /// Absent on a daemon that predates inference; reads as declared.
    #[serde(default)]
    pub origin: Option<String>,
    /// Omitted unless the arrow is inferred.
    #[serde(default)]
    pub confidence: Option<String>,
}

/// `GET /v0/arrow/:ns`. `namespace` is the catalog identity, `bare@selector`, always -- an
/// uncatalogued refless read is resolved live and reported under the repository's default channel.
/// The row-state fields are optional because an older core sends none of them; the mapper reads
/// that as a pin with nothing resolved.
#[derive(Debug, Clone, Deserialize)]
pub struct ArrowDetailDto {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    pub state: ArrowState,
    /// Null for an arrow with no `tags:`, exactly as above.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Omitted while the arrow is not on disk.
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub user_installed: bool,
    #[serde(default)]
    pub selector_kind: Option<String>,
    #[serde(default)]
    pub resolved_ref: Option<String>,
    #[serde(default)]
    pub installed_commit: Option<String>,
    /// Omitted when current.
    #[serde(default)]
    pub available: Option<AvailableVersion>,
    #[serde(default)]
    pub outdated: Option<bool>,
    #[serde(default)]
    pub active_run: Option<ActiveRunDto>,
    #[serde(default)]
    pub last_return: Option<LastReturnDetail>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub inference: Option<InferenceDto>,
}

pub fn to_arrow_catalog_records(
    items: &[ArrowListResponseItemDto],
    connection_id: &str,
) -> Vec<ArrowCatalogRecord> {
    items
        .iter()
        .flat_map(|arrow| {
            arrow.versions.iter().map(move |v| ArrowCatalogRecord {
                connection_id: connection_id.to_string(),
                namespace: format!("{}@{}", arrow.namespace, v.reference),
                name: arrow.name.clone(),
                description: arrow.description.clone(),
                tags: arrow.tags.clone().unwrap_or_default(),
                icon: arrow
                    .media
                    .as_ref()
                    .and_then(|m| m.icon.clone())
                    .filter(|s| !s.is_empty()),
                banner: arrow
                    .media
                    .as_ref()
                    .and_then(|m| m.banner.clone())
                    .filter(|s| !s.is_empty()),
                version: v.resolved_ref.clone().unwrap_or_default(),
                last_used_at: v.last_used_at.clone(),
                origin: Some(parse_arrow_origin(arrow.origin.as_deref())),
                confidence: parse_inference_confidence(arrow.confidence.as_deref()),
            })
        })
        .collect()
}

pub fn to_initial_runtime_updates(items: &[ArrowListResponseItemDto]) -> Vec<RuntimeUpdate> {
    items
        .iter()
        .flat_map(|arrow| {
            arrow.versions.iter().map(move |v| RuntimeUpdate {
                namespace: format!("{}@{}", arrow.namespace, v.reference),
                state: v.state,
                active_run: None,
                last_return: None,
            })
        })
        .collect()
}

fn to_available(dto: Option<AvailableVersion>) -> Option<AvailableVersion> {
    dto.filter(|a| !a.reference.is_empty())
}

/// Merges the six real endpoints into the one shape the Arrow Details page consumes. `channels`
/// comes from `/channels`, `readme` from `/readme` (`None` when that 404s), `dependencies` and
/// `dependents` from the two dependency-graph endpoints, empty when nothing 200s back.
pub fn to_arrow_detail(
    detail: ArrowDetailDto,
    manifest: ArrowManifestDto,
    channels: Vec<ArrowChannel>,
    readme: Option<String>,
    dependencies: Vec<ArrowDependencyDto>,
    dependents: Vec<String>,
) -> ArrowDetail {
    let content = manifest.manifest.unwrap_or_default();
    let meta = content.metadata.unwrap_or_default();
    let media = meta.media.unwrap_or_default();
    let available = to_available(detail.available);
    let inference = detail.inference.unwrap_or_default();
    ArrowDetail {
        selector: selector_of(&detail.namespace).to_string(),
        license: detail.license.or(meta.license).unwrap_or_default(),
        url: meta.url.unwrap_or_default(),
        tags: detail.tags.unwrap_or_default(),
        media: ArrowMedia {
            icon: media.icon,
            banner: media.banner,
        },
        maintainers: meta.maintainers.unwrap_or_default(),
        credits: meta.credits.unwrap_or_default(),
        netbridge: content.netbridge.unwrap_or_default(),
        variables: manifest.variables.unwrap_or_default(),
        targets: to_targets(manifest.targets.unwrap_or_default()),
        state: detail.state,
        user_installed: detail.user_installed,
        installed_at: detail.installed_at,
        selector_kind: parse_selector_kind(detail.selector_kind.as_deref()),
        resolved_ref: detail.resolved_ref.unwrap_or_default(),
        installed_commit: detail.installed_commit.unwrap_or_default(),
        outdated: detail.outdated.unwrap_or(available.is_some()),
        available,
        active_run: to_active_run(detail.active_run),
        last_return: detail.last_return,
        origin: Some(parse_arrow_origin(detail.origin.as_deref())),
        confidence: parse_inference_confidence(inference.confidence.as_deref()),
        warnings: inference.warnings.unwrap_or_default(),
        channels,
        readme,
        dependencies: to_arrow_dependencies(dependencies),
        dependents,
        namespace: detail.namespace,
        name: detail.name,
        description: detail.description,
    }
}

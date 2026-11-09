//! Wire types for the two search lanes. Both lanes carry the same result shape: core builds one
//! model and hands it to one mapper, so `GET /v0/search` and the discovery stream cannot drift.

use serde::Deserialize;

use crate::domain::{
    parse_arrow_origin, parse_inference_confidence, DiscoveryProvider, DiscoverySummary,
    SearchEntry, SearchProvenance,
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MediaDto {
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResultDto {
    /// Bare, without a ref. `versions` carries the refs.
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Both halves are optional in practice: core serialises an unset image as `""`.
    #[serde(default)]
    pub media: Option<MediaDto>,
    /// Refs known locally, not every ref that exists upstream.
    #[serde(default)]
    pub versions: Option<Vec<String>>,
    /// Advisory: install-time re-resolution is authoritative.
    #[serde(default)]
    pub compatible_os: Option<Vec<String>>,
    /// Absent when the server cannot say which provenance the catalog recorded.
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub known: bool,
    #[serde(default)]
    pub stars: Option<u64>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

/// The answer to `POST /v0/search/discover`, returned before any provider has been asked.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryJobStartedDto {
    pub job_id: String,
    pub query: String,
    /// While the pass runs this is a floor rather than a promise.
    #[serde(default)]
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryProviderDto {
    pub host: String,
    pub ok: bool,
    pub returned: u32,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub retry_after: Option<u32>,
}

/// Everything the stream cannot carry: counts and the hosts that refused. Read once, after the
/// socket closes -- they are written when the pass ends.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryJobDto {
    pub job_id: String,
    #[serde(default)]
    pub status: Option<String>,
    pub query: String,
    pub found: u32,
    pub verified: u32,
    pub skipped: u32,
    #[serde(default)]
    pub providers: Option<Vec<DiscoveryProviderDto>>,
}

/// Core writes an absent image as `""`, not as null or an omitted key, so presence alone is not
/// enough; normalising here is what keeps `icon == None` the single way to ask.
fn media_url(value: Option<&String>) -> Option<String> {
    value.filter(|v| !v.is_empty()).cloned()
}

fn parse_provenance(value: Option<&str>) -> Option<SearchProvenance> {
    match value? {
        "installed" => Some(SearchProvenance::Installed),
        "dependency" => Some(SearchProvenance::Dependency),
        "collection" => Some(SearchProvenance::Collection),
        "seen" => Some(SearchProvenance::Seen),
        _ => None,
    }
}

pub fn to_search_entry(dto: SearchResultDto) -> SearchEntry {
    SearchEntry {
        icon: media_url(dto.media.as_ref().and_then(|m| m.icon.as_ref())),
        banner: media_url(dto.media.as_ref().and_then(|m| m.banner.as_ref())),
        origin: Some(parse_arrow_origin(dto.origin.as_deref())),
        confidence: parse_inference_confidence(dto.confidence.as_deref()),
        provenance: parse_provenance(dto.provenance.as_deref()),
        namespace: dto.namespace,
        name: dto.name,
        description: dto.description,
        tags: dto.tags.unwrap_or_default(),
        versions: dto.versions.unwrap_or_default(),
        compatible_os: dto.compatible_os.unwrap_or_default(),
        installed: dto.installed,
        known: dto.known,
        stars: dto.stars.unwrap_or(0),
        source: dto.source,
    }
}

pub fn to_discovery_summary(dto: DiscoveryJobDto) -> DiscoverySummary {
    DiscoverySummary {
        job_id: dto.job_id,
        query: dto.query,
        found: dto.found,
        verified: dto.verified,
        skipped: dto.skipped,
        providers: dto
            .providers
            .unwrap_or_default()
            .into_iter()
            .map(|p| DiscoveryProvider {
                host: p.host,
                ok: p.ok,
                returned: p.returned,
                reason: p.reason,
                retry_after: p.retry_after,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::domain::{ArrowOrigin, InferenceConfidence};

    fn entry(extra: Value) -> SearchEntry {
        let mut base = json!({
            "namespace": "github.com/a/one", "name": "one", "description": "d", "tags": ["t"],
            "versions": ["v1"], "compatible_os": ["linux/amd64"], "installed": false, "known": true, "stars": 3
        });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        to_search_entry(serde_json::from_value(base).unwrap())
    }

    #[test]
    fn flattens_media_and_drops_empty_strings() {
        let e = entry(json!({"media": {"icon": "i.png", "banner": ""}}));
        assert_eq!((e.icon.as_deref(), e.banner), (Some("i.png"), None));
    }

    #[test]
    fn reads_an_absent_provenance_as_none_and_keeps_installed_and_known_apart() {
        let e = entry(json!({"installed": false, "known": true}));
        assert_eq!(e.provenance, None);
        assert!(!e.installed && e.known);
        assert_eq!(
            entry(json!({"provenance": "collection"})).provenance,
            Some(SearchProvenance::Collection)
        );
    }

    #[test]
    fn defaults_absent_media_source_lists_and_stars() {
        let e = to_search_entry(
            serde_json::from_value(
                json!({"namespace": "n", "name": "n", "installed": true, "known": true}),
            )
            .unwrap(),
        );
        assert_eq!((e.icon, e.banner, e.source), (None, None, None));
        assert!(e.tags.is_empty() && e.versions.is_empty() && e.compatible_os.is_empty());
        assert_eq!(e.stars, 0);
    }

    #[test]
    fn carries_origin_and_confidence_and_reads_unknown_origin_as_declared() {
        let e = entry(json!({"origin": "inferred", "confidence": "low"}));
        assert_eq!(
            (e.origin, e.confidence),
            (Some(ArrowOrigin::Inferred), Some(InferenceConfidence::Low))
        );
        let e = entry(json!({}));
        assert_eq!(
            (e.origin, e.confidence),
            (Some(ArrowOrigin::Declared), None)
        );
        assert_eq!(
            entry(json!({"origin": "weird"})).origin,
            Some(ArrowOrigin::Declared)
        );
    }

    #[test]
    fn summary_normalises_absent_reason_and_retry_to_none_and_no_providers_to_empty() {
        let s = to_discovery_summary(
            serde_json::from_value(json!({
                "job_id": "j", "status": "completed", "query": "q", "found": 7, "verified": 5, "skipped": 2,
                "providers": [
                    {"host": "github.com", "ok": true, "returned": 7},
                    {"host": "gitlab.com", "ok": false, "returned": 0, "reason": "rate limited", "retry_after": 30}
                ]
            }))
            .unwrap(),
        );
        assert_eq!((s.found, s.verified, s.skipped), (7, 5, 2));
        assert_eq!(
            (s.providers[0].reason.clone(), s.providers[0].retry_after),
            (None, None)
        );
        assert_eq!(
            (s.providers[1].reason.as_deref(), s.providers[1].retry_after),
            (Some("rate limited"), Some(30))
        );
        let none = to_discovery_summary(
            serde_json::from_value(
                json!({"job_id": "j", "query": "q", "found": 0, "verified": 0, "skipped": 0}),
            )
            .unwrap(),
        );
        assert!(none.providers.is_empty());
    }
}

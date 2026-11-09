use serde::{Deserialize, Serialize};

use super::arrow::{ArrowOrigin, InferenceConfidence};

/// `None` on a [`SearchEntry`] when the server cannot say, not when there is none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchProvenance {
    Installed,
    Dependency,
    Collection,
    Seen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchEntry {
    /// Bare, without a ref, unlike `ArrowEntry.namespace`. `versions` carries the refs.
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub versions: Vec<String>,
    pub compatible_os: Vec<String>,
    pub provenance: Option<SearchProvenance>,
    /// The catalog holds it. Narrower than `known` and never implied by it.
    pub installed: bool,
    /// The catalog or the vault holds it. Browsing is not having.
    pub known: bool,
    pub stars: u64,
    pub source: Option<String>,
    pub origin: Option<ArrowOrigin>,
    pub confidence: Option<InferenceConfidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryProvider {
    pub host: String,
    pub ok: bool,
    pub returned: u32,
    pub reason: Option<String>,
    /// Seconds.
    pub retry_after: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverySummary {
    pub job_id: String,
    pub query: String,
    pub found: u32,
    pub verified: u32,
    pub skipped: u32,
    pub providers: Vec<DiscoveryProvider>,
}

/// Whether the arrow is already yours, which is what separates the two result shelves (spec 9.3).
///
/// `installed` carries this: core sets it from the catalog alone. `known` cannot -- every
/// `GET /v0/search` result is known by construction, because discovery indexes each arrow it proves.
///
/// Following a collection is the exception the boolean misses: it caches the collection's arrows
/// into the vault without writing catalog rows, so a curated arrow reports `installed: false`
/// while plainly being yours.
pub fn is_held(entry: &SearchEntry) -> bool {
    entry.installed || entry.provenance == Some(SearchProvenance::Collection)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn entry(installed: bool, provenance: Option<SearchProvenance>) -> SearchEntry {
        SearchEntry {
            namespace: "ns".into(),
            name: "n".into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            versions: vec![],
            compatible_os: vec![],
            provenance,
            installed,
            known: true,
            stars: 0,
            source: None,
            origin: None,
            confidence: None,
        }
    }

    #[test]
    fn held_when_installed_or_from_a_followed_collection() {
        assert!(is_held(&entry(true, Some(SearchProvenance::Seen))));
        assert!(is_held(&entry(false, Some(SearchProvenance::Collection))));
        assert!(!is_held(&entry(false, Some(SearchProvenance::Seen))));
        assert!(!is_held(&entry(false, None)));
    }
}

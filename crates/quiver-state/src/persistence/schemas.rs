use serde::{Deserialize, Serialize};

use crate::domain::{ArrowOrigin, InferenceConfidence};

/// One cached catalog row, keyed by `(connection_id, namespace)`. `origin`/`confidence` are absent
/// on a row cached before quiver.core reported them, which reads as declared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrowCatalogRecord {
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub version: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub origin: Option<ArrowOrigin>,
    #[serde(default)]
    pub confidence: Option<InferenceConfidence>,
}

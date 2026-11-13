use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionArrow {
    pub namespace: String,
    pub version: Option<String>,
    pub resolved: bool,
    pub name: Option<String>,
    pub description: Option<String>,
    /// The member's own icon and banner (from its resolved manifest); absent when it has none.
    pub icon: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionListItem {
    pub namespace: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub followed: bool,
    pub arrow_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMedia {
    pub icon: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionDetail {
    pub item: CollectionListItem,
    pub url: Option<String>,
    pub maintainers: Vec<String>,
    /// Always present, never undefined; empty-valued when the collection declares none.
    pub media: CollectionMedia,
    pub arrows: Vec<CollectionArrow>,
}

/// The exact route quiver.core resolved this member from -- `owner/repo@version`, or bare
/// `owner/repo` when the member carries no version.
pub fn collection_arrow_route(arrow: &CollectionArrow) -> String {
    match arrow.version.as_deref() {
        Some(v) if !v.is_empty() => format!("{}@{v}", arrow.namespace),
        _ => arrow.namespace.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(version: Option<&str>) -> CollectionArrow {
        CollectionArrow {
            namespace: "github.com/rabbyte/ark".into(),
            version: version.map(Into::into),
            resolved: false,
            name: None,
            description: None,
            icon: None,
            banner: None,
        }
    }

    #[test]
    fn route_carries_the_version_when_there_is_one() {
        assert_eq!(
            collection_arrow_route(&member(Some("v3.1.0"))),
            "github.com/rabbyte/ark@v3.1.0"
        );
        assert_eq!(
            collection_arrow_route(&member(None)),
            "github.com/rabbyte/ark"
        );
    }
}

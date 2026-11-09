use serde::Deserialize;

use crate::domain::{CollectionArrow, CollectionDetail, CollectionListItem, CollectionMedia};
use crate::namespace::split_namespace;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CollectionMediaDto {
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectionArrowDto {
    /// `owner/repo@version` -- core sends no separate version field for a member arrow.
    pub namespace: String,
    pub resolved: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// The member's own media; both fields are empty strings when it declares none.
    #[serde(default)]
    pub media: Option<CollectionMediaDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectionListItemDto {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub arrow_count: usize,
    #[serde(default)]
    pub followed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectionDetailDto {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub maintainers: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub media: Option<CollectionMediaDto>,
    #[serde(default)]
    pub arrows: Vec<CollectionArrowDto>,
    #[serde(default)]
    pub followed: bool,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

/// Core folds the version into `namespace` as `owner/repo@version`; this splits it back apart with
/// the same rule the sidebar's namespace rows use.
pub fn to_collection_arrow(dto: CollectionArrowDto) -> CollectionArrow {
    let parts = split_namespace(&dto.namespace);
    let (namespace, version) = (parts.head.to_string(), (!parts.tail.is_empty()).then(|| parts.tail[1..].to_string()));
    let media = dto.media.unwrap_or_default();
    CollectionArrow {
        namespace,
        version,
        resolved: dto.resolved,
        name: dto.name,
        description: dto.description,
        icon: non_empty(media.icon),
        banner: non_empty(media.banner),
    }
}

pub fn to_collection_list_item(dto: CollectionListItemDto) -> CollectionListItem {
    CollectionListItem {
        namespace: dto.namespace,
        name: dto.name,
        description: dto.description.unwrap_or_default(),
        tags: dto.tags.unwrap_or_default(),
        followed: dto.followed,
        arrow_count: dto.arrow_count,
    }
}

pub fn to_collection_detail(dto: CollectionDetailDto) -> CollectionDetail {
    let item = to_collection_list_item(CollectionListItemDto {
        namespace: dto.namespace,
        name: dto.name,
        description: dto.description,
        tags: dto.tags,
        arrow_count: dto.arrows.len(),
        followed: dto.followed,
    });
    let media = dto.media.unwrap_or_default();
    CollectionDetail {
        item,
        url: dto.url,
        maintainers: dto.maintainers.unwrap_or_default(),
        media: CollectionMedia { icon: media.icon, banner: media.banner },
        arrows: dto.arrows.into_iter().map(to_collection_arrow).collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn arrow(value: serde_json::Value) -> CollectionArrow {
        to_collection_arrow(serde_json::from_value(value).unwrap())
    }

    #[test]
    fn splits_the_version_out_of_a_versioned_namespace() {
        let a = arrow(json!({"namespace": "github.com/r/mc@v1.21.4", "resolved": true, "name": "MC"}));
        assert_eq!((a.namespace.as_str(), a.version.as_deref()), ("github.com/r/mc", Some("v1.21.4")));
        let bare = arrow(json!({"namespace": "github.com/r/mc", "resolved": true}));
        assert_eq!(bare.version, None);
    }

    #[test]
    fn carries_member_media_and_drops_empty_ones() {
        let a = arrow(json!({"namespace": "n@1", "resolved": true, "media": {"icon": "i.png", "banner": ""}}));
        assert_eq!((a.icon.as_deref(), a.banner), (Some("i.png"), None));
        let a = arrow(json!({"namespace": "n@1", "resolved": false}));
        assert!(!a.resolved && a.name.is_none() && a.description.is_none() && a.icon.is_none());
    }

    #[test]
    fn list_item_defaults_description_and_tags() {
        let i = to_collection_list_item(
            serde_json::from_value(json!({"namespace": "n", "name": "N", "arrow_count": 3, "followed": true})).unwrap(),
        );
        assert_eq!((i.description.as_str(), i.arrow_count, i.followed), ("", 3, true));
        assert!(i.tags.is_empty());
    }

    #[test]
    fn detail_derives_the_arrow_count_defaults_media_and_keeps_url_and_maintainers() {
        let d = to_collection_detail(
            serde_json::from_value(json!({
                "namespace": "n", "name": "N", "followed": true, "url": "https://x", "maintainers": ["a"],
                "arrows": [{"namespace": "a@1", "resolved": true}, {"namespace": "b@2", "resolved": false}]
            }))
            .unwrap(),
        );
        assert_eq!(d.item.arrow_count, 2);
        assert_eq!(d.media, CollectionMedia::default());
        assert_eq!((d.url.as_deref(), d.maintainers), (Some("https://x"), vec!["a".to_string()]));
        let bare = to_collection_detail(serde_json::from_value(json!({"namespace": "n", "name": "N"})).unwrap());
        assert!(bare.maintainers.is_empty() && bare.url.is_none());
    }
}

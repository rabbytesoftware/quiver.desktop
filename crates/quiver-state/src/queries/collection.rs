use crate::core_api::{CoreApi, CoreApiExt};
use crate::domain::{CollectionDetail, CollectionListItem};
use crate::dto::collection::{
    to_collection_detail, to_collection_list_item, CollectionDetailDto, CollectionListItemDto,
};
use crate::namespace::encode_uri_component;
use crate::Result;

/// A bare `/collection/` route (an empty splat) has nothing to fetch -- don't fire
/// `/v0/collection/` at the backend for it.
pub fn is_enabled(namespace: &str) -> bool {
    !namespace.is_empty()
}

pub fn collection_path(namespace: &str) -> String {
    format!("/v0/collection/{}", encode_uri_component(namespace))
}

pub async fn fetch_collection_detail(
    api: &dyn CoreApi,
    namespace: &str,
) -> Result<CollectionDetail> {
    Ok(to_collection_detail(
        api.get::<CollectionDetailDto>(&collection_path(namespace))
            .await?,
    ))
}

/// `GET /v0/collection?followed=true` -- the collections the user has already opted into, for Home
/// and the full Collections page.
pub async fn fetch_followed_collections(api: &dyn CoreApi) -> Result<Vec<CollectionListItem>> {
    let items = api
        .get::<Vec<CollectionListItemDto>>("/v0/collection?followed=true")
        .await?;
    Ok(items.into_iter().map(to_collection_list_item).collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::core_api::{FakeCoreApi, Method};

    #[tokio::test]
    async fn fetches_and_maps_a_collection_with_the_namespace_encoded() {
        let api = FakeCoreApi::new();
        api.route(
            Method::Get,
            "/v0/collection/github.com%2Fr%2Fcol",
            json!({"namespace": "github.com/r/col", "name": "Col", "followed": true, "arrows": [{"namespace": "a@1", "resolved": true}]}),
        );
        let d = fetch_collection_detail(&api, "github.com/r/col")
            .await
            .unwrap();
        assert_eq!((d.item.arrow_count, d.item.followed), (1, true));
        assert!(!is_enabled(""));
    }

    #[tokio::test]
    async fn lists_the_followed_collections() {
        let api = FakeCoreApi::new();
        api.route(
            Method::Get,
            "/v0/collection?followed=true",
            json!([{"namespace": "n", "name": "N", "arrow_count": 3, "followed": true}]),
        );
        let items = fetch_followed_collections(&api).await.unwrap();
        assert_eq!(
            (items[0].arrow_count, items[0].description.as_str()),
            (3, "")
        );
    }
}

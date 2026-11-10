//! The two lookups behind "Enter takes me there" (port of `resolve-namespace.ts`): ask
//! `GET /v0/{collection,arrow}/:namespace`, in parallel, and let `pick_target` decide.

use super::resolve::{looks_like_namespace, pick_target, should_check_collection, NamespaceTarget};
use crate::core_api::{ApiError, CoreApi, CoreApiExt, Method};
use crate::namespace::encode_uri_component;

/// 404 is "no such thing"; any other failure is a real one and propagates (a daemon that is down must
/// not read as "nothing there", or Enter would silently fall through to a search).
async fn exists(api: &dyn CoreApi, path: String) -> Result<bool, ApiError> {
    match api.send(Method::Get, &path, None).await {
        Ok(_) => Ok(true),
        Err(err) if err.is_not_found() => Ok(false),
        Err(err) => Err(err),
    }
}

/// `None` for anything that is not namespace-shaped (no request is made) or names neither an arrow nor
/// a collection.
pub async fn resolve_namespace(
    api: &dyn CoreApi,
    input: &str,
) -> Result<Option<NamespaceTarget>, ApiError> {
    let namespace = input.trim();
    if !looks_like_namespace(namespace) {
        return Ok(None);
    }
    let encoded = encode_uri_component(namespace);
    let collection = async {
        if should_check_collection(namespace) {
            exists(api, format!("/v0/collection/{encoded}")).await
        } else {
            Ok(false)
        }
    };
    let (is_collection, is_arrow) =
        tokio::join!(collection, exists(api, format!("/v0/arrow/{encoded}")));
    Ok(pick_target(namespace, is_collection?, is_arrow?))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::core_api::FakeCoreApi;
    use crate::search::resolve::TargetKind;

    const NS: &str = "github.com/a/b";
    const ENC: &str = "github.com%2Fa%2Fb";

    fn api(collection: Option<u16>, arrow: Option<u16>) -> FakeCoreApi {
        let api = FakeCoreApi::new();
        for (kind, status) in [("collection", collection), ("arrow", arrow)] {
            let path = format!("/v0/{kind}/{ENC}");
            match status {
                Some(200) => api.route(Method::Get, &path, json!({})),
                Some(code) => api.route_error(Method::Get, &path, code, "x"),
                None => api.route_error(Method::Get, &path, 404, "not found"),
            }
        }
        api
    }

    #[tokio::test]
    async fn a_collection_beats_an_arrow_and_an_arrow_beats_nothing() {
        let both = resolve_namespace(&api(Some(200), Some(200)), NS)
            .await
            .unwrap();
        assert_eq!(both.unwrap().kind, TargetKind::Collection);
        let arrow = resolve_namespace(&api(None, Some(200)), NS).await.unwrap();
        assert_eq!(arrow.unwrap().kind, TargetKind::Arrow);
        assert_eq!(resolve_namespace(&api(None, None), NS).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_server_error_propagates_instead_of_reading_as_not_found() {
        let err = resolve_namespace(&api(Some(500), None), NS)
            .await
            .unwrap_err();
        assert_eq!(err.status, 500);
    }

    #[tokio::test]
    async fn plain_words_make_no_request_and_a_versioned_namespace_skips_the_collection() {
        let fake = api(Some(200), Some(200));
        assert_eq!(resolve_namespace(&fake, "rust").await.unwrap(), None);
        assert!(fake.calls().is_empty());
        let versioned = format!("{NS}@v1");
        fake.route(Method::Get, &format!("/v0/arrow/{ENC}%40v1"), json!({}));
        let target = resolve_namespace(&fake, &versioned).await.unwrap().unwrap();
        assert_eq!(target.kind, TargetKind::Arrow);
        assert_eq!(
            fake.calls_to(Method::Get, &format!("/v0/collection/{ENC}%40v1")),
            0
        );
    }
}

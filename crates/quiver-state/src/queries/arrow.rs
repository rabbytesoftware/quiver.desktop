//! The per-arrow reads. quiver.core exposes six real endpoints for one arrow and there is no
//! single endpoint that returns all six; do not add one to a mock as a shortcut, since that would
//! stop the mock from catching a client that assumes there is.

use crate::core_api::{CoreApi, CoreApiExt};
use crate::domain::{ArrowChannel, ArrowDependency, ArrowDetail};
use crate::dto::arrow::{to_arrow_detail, ArrowDetailDto};
use crate::dto::manifest::{
    to_arrow_channels, to_arrow_dependencies, ArrowDependenciesDto, ArrowDependentsDto, ArrowManifestDto,
    ArrowReadmeDto, ChannelListDto,
};
use crate::namespace::{bare_namespace, namespace_segment};
use crate::Result;

/// A bare route (an empty namespace) has nothing to fetch -- don't fire `/v0/arrow/` for it.
pub fn is_enabled(namespace: &str) -> bool {
    !namespace.is_empty()
}

/// 404 is an expected outcome for the optional reads (no readme, nothing published): `fallback`
/// stands in for it, anything else is a real failure.
fn or_on_not_found<T>(result: Result<T>, fallback: T) -> Result<T> {
    match result {
        Err(err) if err.is_not_found() => Ok(fallback),
        other => other,
    }
}

/// `GET /v0/arrow/:ns/readme` 404s when the arrow has no readme -- a plain `arrow.yaml` delivery,
/// or an ARROW.md with nothing outside its fenced block -- which is not a failure of the whole
/// detail fetch. Read at the identity: two selectors of one repository can sit on manifests with
/// different prose.
pub async fn fetch_readme(api: &dyn CoreApi, namespace: &str) -> Result<Option<String>> {
    let dto = api.get::<ArrowReadmeDto>(&format!("/v0/arrow/{}/readme", namespace_segment(namespace))).await;
    or_on_not_found(dto.map(|d| Some(d.readme)), None)
}

/// `GET /v0/arrow/:ns/channels` -- what the repository publishes, not a property of any one row, so
/// it takes the bare namespace. 404 means the arrow doesn't resolve (or has no channels).
pub async fn fetch_channels(api: &dyn CoreApi, namespace: &str) -> Result<Vec<ArrowChannel>> {
    let path = format!("/v0/arrow/{}/channels", namespace_segment(bare_namespace(namespace)));
    let dto = api.get::<ChannelListDto>(&path).await.map(to_arrow_channels);
    or_on_not_found(dto, vec![])
}

/// `GET /v0/arrow/:ns/dependencies` -- unlike readme/manifest this takes the full `namespace@ref`:
/// the resolved plan is version-specific. 404 means the arrow doesn't resolve, not "no
/// dependencies" (that is an empty array on a 200); an empty list is a fine fallback for this page.
pub async fn fetch_dependencies(api: &dyn CoreApi, namespace: &str) -> Result<Vec<ArrowDependency>> {
    let path = format!("/v0/arrow/{}/dependencies", namespace_segment(namespace));
    let dto = api
        .get::<ArrowDependenciesDto>(&path)
        .await
        .map(|d| to_arrow_dependencies(d.dependencies.unwrap_or_default()));
    or_on_not_found(dto, vec![])
}

/// `GET /v0/arrow/:ns/dependents` -- core normalizes `:ns` to bare internally, but the full
/// namespace@ref is passed for consistency with `/dependencies`.
pub async fn fetch_dependents(api: &dyn CoreApi, namespace: &str) -> Result<Vec<String>> {
    let path = format!("/v0/arrow/{}/dependents", namespace_segment(namespace));
    let dto = api.get::<ArrowDependentsDto>(&path).await.map(|d| d.dependents.unwrap_or_default());
    or_on_not_found(dto, vec![])
}

/// The FAST/primary half: `GET /v0/arrow/:ns` (state/active_run/last_return and the row state) and
/// `GET /v0/arrow/:ns/manifest` (media, maintainers, credits, url, requirements, netbridge,
/// variables, methods). Both are simple, fast reads, unlike readme, channels and the dependency
/// graph -- `dependencies` especially, a real git resolution that can run 1.5-2+ seconds. This is
/// deliberately the ONLY read the details screen's loading/error gate reflects: identity, status
/// and the action row shouldn't sit behind a spinner waiting on a git resolution they don't need.
///
/// Returns a full [`ArrowDetail`] once these two resolve, with the four slow fields defaulted to
/// their empty shape (`channels: []`, `readme: None`, `dependencies: []`, `dependents: []`); each of
/// `fetch_channels`/`fetch_readme`/`fetch_dependencies`/`fetch_dependents` overlays its real value
/// once it resolves.
///
/// The manifest is read at the identity, like the detail: each selector of a repository is its own
/// row with its own manifest, and a bare namespace only reaches whichever row core prefers.
pub async fn fetch_arrow_detail(api: &dyn CoreApi, namespace: &str) -> Result<ArrowDetail> {
    let base = format!("/v0/arrow/{}", namespace_segment(namespace));
    let manifest_path = format!("{base}/manifest");
    let (detail, manifest) = tokio::join!(api.get::<ArrowDetailDto>(&base), api.get::<ArrowManifestDto>(&manifest_path));
    Ok(to_arrow_detail(detail?, manifest?, vec![], None, vec![], vec![]))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::core_api::{FakeCoreApi, Method};
    use crate::domain::{ChannelKind, DependencyType};

    const NS: &str = "github.com/rabbyte/minecraft@v1.21.4";
    const SEG: &str = "github.com%2Frabbyte%2Fminecraft%40v1.21.4";

    fn api() -> FakeCoreApi {
        FakeCoreApi::new()
    }

    fn routes_for_detail(api: &FakeCoreApi) {
        api.route(
            Method::Get,
            &format!("/v0/arrow/{SEG}"),
            json!({"namespace": NS, "name": "M", "description": "d", "state": "ready", "tags": null, "user_installed": true}),
        );
        api.route(
            Method::Get,
            &format!("/v0/arrow/{SEG}/manifest"),
            json!({"namespace": "github.com/rabbyte/minecraft", "name": "M", "tags": null, "variables": null, "targets": {}, "manifest": null}),
        );
    }

    #[tokio::test]
    async fn detail_fetches_only_detail_and_manifest_with_placeholders_for_the_slow_fields() {
        let api = api();
        routes_for_detail(&api);
        let d = fetch_arrow_detail(&api, NS).await.unwrap();
        assert_eq!(api.calls().len(), 2, "never readme/channels/dependencies/dependents");
        assert!(d.channels.is_empty() && d.readme.is_none() && d.dependencies.is_empty() && d.dependents.is_empty());
    }

    #[tokio::test]
    async fn detail_fails_when_the_manifest_fails() {
        let api = api();
        routes_for_detail(&api);
        api.route_error(Method::Get, &format!("/v0/arrow/{SEG}/manifest"), 500, "boom");
        assert!(fetch_arrow_detail(&api, NS).await.is_err());
    }

    #[tokio::test]
    async fn channels_read_the_bare_namespace_and_map() {
        let api = api();
        api.route(
            Method::Get,
            "/v0/arrow/github.com%2Frabbyte%2Fminecraft/channels",
            json!({"channels": [{"name": "stable", "kind": "ordered", "latest": "v2", "count": 1, "members": ["v2"]}]}),
        );
        let channels = fetch_channels(&api, NS).await.unwrap();
        assert_eq!((channels[0].kind, channels[0].latest.as_str()), (ChannelKind::Ordered, "v2"));
    }

    #[tokio::test]
    async fn readme_reads_the_identity_and_returns_the_prose() {
        let api = api();
        api.route(Method::Get, &format!("/v0/arrow/{SEG}/readme"), json!({"namespace": NS, "readme": "# Hi"}));
        assert_eq!(fetch_readme(&api, NS).await.unwrap().as_deref(), Some("# Hi"));
    }

    #[tokio::test]
    async fn dependencies_read_the_full_namespace_and_map() {
        let api = api();
        api.route(
            Method::Get,
            &format!("/v0/arrow/{SEG}/dependencies"),
            json!({"namespace": NS, "dependencies": [{"namespace": "java@21", "type": "tool"}]}),
        );
        let deps = fetch_dependencies(&api, NS).await.unwrap();
        assert_eq!((deps[0].namespace.as_str(), deps[0].dep_type), ("java@21", DependencyType::Tool));
    }

    #[tokio::test]
    async fn dependents_read_the_full_namespace() {
        let api = api();
        api.route(Method::Get, &format!("/v0/arrow/{SEG}/dependents"), json!({"namespace": NS, "dependents": ["a@1"]}));
        assert_eq!(fetch_dependents(&api, NS).await.unwrap(), vec!["a@1"]);
    }

    #[tokio::test]
    async fn each_optional_read_falls_back_on_a_404_and_errors_on_anything_else() {
        let api = api(); // no routes: every call 404s
        assert_eq!(fetch_readme(&api, NS).await.unwrap(), None);
        assert!(fetch_channels(&api, NS).await.unwrap().is_empty());
        assert!(fetch_dependencies(&api, NS).await.unwrap().is_empty());
        assert!(fetch_dependents(&api, NS).await.unwrap().is_empty());

        let broken = FakeCoreApi::new();
        for suffix in ["readme", "dependencies", "dependents"] {
            broken.route_error(Method::Get, &format!("/v0/arrow/{SEG}/{suffix}"), 500, "boom");
        }
        broken.route_error(Method::Get, "/v0/arrow/github.com%2Frabbyte%2Fminecraft/channels", 500, "boom");
        assert!(fetch_readme(&broken, NS).await.is_err());
        assert!(fetch_channels(&broken, NS).await.is_err());
        assert!(fetch_dependencies(&broken, NS).await.is_err());
        assert!(fetch_dependents(&broken, NS).await.is_err());
    }

    #[test]
    fn an_empty_namespace_is_not_fetched() {
        assert!(!is_enabled(""));
        assert!(is_enabled("a"));
    }
}

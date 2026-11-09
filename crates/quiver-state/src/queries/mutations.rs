//! The writes (`src/lib/core-store/mutations`): lifecycle verbs, catalog rows, collection follows.
//! Each is one daemon call; the results arrive over the streams, never from here.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use super::client::QueryClient;
use super::collection::collection_path;
use super::keys::collection_key;
use crate::core_api::{ApiResult, CoreApi, CoreApiExt, Method};
use crate::domain::{AvailableVersion, CollectionDetail};
use crate::namespace::namespace_segment;
use crate::Result;

pub type Variables = BTreeMap<String, String>;

/// What `POST /v0/runtime/:ns/update` did: `Started` (202) -- the update runs and reports over the
/// runtime stream, like any other lifecycle method -- or `Current` (200), an idempotent no-op:
/// nothing newer, and no runtime event will ever follow, so nothing may wait for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Started,
    Current,
}

/// `PATCH /v0/arrow/:ns` answer: what is ahead of the row's selector, if anything.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RecheckResult {
    #[serde(default)]
    pub available: Option<AvailableVersion>,
}

#[derive(Clone)]
pub struct Mutations {
    api: Arc<dyn CoreApi>,
    queries: QueryClient,
}

impl Mutations {
    pub fn new(api: Arc<dyn CoreApi>, queries: QueryClient) -> Self {
        Self { api, queries }
    }

    async fn runtime_request(
        &self,
        namespace: &str,
        method: &str,
        variables: &Variables,
    ) -> Result<ApiResult<Option<serde_json::Value>>> {
        let path = format!("/v0/runtime/{}/{method}", namespace_segment(namespace));
        Ok(self
            .api
            .request(Method::Post, &path, Some(json!({ "variables": variables })))
            .await?)
    }

    /// The plain request behind every lifecycle mutation, for callers that cannot hold state (a
    /// context menu on a list row).
    pub async fn runtime_method(
        &self,
        namespace: &str,
        method: &str,
        variables: &Variables,
    ) -> Result<()> {
        self.runtime_request(namespace, method, variables)
            .await
            .map(drop)
    }

    pub async fn install(&self, namespace: &str, variables: &Variables) -> Result<()> {
        self.runtime_method(namespace, "install", variables).await
    }

    pub async fn uninstall(&self, namespace: &str, variables: &Variables) -> Result<()> {
        self.runtime_method(namespace, "uninstall", variables).await
    }

    pub async fn stop(&self, namespace: &str, variables: &Variables) -> Result<()> {
        self.runtime_method(namespace, "stop", variables).await
    }

    pub async fn update(&self, namespace: &str, variables: &Variables) -> Result<UpdateOutcome> {
        let result = self.runtime_request(namespace, "update", variables).await?;
        Ok(if result.status == 202 {
            UpdateOutcome::Started
        } else {
            UpdateOutcome::Current
        })
    }

    /// The one universal "go" action -- `Target.Lifecycle.Execute`, always this exact call, never a
    /// custom method name. Hard-gated to `ready` only by core itself (`BeginExecution.Validate`);
    /// no manifest override is possible, so don't add a state check here that could drift from that.
    pub async fn execute_arrow(&self, namespace: &str, variables: &Variables) -> Result<()> {
        self.runtime_method(namespace, "execute", variables).await
    }

    /// A manifest-declared custom method (`backup`, `rcon`, ...).
    pub async fn execute(
        &self,
        namespace: &str,
        method: &str,
        variables: &Variables,
    ) -> Result<()> {
        self.runtime_method(namespace, method, variables).await
    }

    /// `POST /v0/arrow/:ns`, no body. `namespace` is the identity the new row is filed under: its
    /// selector (a channel, constraint, pinned ref or commit) travels in the path and never changes
    /// afterwards. A refless namespace is registered under the repository's default channel.
    pub async fn register_arrow(&self, namespace: &str) -> Result<()> {
        let path = format!("/v0/arrow/{}", namespace_segment(namespace));
        self.api
            .send(Method::Post, &path, None)
            .await
            .map(drop)
            .map_err(Into::into)
    }

    pub async fn remove_arrow(&self, namespace: &str) -> Result<()> {
        let path = format!("/v0/arrow/{}", namespace_segment(namespace));
        self.api
            .send(Method::Delete, &path, None)
            .await
            .map(drop)
            .map_err(Into::into)
    }

    /// `PATCH /v0/arrow/:ns`, no body: re-checks the row's selector against its repository and
    /// answers what is ahead, if anything. An installed row stays where it is; only the update call moves it.
    pub async fn recheck_arrow(&self, namespace: &str) -> Result<RecheckResult> {
        let path = format!("/v0/arrow/{}", namespace_segment(namespace));
        let data = self.api.send(Method::Patch, &path, None).await?;
        Ok(data
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default())
    }

    pub async fn follow_collection(&self, namespace: &str) -> Result<()> {
        self.set_collection_followed(Method::Post, true, namespace)
            .await
    }

    pub async fn unfollow_collection(&self, namespace: &str) -> Result<()> {
        self.set_collection_followed(Method::Delete, false, namespace)
            .await
    }

    /// Follow and unfollow are the same request shape either way the boolean goes: optimistic flip
    /// first, rollback on error. Both endpoints return no body to reconcile against, so the settle
    /// invalidates rather than trusts the optimistic value forever -- it's the only way this ever
    /// finds out the request landed on a `followed` quiver.core computed differently (a second
    /// tab, a server-side no-op).
    async fn set_collection_followed(
        &self,
        method: Method,
        followed: bool,
        namespace: &str,
    ) -> Result<()> {
        let key = collection_key(namespace);
        let previous: Option<CollectionDetail> = self.queries.get_data(&key);
        if let Some(prev) = &previous {
            let mut flipped = prev.clone();
            flipped.item.followed = followed;
            self.queries.set_data(&key, &flipped);
        }
        let path = format!("{}/follow", collection_path(namespace));
        let result = self.api.send(method, &path, None).await;
        if result.is_err() {
            if let Some(prev) = &previous {
                self.queries.set_data(&key, prev);
            }
        }
        self.queries.invalidate(&key);
        result.map(drop).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::core_api::FakeCoreApi;
    use crate::domain::{CollectionListItem, CollectionMedia};

    fn setup() -> (FakeCoreApi, QueryClient, Mutations) {
        let api = FakeCoreApi::new();
        let queries = QueryClient::new();
        let m = Mutations::new(Arc::new(api.clone()), queries.clone());
        (api, queries, m)
    }

    fn vars(pairs: &[(&str, &str)]) -> Variables {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[tokio::test]
    async fn lifecycle_verbs_post_to_the_runtime_path_with_a_variables_body() {
        let (api, _q, m) = setup();
        let ns = "github.com/rabbyte/minecraft@v1.21.4";
        let seg = "github.com%2Frabbyte%2Fminecraft%40v1.21.4";
        for verb in ["install", "uninstall", "stop", "execute", "backup"] {
            api.route_status(
                Method::Post,
                &format!("/v0/runtime/{seg}/{verb}"),
                202,
                None,
            );
        }
        m.install(ns, &vars(&[("K", "v")])).await.unwrap();
        m.uninstall(ns, &Variables::new()).await.unwrap();
        m.stop(ns, &Variables::new()).await.unwrap();
        m.execute_arrow(ns, &Variables::new()).await.unwrap();
        m.execute(ns, "backup", &Variables::new()).await.unwrap();
        let calls = api.calls();
        assert_eq!(calls[0].path, format!("/v0/runtime/{seg}/install"));
        assert_eq!(calls[0].body, Some(json!({"variables": {"K": "v"}})));
        assert_eq!(
            calls[1].body,
            Some(json!({"variables": {}})),
            "variables default to an empty object"
        );
        assert_eq!(calls[3].path, format!("/v0/runtime/{seg}/execute"));
        assert_eq!(calls[4].path, format!("/v0/runtime/{seg}/backup"));
        assert!(calls.iter().all(|c| c.method == Method::Post));
    }

    #[tokio::test]
    async fn update_tells_a_started_202_from_a_nothing_newer_200() {
        let (api, _q, m) = setup();
        api.route_status(Method::Post, "/v0/runtime/a/update", 202, None);
        assert_eq!(
            m.update("a", &Variables::new()).await.unwrap(),
            UpdateOutcome::Started
        );
        api.route_status(Method::Post, "/v0/runtime/a/update", 200, None);
        assert_eq!(
            m.update("a", &Variables::new()).await.unwrap(),
            UpdateOutcome::Current
        );
    }

    #[tokio::test]
    async fn register_remove_and_recheck_address_the_identity_as_one_path_segment() {
        let (api, _q, m) = setup();
        for ns in [
            "github.com/x/y@beta",
            "github.com/x/y@feat/x",
            "github.com/x/y@v1.*",
        ] {
            api.route_status(
                Method::Post,
                &format!("/v0/arrow/{}", namespace_segment(ns)),
                200,
                None,
            );
            m.register_arrow(ns).await.unwrap();
        }
        let paths: Vec<_> = api.calls().into_iter().map(|c| c.path).collect();
        assert_eq!(
            paths,
            vec![
                "/v0/arrow/github.com%2Fx%2Fy%40beta",
                "/v0/arrow/github.com%2Fx%2Fy%40feat%2Fx",
                "/v0/arrow/github.com%2Fx%2Fy%40v1.*"
            ]
        );
        assert!(
            api.calls().iter().all(|c| c.body.is_none()),
            "the selector lives in the path"
        );

        api.route_status(
            Method::Delete,
            "/v0/arrow/github.com%2Fx%2Fy%40v1",
            204,
            None,
        );
        m.remove_arrow("github.com/x/y@v1").await.unwrap();
        api.route(
            Method::Patch,
            "/v0/arrow/n",
            json!({"available": {"ref": "v2", "commit": "c"}}),
        );
        assert_eq!(
            m.recheck_arrow("n")
                .await
                .unwrap()
                .available
                .unwrap()
                .reference,
            "v2"
        );
        api.route_status(Method::Patch, "/v0/arrow/n", 200, None);
        assert!(m.recheck_arrow("n").await.unwrap().available.is_none());
    }

    fn detail(followed: bool) -> CollectionDetail {
        CollectionDetail {
            item: CollectionListItem {
                namespace: "github.com/r/col".into(),
                name: "Col".into(),
                description: String::new(),
                tags: vec![],
                followed,
                arrow_count: 0,
            },
            url: None,
            maintainers: vec![],
            media: CollectionMedia::default(),
            arrows: vec![],
        }
    }

    const FOLLOW: &str = "/v0/collection/github.com%2Fr%2Fcol/follow";
    const NS: &str = "github.com/r/col";

    #[tokio::test]
    async fn follow_flips_the_cache_optimistically_and_invalidates_once_settled() {
        let (api, q, m) = setup();
        api.route_status(Method::Post, FOLLOW, 204, None);
        q.set_data(&collection_key(NS), &detail(false));
        m.follow_collection(NS).await.unwrap();
        assert!(
            q.get_data::<CollectionDetail>(&collection_key(NS))
                .unwrap()
                .item
                .followed
        );
        assert_eq!(
            q.epoch(&collection_key(NS)),
            1,
            "neither endpoint returns a body to reconcile against"
        );
    }

    #[tokio::test]
    async fn the_optimistic_flip_is_visible_while_the_request_is_in_flight() {
        let (api, q, m) = setup();
        let observed = Arc::new(std::sync::Mutex::new(None));
        let (sink, probe) = (observed.clone(), q.clone());
        api.route_fn(Method::Post, FOLLOW, move |_| {
            *sink.lock().unwrap() = probe
                .get_data::<CollectionDetail>(&collection_key(NS))
                .map(|d| d.item.followed);
            async {
                Ok(ApiResult {
                    status: 204,
                    data: None,
                })
            }
        });
        q.set_data(&collection_key(NS), &detail(false));
        m.follow_collection(NS).await.unwrap();
        assert_eq!(*observed.lock().unwrap(), Some(true));
    }

    #[tokio::test]
    async fn unfollow_deletes_and_flips_to_false() {
        let (api, q, m) = setup();
        api.route_status(Method::Delete, FOLLOW, 204, None);
        q.set_data(&collection_key(NS), &detail(true));
        m.unfollow_collection(NS).await.unwrap();
        assert!(
            !q.get_data::<CollectionDetail>(&collection_key(NS))
                .unwrap()
                .item
                .followed
        );
    }

    #[tokio::test]
    async fn a_failed_request_rolls_the_flip_back_and_still_invalidates() {
        let (api, q, m) = setup();
        api.route_error(Method::Post, FOLLOW, 500, "boom");
        q.set_data(&collection_key(NS), &detail(false));
        assert!(m.follow_collection(NS).await.is_err());
        assert!(
            !q.get_data::<CollectionDetail>(&collection_key(NS))
                .unwrap()
                .item
                .followed
        );
        assert_eq!(q.epoch(&collection_key(NS)), 1);
    }

    #[tokio::test]
    async fn nothing_cached_means_nothing_to_flip_or_roll_back() {
        let (api, q, m) = setup();
        api.route_error(Method::Post, FOLLOW, 500, "boom");
        assert!(m.follow_collection(NS).await.is_err());
        assert_eq!(q.get_data::<Value>(&collection_key(NS)), None);
        api.route_status(Method::Post, FOLLOW, 204, None);
        m.follow_collection(NS).await.unwrap();
        assert_eq!(q.get_data::<Value>(&collection_key(NS)), None);
    }
}

//! The Engine tab's calls to Core: read `GET /v0/config`, patch it, read and set up `PATH`. Decisions on
//! what the answers mean are in `engine` / `path`; this only moves them (port of `engine-api.ts`,
//! `path-api.ts` and the engine store's load/patch).

use serde_json::Value;

use super::engine::{parse_patch_response, ConfigView, PatchResult, Rejection};
use super::path::PathStatus;
use crate::arrow_details::invoke::ActionFailure;
use crate::core_api::{CoreApi, CoreApiExt, Method};
use crate::dto::path::{to_path_status, PathStatusDto};
use crate::transport::api::RawFetch;

const CONFIG_PATH: &str = "/v0/config";
const PATH_PATH: &str = "/v0/system/path";

pub async fn load_config(api: &dyn CoreApi) -> Result<ConfigView, String> {
    let value = api.get_json(CONFIG_PATH).await.map_err(|e| e.to_string())?;
    ConfigView::from_json(&value).ok_or_else(|| "the daemon sent an unreadable config".to_owned())
}

/// Core answers a fully rejected patch with a 422 whose body still carries the per-key reasons, so this
/// reads the raw envelope instead of going through the usual "non-2xx is an error" rule.
pub async fn patch_config(
    raw: &dyn RawFetch,
    patch: &Value,
) -> Result<PatchResult, ActionFailure> {
    let response = raw
        .fetch(Method::Patch, CONFIG_PATH, Some(patch.to_string()))
        .await
        .map_err(|e| ActionFailure {
            status: Some(e.status).filter(|s| *s != 0),
            message: e.message,
        })?;
    let body: Option<Value> = serde_json::from_str(&response.body).ok();
    parse_patch_response(response.status, &response.status_text, body.as_ref())
}

/// Patches, then re-reads: the daemon owns `restart_required` and `corrected`, so the answer is never
/// merged in by hand.
pub async fn patch_and_reload(
    raw: &dyn RawFetch,
    api: &dyn CoreApi,
    patch: &Value,
) -> Result<(Vec<Rejection>, ConfigView), String> {
    let result = patch_config(raw, patch).await.map_err(|f| f.message)?;
    let view = load_config(api).await?;
    Ok((result.rejected, view))
}

fn path_status(value: Option<Value>) -> Result<PathStatus, ActionFailure> {
    let dto: PathStatusDto =
        serde_json::from_value(value.unwrap_or(Value::Null)).map_err(|e| ActionFailure {
            status: None,
            message: e.to_string(),
        })?;
    let status = to_path_status(dto);
    Ok(PathStatus {
        bin_dir: status.bin_dir,
        on_path: status.on_path,
        configured: status.configured,
        files: status.files,
    })
}

fn failure(error: crate::core_api::ApiError) -> ActionFailure {
    ActionFailure {
        status: Some(error.status).filter(|s| *s != 0),
        message: error.message,
    }
}

pub async fn load_path(api: &dyn CoreApi) -> Result<PathStatus, ActionFailure> {
    path_status(
        api.send(Method::Get, PATH_PATH, None)
            .await
            .map_err(failure)?,
    )
}

/// Only ever called from an explicit user action: Core never edits `PATH` on its own.
pub async fn setup_path(api: &dyn CoreApi) -> Result<PathStatus, ActionFailure> {
    path_status(
        api.send(Method::Post, PATH_PATH, None)
            .await
            .map_err(failure)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_api::FakeCoreApi;
    use crate::transport::api::RawResponse;
    use serde_json::json;

    struct Canned(u16, &'static str);

    #[async_trait::async_trait]
    impl RawFetch for Canned {
        async fn fetch(
            &self,
            _: Method,
            _: &str,
            _: Option<String>,
        ) -> Result<RawResponse, crate::core_api::ApiError> {
            Ok(RawResponse {
                status: self.0,
                status_text: "x".into(),
                proxy_error: false,
                body: self.1.into(),
            })
        }
    }

    fn config() -> Value {
        json!({"running": {}, "configured": {"logger": {"level": "info"}}, "defaults": {}})
    }

    #[tokio::test]
    async fn a_rejected_patch_keeps_its_reasons_and_the_view_is_re_read() {
        let api = FakeCoreApi::new();
        api.route(Method::Get, CONFIG_PATH, config());
        let raw = Canned(
            422,
            r#"{"success":false,"error":null,"data":{"applied":[],"rejected":[{"key":"logger.level","message":"unusable"}]}}"#,
        );
        let (rejected, view) = patch_and_reload(&raw, &api, &json!({"logger": {"level": "x"}}))
            .await
            .unwrap();
        assert_eq!(rejected[0].message, "unusable");
        assert_eq!(view.configured["logger"]["level"], "info");
        assert_eq!(api.calls_to(Method::Get, CONFIG_PATH), 1);
    }

    #[tokio::test]
    async fn a_body_that_is_not_a_patch_result_is_a_failure_with_the_status() {
        let err = patch_config(&Canned(502, "<html>"), &json!({}))
            .await
            .unwrap_err();
        assert_eq!(err.status, Some(502));
    }

    #[tokio::test]
    async fn path_status_loads_and_a_404_is_reported_by_status() {
        let api = FakeCoreApi::new();
        api.route(
            Method::Get,
            PATH_PATH,
            json!({"bin_dir": "/b", "on_path": false, "configured": true, "files": ["/f"]}),
        );
        let status = load_path(&api).await.unwrap();
        assert!(status.configured && !status.on_path);
        let none = FakeCoreApi::new();
        none.route_error(Method::Get, PATH_PATH, 404, "not found");
        assert_eq!(load_path(&none).await.unwrap_err().status, Some(404));
    }
}

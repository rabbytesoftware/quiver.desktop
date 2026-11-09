//! The response envelope and retry rules of `src/lib/transport/api.ts`, over a raw fetch.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::core_api::{ApiError, ApiResult, Method};

/// Header the shell's proxy sets when ITS upstream is down (as opposed to the daemon answering 502).
pub const PROXY_ERROR_HEADER: &str = "x-quiver-proxy";

/// One HTTP answer, before any interpretation.
#[derive(Debug, Clone)]
pub struct RawResponse {
    pub status: u16,
    pub status_text: String,
    /// The proxy marked this failure (`x-quiver-proxy: error`).
    pub proxy_error: bool,
    pub body: String,
}

impl RawResponse {
    fn is_ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    fn is_proxy_failure(&self) -> bool {
        (self.status == 502 || self.status == 504) && self.proxy_error
    }
}

#[async_trait]
pub trait RawFetch: Send + Sync {
    /// Err only when no answer came back at all.
    async fn fetch(&self, method: Method, path: &str, body: Option<String>) -> Result<RawResponse, ApiError>;
}

#[async_trait]
pub trait Sleeper: Send + Sync {
    async fn sleep(&self, duration: Duration);
}

pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

#[derive(Clone)]
pub struct RetryConfig {
    pub attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub sleeper: Arc<dyn Sleeper>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            attempts: 8,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000),
            sleeper: Arc::new(TokioSleeper),
        }
    }
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    data: Option<Value>,
}

/// `apiFetch` with the response status kept beside the data. Only an idempotent read (GET) is ever
/// retried, and only on a proxy-marked 502/504.
pub async fn api_request(
    raw: &dyn RawFetch,
    method: Method,
    path: &str,
    body: Option<String>,
    retry: &RetryConfig,
) -> Result<ApiResult<Option<Value>>, ApiError> {
    let max_attempts = if method == Method::Get { retry.attempts.max(1) } else { 1 };
    let mut attempt = 1u32;
    loop {
        let res = raw.fetch(method, path, body.clone()).await?;
        if res.is_proxy_failure() {
            if attempt < max_attempts {
                let backoff = retry.base_delay.saturating_mul(1u32 << (attempt - 1).min(20));
                retry.sleeper.sleep(backoff.min(retry.max_delay)).await;
                attempt += 1;
                continue;
            }
            return Err(ApiError::new(res.body, res.status));
        }
        // A body that is not an envelope (or not JSON at all) reads as no body.
        let envelope: Option<Envelope> = serde_json::from_str::<Option<Envelope>>(&res.body).ok().flatten();
        if res.is_ok() && (res.status == 204 || envelope.is_none()) {
            return Ok(ApiResult { status: res.status, data: None });
        }
        // A 202 may carry an envelope -- POST /v0/search/discover answers with the job ticket
        // before any provider has been asked, and the caller needs it. A 202 carrying anything
        // else is a bare acknowledgement.
        let success = envelope.as_ref().is_some_and(|e| e.success);
        if res.status == 202 && !success {
            return Ok(ApiResult { status: res.status, data: None });
        }
        if !res.is_ok() || !success {
            let message = envelope
                .and_then(|e| e.error)
                .unwrap_or_else(|| format!("{} {}", res.status, res.status_text));
            return Err(ApiError::new(message, res.status));
        }
        return Ok(ApiResult { status: res.status, data: envelope.and_then(|e| e.data) });
    }
}

/// `GET /v0/health`, once, no retry. The daemon's health body is not enveloped, so only the status counts.
pub async fn core_is_reachable(raw: &dyn RawFetch) -> bool {
    matches!(raw.fetch(Method::Get, "/v0/health", None).await, Ok(res) if res.is_ok())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    struct Script {
        replies: Mutex<VecDeque<Result<RawResponse, ApiError>>>,
        calls: Mutex<u32>,
    }

    impl Script {
        fn new(replies: Vec<Result<RawResponse, ApiError>>) -> Self {
            Self { replies: Mutex::new(replies.into()), calls: Mutex::new(0) }
        }

        fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    #[async_trait]
    impl RawFetch for Script {
        async fn fetch(&self, _m: Method, _p: &str, _b: Option<String>) -> Result<RawResponse, ApiError> {
            *self.calls.lock().unwrap() += 1;
            self.replies.lock().unwrap().pop_front().unwrap_or_else(|| Err(ApiError::transport("script empty")))
        }
    }

    struct Recorder(Mutex<Vec<Duration>>);

    #[async_trait]
    impl Sleeper for Recorder {
        async fn sleep(&self, d: Duration) {
            self.0.lock().unwrap().push(d);
        }
    }

    fn res(status: u16, body: &str) -> RawResponse {
        RawResponse { status, status_text: "Text".into(), proxy_error: false, body: body.into() }
    }

    fn proxy(status: u16) -> RawResponse {
        RawResponse { proxy_error: true, ..res(status, "proxy down") }
    }

    fn config(attempts: u32) -> (RetryConfig, Arc<Recorder>) {
        let rec = Arc::new(Recorder(Mutex::new(vec![])));
        let cfg = RetryConfig {
            attempts,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000),
            sleeper: rec.clone(),
        };
        (cfg, rec)
    }

    async fn get(script: &Script, cfg: &RetryConfig) -> Result<ApiResult<Option<Value>>, ApiError> {
        api_request(script, Method::Get, "/v0/x", None, cfg).await
    }

    #[tokio::test]
    async fn unwraps_a_successful_envelope() {
        let s = Script::new(vec![Ok(res(200, r#"{"success":true,"error":null,"data":{"a":1}}"#))]);
        let out = get(&s, &config(3).0).await.unwrap();
        assert_eq!(out.data, Some(serde_json::json!({"a":1})));
    }

    #[tokio::test]
    async fn throws_the_envelope_error_message() {
        let s = Script::new(vec![Ok(res(400, r#"{"success":false,"error":"bad"}"#))]);
        let err = get(&s, &config(3).0).await.unwrap_err();
        assert_eq!((err.status, err.message.as_str()), (400, "bad"));
    }

    #[tokio::test]
    async fn retries_an_idempotent_read_on_a_proxy_marked_502_with_capped_backoff() {
        let s = Script::new(vec![
            Ok(proxy(502)),
            Ok(proxy(504)),
            Ok(res(200, r#"{"success":true,"data":1}"#)),
        ]);
        let (cfg, rec) = config(8);
        assert_eq!(get(&s, &cfg).await.unwrap().data, Some(Value::from(1)));
        assert_eq!(s.calls(), 3);
        assert_eq!(*rec.0.lock().unwrap(), vec![Duration::from_millis(100), Duration::from_millis(200)]);
    }

    #[tokio::test]
    async fn backoff_never_exceeds_the_max() {
        let s = Script::new((0..6).map(|_| Ok(proxy(502))).collect());
        let (cfg, rec) = config(6);
        assert!(get(&s, &cfg).await.is_err());
        let slept = rec.0.lock().unwrap().clone();
        assert_eq!(slept.last(), Some(&Duration::from_millis(1000)));
        assert_eq!(slept.len(), 5);
    }

    #[tokio::test]
    async fn does_not_retry_an_unmarked_502() {
        let s = Script::new(vec![Ok(res(502, r#"{"success":false,"error":"upstream"}"#))]);
        assert_eq!(get(&s, &config(3).0).await.unwrap_err().status, 502);
        assert_eq!(s.calls(), 1);
    }

    #[tokio::test]
    async fn never_retries_a_mutation_even_when_marked() {
        let s = Script::new(vec![Ok(proxy(502)), Ok(res(200, r#"{"success":true}"#))]);
        let err = api_request(&s, Method::Post, "/v0/x", None, &config(3).0).await.unwrap_err();
        assert_eq!(err.status, 502);
        assert_eq!(s.calls(), 1);
    }

    #[tokio::test]
    async fn gives_up_after_the_configured_attempts() {
        let s = Script::new((0..5).map(|_| Ok(proxy(502))).collect());
        assert!(get(&s, &config(3).0).await.is_err());
        assert_eq!(s.calls(), 3);
    }

    #[tokio::test]
    async fn surfaces_a_404_as_a_terminal_error_with_the_status_line_fallback() {
        let s = Script::new(vec![Ok(res(404, "not json"))]);
        let err = get(&s, &config(3).0).await.unwrap_err();
        assert!(err.is_not_found());
        assert_eq!(err.message, "404 Text");
    }

    #[tokio::test]
    async fn treats_204_and_a_non_envelope_2xx_as_success_with_no_payload() {
        let s = Script::new(vec![Ok(res(204, "")), Ok(res(200, "null")), Ok(res(200, "garbage"))]);
        let cfg = config(3).0;
        for _ in 0..3 {
            assert_eq!(get(&s, &cfg).await.unwrap().data, None);
        }
    }

    #[tokio::test]
    async fn treats_202_as_success_even_with_a_non_envelope_body() {
        let s = Script::new(vec![Ok(res(202, "accepted"))]);
        let out = get(&s, &config(3).0).await.unwrap();
        assert_eq!((out.status, out.data), (202, None));
    }

    #[tokio::test]
    async fn a_202_carrying_an_envelope_keeps_its_ticket() {
        let s = Script::new(vec![Ok(res(202, r#"{"success":true,"data":{"job_id":"j"}}"#))]);
        let out = get(&s, &config(3).0).await.unwrap();
        assert_eq!(out.status, 202);
        assert_eq!(out.data, Some(serde_json::json!({"job_id":"j"})));
    }

    #[tokio::test]
    async fn reports_the_status_beside_the_data_so_a_200_noop_differs_from_a_202() {
        let s = Script::new(vec![Ok(res(200, r#"{"success":true}"#))]);
        assert_eq!(get(&s, &config(3).0).await.unwrap().status, 200);
    }

    #[tokio::test]
    async fn a_state_violation_carries_its_status() {
        let s = Script::new(vec![Ok(res(409, r#"{"success":false,"error":"wrong state"}"#))]);
        let err = get(&s, &config(3).0).await.unwrap_err();
        assert_eq!((err.status, err.message.as_str()), (409, "wrong state"));
    }

    #[tokio::test]
    async fn reachability_counts_status_only_and_never_retries() {
        let s = Script::new(vec![Ok(res(200, "ok not json")), Ok(proxy(502)), Err(ApiError::transport("down"))]);
        assert!(core_is_reachable(&s).await);
        assert!(!core_is_reachable(&s).await);
        assert!(!core_is_reachable(&s).await);
        assert_eq!(s.calls(), 3);
    }
}

//! GPUI's HTTP client: what `img(url)` fetches an arrow's icon and banner with (`media.icon`, `media.banner`,
//! the README's images). GPUI ships none, so without one every remote image failed with "Failed to load asset"
//! and only the drawn stand-ins showed.
//!
//! Requests run on the app's tokio runtime (reqwest needs its reactor; GPUI polls the returned future on its
//! own executor) and the body is read whole, capped, since an image is decoded in one piece anyway.

use std::sync::Arc;
use std::time::Duration;

use futures_util::future::BoxFuture;
use gpui::http_client::{self, AsyncBody, HttpClient, Request, Response, Url};
use tokio::runtime::Handle;

/// Larger answers are refused rather than buffered: no icon or banner is near this.
const MAX_BYTES: usize = 32 * 1024 * 1024;

pub struct ReqwestHttp {
    client: reqwest::Client,
    handle: Handle,
    agent: http_client::http::HeaderValue,
}

impl ReqwestHttp {
    pub fn new(handle: Handle) -> Arc<Self> {
        let agent = concat!("quiver.desktop/", env!("CARGO_PKG_VERSION"));
        let client = reqwest::Client::builder()
            .user_agent(agent)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Arc::new(Self {
            client,
            handle,
            agent: http_client::http::HeaderValue::from_static(agent),
        })
    }
}

async fn fetch(
    client: reqwest::Client,
    method: reqwest::Method,
    url: String,
    headers: Vec<(String, Vec<u8>)>,
) -> http_client::Result<(u16, Vec<(String, Vec<u8>)>, Vec<u8>)> {
    let mut request = client.request(method, &url);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    let mut response = request.send().await?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(n, v)| (n.as_str().to_owned(), v.as_bytes().to_vec()))
        .collect();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err(http_client::anyhow!(
                "{url} is larger than {MAX_BYTES} bytes"
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok((status, headers, body))
}

impl HttpClient for ReqwestHttp {
    fn type_name(&self) -> &'static str {
        "quiver-reqwest"
    }

    fn user_agent(&self) -> Option<&http_client::http::HeaderValue> {
        Some(&self.agent)
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(
        &self,
        request: Request<AsyncBody>,
    ) -> BoxFuture<'static, http_client::Result<Response<AsyncBody>>> {
        let (parts, _body) = request.into_parts();
        let client = self.client.clone();
        let url = parts.uri.to_string();
        let headers = parts
            .headers
            .iter()
            .map(|(n, v)| (n.as_str().to_owned(), v.as_bytes().to_vec()))
            .collect();
        let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes());
        let task = method.map(|method| self.handle.spawn(fetch(client, method, url, headers)));
        Box::pin(async move {
            let (status, headers, body) = task?.await??;
            let mut response = Response::builder().status(status);
            for (name, value) in headers {
                response = response.header(name, value);
            }
            Ok(response.body(AsyncBody::from(body))?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_connection_is_an_error_not_a_hang() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = ReqwestHttp::new(runtime.handle().clone());
        let request = Request::builder()
            .method("GET")
            .uri("http://127.0.0.1:9/")
            .body(AsyncBody::from(()))
            .unwrap();
        // Nothing listens on the discard port; the future is polled off the runtime, as GPUI does.
        let result = futures_util::FutureExt::now_or_never(client.send(request));
        assert!(result.is_none() || matches!(result, Some(Err(_))));
        let result = std::thread::spawn({
            let client = ReqwestHttp::new(runtime.handle().clone());
            move || {
                futures_executor_block_on(
                    client.send(
                        Request::builder()
                            .uri("http://127.0.0.1:9/")
                            .body(AsyncBody::from(()))
                            .unwrap(),
                    ),
                )
            }
        })
        .join()
        .unwrap();
        assert!(result.is_err());
    }

    /// A minimal executor that is not tokio's, standing in for GPUI's.
    fn futures_executor_block_on<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        loop {
            if let std::task::Poll::Ready(out) = future.as_mut().poll(&mut cx) {
                return out;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

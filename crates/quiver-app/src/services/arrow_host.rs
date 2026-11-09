//! The arrow host's upstream: every arrow page request goes to the daemon behind the ACTIVE
//! connection. The bearer token is added by the transport, in Rust, so it never reaches a page.

use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use http::{HeaderName, HeaderValue, Request};
use quiver_arrow_host::{
    ArrowRequest, ArrowResponse, ArrowUpstream, Error, Headers, Result, WsRelay,
};
use quiver_core_client::transport::WsStream;
use quiver_core_client::ConnectionManager;
use tokio_tungstenite::tungstenite::Message;

pub struct CoreUpstream {
    manager: Arc<ConnectionManager>,
}

impl CoreUpstream {
    pub fn new(manager: Arc<ConnectionManager>) -> Self {
        Self { manager }
    }
}

/// `req.uri` is already the daemon path (`/v0/ui/<ns>/...`).
fn daemon_request(req: ArrowRequest) -> Result<Request<Vec<u8>>> {
    let mut builder = Request::builder()
        .method(req.method.as_str())
        .uri(format!("quiver://localhost{}", req.uri));
    for (name, value) in req.headers.iter() {
        // A header the HTTP types reject is dropped rather than failing the page's request.
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            builder = builder.header(name, value);
        }
    }
    builder
        .body(req.body)
        .map_err(|e| Error::Upstream(format!("bad request: {e}")))
}

#[async_trait]
impl ArrowUpstream for CoreUpstream {
    async fn request(&self, req: ArrowRequest) -> Result<ArrowResponse> {
        let request = daemon_request(req)?;
        let resp = self
            .manager
            .transport()
            .await
            .request(request)
            .await
            .map_err(|e| Error::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers: Headers = resp
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
            })
            .collect();
        Ok(ArrowResponse {
            status,
            headers,
            body: resp.into_body(),
        })
    }

    async fn open_ws(&self, path: &str) -> Result<Box<dyn WsRelay>> {
        let socket = self
            .manager
            .transport()
            .await
            .open_ws(path)
            .await
            .map_err(|e| Error::Upstream(e.to_string()))?;
        Ok(Box::new(SocketRelay { socket }))
    }
}

/// Text frames in both directions; everything else is dropped, as the bridge does.
struct SocketRelay {
    socket: WsStream,
}

#[async_trait]
impl WsRelay for SocketRelay {
    async fn send(&mut self, text: String) -> Result<()> {
        self.socket
            .send(Message::Text(text))
            .await
            .map_err(|e| Error::Upstream(e.to_string()))
    }

    async fn recv(&mut self) -> Option<String> {
        loop {
            match self.socket.next().await {
                Some(Ok(Message::Text(text))) => return Some(text),
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return None,
                Some(Ok(_)) => {}
            }
        }
    }

    async fn close(&mut self) {
        let _ = self.socket.close(None).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_daemon_path_headers_and_body_reach_the_transport_request() {
        let mut headers = Headers::new();
        headers.append("Accept", "text/html");
        headers.append("bad header name", "x");
        let req = daemon_request(ArrowRequest {
            method: "POST".into(),
            uri: "/v0/ui/ns/a?b=1".into(),
            headers,
            body: b"hi".to_vec(),
        })
        .unwrap();
        assert_eq!(req.method(), "POST");
        assert_eq!(req.uri().path_and_query().unwrap(), "/v0/ui/ns/a?b=1");
        assert_eq!(req.headers().get("accept").unwrap(), "text/html");
        assert_eq!(req.headers().len(), 1);
        assert_eq!(req.body(), b"hi");
    }

    #[test]
    fn a_nonsense_method_is_an_upstream_error_not_a_panic() {
        let bad = ArrowRequest {
            method: "NOT A METHOD".into(),
            ..ArrowRequest::get("/v0/ui/ns/")
        };
        assert!(matches!(daemon_request(bad), Err(Error::Upstream(_))));
    }
}

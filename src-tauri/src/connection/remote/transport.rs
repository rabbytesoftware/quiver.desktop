use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;

use crate::connection::http::{parse_api_error, HttpError, HttpTransport};

pub struct RemoteTransport {
	client: Client,
	base_url: String,
}

impl RemoteTransport {
	pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
		let token = token.into();
		let mut headers = reqwest::header::HeaderMap::new();
		let mut auth = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
			.expect("token must be a valid header value");
		auth.set_sensitive(true);
		headers.insert(reqwest::header::AUTHORIZATION, auth);

		let client = Client::builder()
			.default_headers(headers)
			.build()
			.expect("failed to build reqwest client");

		Self {
			client,
			base_url: base_url.into(),
		}
	}

	fn http_url(&self, path: &str) -> String {
		let base = self.base_url.replace("tcp://", "http://");
		format!("{}{}", base, path)
	}

	pub fn base_ws_url(&self) -> String {
		self.base_url.replace("tcp://", "ws://")
	}
}

#[async_trait]
impl HttpTransport for RemoteTransport {
	async fn get_bytes(&self, path: &str) -> Result<Bytes, HttpError> {
		let resp = self
			.client
			.get(self.http_url(path))
			.send()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let code = resp.status().as_u16();
		let bytes = resp
			.bytes()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		if code >= 400 {
			return Err(HttpError::Api {
				code,
				message: parse_api_error(&bytes),
			});
		}
		Ok(bytes)
	}

	async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), HttpError> {
		let resp = self
			.client
			.post(self.http_url(path))
			.json(&body)
			.send()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let code = resp.status().as_u16();
		if resp.status().is_success() {
			return Ok(());
		}
		let bytes = resp
			.bytes()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		Err(HttpError::Api {
			code,
			message: parse_api_error(&bytes),
		})
	}

	async fn delete(&self, path: &str) -> Result<(), HttpError> {
		let resp = self
			.client
			.delete(self.http_url(path))
			.send()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let code = resp.status().as_u16();
		if resp.status().is_success() {
			return Ok(());
		}
		let bytes = resp
			.bytes()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		Err(HttpError::Api {
			code,
			message: parse_api_error(&bytes),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A transport pointed at a port nothing is listening on, so every request
	/// fails at connect time. Binding and immediately dropping the listener is
	/// what makes the port both free and known-unused.
	async fn unreachable() -> RemoteTransport {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind");
		let addr = listener.local_addr().expect("addr");
		drop(listener);
		RemoteTransport::new(format!("tcp://{addr}"), "tok")
	}

	/// Answers with a status line promising more body than it sends, then hangs
	/// up. reqwest reads the headers fine and then fails partway through
	/// `.bytes()`, which is the only way to reach the body-read error arms.
	async fn truncates_its_body(status: u16) -> RemoteTransport {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			while let Ok((mut sock, _)) = listener.accept().await {
				use tokio::io::AsyncWriteExt;
				let _ = sock
					.write_all(
						format!(
							"HTTP/1.1 {status} X\r\nContent-Length: 64\r\n\r\nshort"
						)
						.as_bytes(),
					)
					.await;
				let _ = sock.shutdown().await;
			}
		});
		RemoteTransport::new(format!("tcp://{addr}"), "tok")
	}

	#[test]
	fn http_url_replaces_tcp_scheme() {
		let t = RemoteTransport::new("tcp://10.0.0.1:40257", "tok");
		assert!(t.http_url("/health").starts_with("http://"));
	}

	#[test]
	fn ws_url_replaces_tcp_scheme() {
		let t = RemoteTransport::new("tcp://10.0.0.1:40257", "tok");
		assert!(t.base_ws_url().starts_with("ws://"));
	}

	// A remote daemon that is simply not there is the ordinary failure for this
	// transport — a laptop that slept, a tunnel that dropped. It has to surface
	// as Request (the connection is at fault) and never as Api (the daemon
	// answered), because the two lead a caller to very different conclusions.

	#[tokio::test]
	async fn get_reports_an_unreachable_host_as_a_request_error() {
		let err = unreachable()
			.await
			.get_bytes("/v0/health")
			.await
			.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}

	#[tokio::test]
	async fn post_reports_an_unreachable_host_as_a_request_error() {
		let err = unreachable()
			.await
			.post_json("/v0/arrow/x", serde_json::Value::Null)
			.await
			.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}

	#[tokio::test]
	async fn delete_reports_an_unreachable_host_as_a_request_error() {
		let err = unreachable().await.delete("/v0/arrow/x").await.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}

	// A connection that dies mid-body is not a 4xx/5xx from the daemon, and
	// must not be reported as one.

	#[tokio::test]
	async fn get_reports_a_truncated_body_as_a_request_error() {
		let err = truncates_its_body(200)
			.await
			.get_bytes("/v0/arrow")
			.await
			.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}

	#[tokio::test]
	async fn post_reports_a_truncated_error_body_as_a_request_error() {
		// 500 so the success short-circuit does not fire and the body is read.
		let err = truncates_its_body(500)
			.await
			.post_json("/v0/arrow/x", serde_json::Value::Null)
			.await
			.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}

	#[tokio::test]
	async fn delete_reports_a_truncated_error_body_as_a_request_error() {
		let err = truncates_its_body(500)
			.await
			.delete("/v0/arrow/x")
			.await
			.unwrap_err();
		assert!(
			matches!(err, HttpError::Request(_)),
			"expected Request, got {err:?}"
		);
	}
}

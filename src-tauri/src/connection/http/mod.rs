use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
	#[error("request failed: {0}")]
	Request(String),
	#[error("api error: {code} {message}")]
	Api { code: u16, message: String },
	#[error("parse error: {0}")]
	Parse(#[from] serde_json::Error),
}

// ── Transport trait ───────────────────────────────────────────────────────────

#[async_trait]
pub trait HttpTransport: Send + Sync {
	async fn get_bytes(&self, path: &str) -> Result<Bytes, HttpError>;
	async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), HttpError>;
	async fn delete(&self, path: &str) -> Result<(), HttpError>;
}

// ── HttpClient ────────────────────────────────────────────────────────────────

pub struct HttpClient {
	transport: Arc<dyn HttpTransport>,
}

impl HttpClient {
	pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
		Self { transport }
	}

	pub async fn get_raw_bytes(&self, path: &str) -> Result<Bytes, HttpError> {
		self.transport.get_bytes(path).await
	}

	async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
		let bytes = self.transport.get_bytes(path).await?;
		let envelope: ApiEnvelope<T> = serde_json::from_slice(&bytes)?;
		envelope.data.ok_or_else(|| HttpError::Api {
			code: 500,
			message: envelope.error.unwrap_or_default(),
		})
	}

	pub async fn health(&self) -> Result<(), HttpError> {
		self.transport.get_bytes("/v0/health").await.map(|_| ())
	}

	pub async fn fetch_arrows(&self) -> Result<serde_json::Value, HttpError> {
		self.get_json("/v0/arrow?user_installed=true").await
	}

	pub async fn get_arrow_detail(
		&self,
		namespace: &str,
	) -> Result<serde_json::Value, HttpError> {
		let path = format!("/v0/arrow/{}", urlencoded(namespace));
		self.get_json(&path).await
	}

	pub async fn install(
		&self,
		namespace: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let path = format!("/v0/runtime/{}/install", urlencoded(namespace));
		self.transport
			.post_json(&path, serde_json::json!({ "variables": variables }))
			.await
	}

	pub async fn uninstall(
		&self,
		namespace: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let path = format!("/v0/runtime/{}/uninstall", urlencoded(namespace));
		self.transport
			.post_json(&path, serde_json::json!({ "variables": variables }))
			.await
	}

	pub async fn execute(
		&self,
		namespace: &str,
		method: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let path = format!("/v0/runtime/{}/{}", urlencoded(namespace), method);
		self.transport
			.post_json(&path, serde_json::json!({ "variables": variables }))
			.await
	}

	pub async fn stop(&self, namespace: &str) -> Result<(), HttpError> {
		let path = format!("/v0/runtime/{}/stop", urlencoded(namespace));
		self.transport
			.post_json(&path, serde_json::Value::Null)
			.await
	}

	pub async fn register_arrow(&self, namespace: &str) -> Result<(), HttpError> {
		let path = format!("/v0/arrow/{}", urlencoded(namespace));
		self.transport
			.post_json(&path, serde_json::Value::Null)
			.await
	}

	pub async fn remove_arrow(&self, namespace: &str) -> Result<(), HttpError> {
		let path = format!("/v0/arrow/{}", urlencoded(namespace));
		self.transport.delete(&path).await
	}

	pub async fn follow_collection(&self, namespace: &str) -> Result<(), HttpError> {
		let path = format!("/v0/collection/{}/follow", urlencoded(namespace));
		self.transport
			.post_json(&path, serde_json::Value::Null)
			.await
	}

	pub async fn unfollow_collection(&self, namespace: &str) -> Result<(), HttpError> {
		let path = format!("/v0/collection/{}/follow", urlencoded(namespace));
		self.transport.delete(&path).await
	}
}

// ── Wire types (private) ──────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ApiEnvelope<T> {
	#[allow(dead_code)]
	success: bool,
	error: Option<String>,
	data: Option<T>,
}

fn urlencoded(namespace: &str) -> String {
	namespace.replace('/', "%2F")
}

pub(crate) fn parse_api_error(bytes: &[u8]) -> String {
	#[derive(serde::Deserialize)]
	struct Envelope {
		error: Option<String>,
	}
	serde_json::from_slice::<Envelope>(bytes)
		.ok()
		.and_then(|e| e.error)
		.unwrap_or_else(|| "unknown error".into())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Mutex;

	struct MockTransport {
		responses: Mutex<Vec<Bytes>>,
	}

	impl MockTransport {
		fn new(responses: Vec<&str>) -> Arc<Self> {
			Arc::new(Self {
				responses: Mutex::new(
					responses
						.iter()
						.map(|s| Bytes::from(s.to_string()))
						.collect(),
				),
			})
		}
	}

	#[async_trait]
	impl HttpTransport for MockTransport {
		async fn get_bytes(&self, _path: &str) -> Result<Bytes, HttpError> {
			self.responses
				.lock()
				.unwrap()
				.drain(..1)
				.next()
				.ok_or_else(|| HttpError::Request("no response".into()))
		}
		async fn post_json(
			&self,
			_path: &str,
			_body: serde_json::Value,
		) -> Result<(), HttpError> {
			Ok(())
		}
		async fn delete(&self, _path: &str) -> Result<(), HttpError> {
			Ok(())
		}
	}

	#[test]
	fn urlencoded_replaces_slashes() {
		assert_eq!(
			urlencoded("github.com/user/repo"),
			"github.com%2Fuser%2Frepo"
		);
	}

	#[tokio::test]
	async fn get_arrow_detail_returns_raw_data() {
		let json = r#"{"success":true,"data":{"namespace":"github.com/user/repo@v1.0.0","name":"My Arrow","state":"ready","active_run":null,"last_return":null}}"#;
		let client = HttpClient::new(MockTransport::new(vec![json]));
		let data = client
			.get_arrow_detail("github.com/user/repo@v1.0.0")
			.await
			.unwrap();
		assert_eq!(data["namespace"], "github.com/user/repo@v1.0.0");
		assert_eq!(data["state"], "ready");
	}

	#[tokio::test]
	async fn fetch_arrows_returns_raw_data() {
		let json = r#"{"success":true,"data":[{"namespace":"github.com/user/repo","name":"My Arrow","versions":[{"ref":"v1.0.0","version":"1.0.0","state":"ready"}]}]}"#;
		let client = HttpClient::new(MockTransport::new(vec![json]));
		let data = client.fetch_arrows().await.unwrap();
		let items = data.as_array().unwrap();
		assert_eq!(items.len(), 1);
		assert_eq!(items[0]["namespace"], "github.com/user/repo");
		assert!(items[0]["versions"].is_array());
	}

	/// An envelope can be well-formed JSON and still carry no data — that is
	/// how quiver.core reports a handled failure. Without this arm the caller
	/// would get a deserialisation error instead of the daemon's own message,
	/// which is the difference between "the arrow is unknown" and "the desktop
	/// app is broken".
	#[tokio::test]
	async fn envelope_without_data_surfaces_the_daemons_error() {
		let json = r#"{"success":false,"error":"arrow not found","data":null}"#;
		let client = HttpClient::new(MockTransport::new(vec![json]));
		let err = client.fetch_arrows().await.unwrap_err();
		match err {
			HttpError::Api { code, message } => {
				assert_eq!(code, 500);
				assert_eq!(message, "arrow not found");
			}
			other => panic!("expected Api, got {other:?}"),
		}
	}

	#[tokio::test]
	async fn health_returns_ok_on_any_response() {
		let client = HttpClient::new(MockTransport::new(vec!["ok"]));
		assert!(client.health().await.is_ok());
	}

	#[tokio::test]
	async fn uninstall_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client
			.uninstall("github.com/user/repo@v1.0.0", Default::default())
			.await
			.is_ok());
	}

	#[tokio::test]
	async fn execute_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client
			.execute("github.com/user/repo@v1.0.0", "run", Default::default())
			.await
			.is_ok());
	}

	#[tokio::test]
	async fn stop_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client.stop("github.com/user/repo@v1.0.0").await.is_ok());
	}

	#[tokio::test]
	async fn remove_arrow_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client
			.remove_arrow("github.com/user/repo@v1.0.0")
			.await
			.is_ok());
	}

	#[tokio::test]
	async fn follow_collection_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client
			.follow_collection("github.com/user/collection@v1.0.0")
			.await
			.is_ok());
	}

	#[tokio::test]
	async fn unfollow_collection_returns_ok() {
		let client = HttpClient::new(MockTransport::new(vec![]));
		assert!(client
			.unfollow_collection("github.com/user/collection@v1.0.0")
			.await
			.is_ok());
	}
}

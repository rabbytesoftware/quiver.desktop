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
	#[error("api error: {0}")]
	Api(String),
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
		envelope.data
			.ok_or_else(|| HttpError::Api(envelope.error.unwrap_or_default()))
	}

	pub async fn health(&self) -> Result<(), HttpError> {
		self.transport.get_bytes("/health").await.map(|_| ())
	}

	pub async fn fetch_arrows(&self) -> Result<Vec<serde_json::Value>, HttpError> {
		let items: Vec<ArrowListResponseItem> =
			self.get_json("/v0/arrow?user_installed=true").await?;
		Ok(build_arrow_values(items))
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

#[derive(serde::Deserialize)]
struct InstalledVersion {
	#[serde(rename = "ref")]
	installed_ref: String,
	version: String,
	state: String,
}

#[derive(serde::Deserialize)]
struct ArrowListResponseItem {
	namespace: String,
	name: String,
	versions: Vec<InstalledVersion>,
}

fn urlencoded(namespace: &str) -> String {
	namespace.replace('/', "%2F")
}

fn build_arrow_values(items: Vec<ArrowListResponseItem>) -> Vec<serde_json::Value> {
	items.into_iter()
		.flat_map(|arrow| {
			let ns = arrow.namespace.clone();
			let name = arrow.name.clone();
			arrow.versions.into_iter().map(move |v| {
				serde_json::json!({
					"namespace": format!("{}@{}", ns, v.installed_ref),
					"name": name,
					"version": v.version,
					"state": v.state,
					"active_run": null,
					"last_outcome": null,
				})
			})
		})
		.collect()
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
	async fn fetch_arrows_builds_versioned_keys() {
		let json = r#"{"success":true,"data":[{"namespace":"github.com/user/repo","name":"My Arrow","versions":[{"ref":"v1.0.0","version":"1.0.0","state":"ready"}]}]}"#;
		let client = HttpClient::new(MockTransport::new(vec![json]));
		let items = client.fetch_arrows().await.unwrap();
		assert_eq!(items.len(), 1);
		assert_eq!(items[0]["namespace"], "github.com/user/repo@v1.0.0");
	}

	#[tokio::test]
	async fn health_returns_ok_on_any_response() {
		let client = HttpClient::new(MockTransport::new(vec!["ok"]));
		assert!(client.health().await.is_ok());
	}
}

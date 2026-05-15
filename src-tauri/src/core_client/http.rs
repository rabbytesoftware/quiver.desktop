use crate::core_client::types::{ApiEnvelope, ArrowListItem, ArrowListResponseItem};
use reqwest::Client;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
	#[error("request failed: {0}")]
	Request(#[from] reqwest::Error),
	#[error("api error: {0}")]
	Api(String),
}

pub struct HttpClient {
	client: Client,
	base_url: String,
}

impl HttpClient {
	pub fn new(base_url: impl Into<String>) -> Self {
		Self {
			client: Client::new(),
			base_url: base_url.into(),
		}
	}

	pub async fn health(&self) -> Result<(), HttpError> {
		let url = format!("{}/health", self.base_url);
		self.client.get(&url).send().await?.error_for_status()?;
		Ok(())
	}

	pub async fn fetch_arrows(&self) -> Result<Vec<ArrowListResponseItem>, HttpError> {
		let url = format!("{}/v0/arrow?user_installed=true", self.base_url);
		let resp: ApiEnvelope<Vec<ArrowListResponseItem>> =
			self.client.get(&url).send().await?.json().await?;
		resp.data
			.ok_or_else(|| HttpError::Api(resp.error.unwrap_or_default()))
	}

	pub async fn install(
		&self,
		namespace: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/runtime/{}/install",
			self.base_url,
			urlencoded(namespace)
		);
		let body = serde_json::json!({ "variables": variables });
		self.client
			.post(&url)
			.json(&body)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	pub async fn uninstall(
		&self,
		namespace: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/runtime/{}/uninstall",
			self.base_url,
			urlencoded(namespace)
		);
		let body = serde_json::json!({ "variables": variables });
		self.client
			.post(&url)
			.json(&body)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	pub async fn execute(
		&self,
		namespace: &str,
		method: &str,
		variables: HashMap<String, String>,
	) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/runtime/{}/{}",
			self.base_url,
			urlencoded(namespace),
			method
		);
		let body = serde_json::json!({ "variables": variables });
		self.client
			.post(&url)
			.json(&body)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	pub async fn stop(&self, namespace: &str) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/runtime/{}/stop",
			self.base_url,
			urlencoded(namespace)
		);
		self.client.post(&url).send().await?.error_for_status()?;
		Ok(())
	}

	pub async fn register_arrow(&self, namespace: &str) -> Result<(), HttpError> {
		let url = format!("{}/v0/arrow/{}", self.base_url, urlencoded(namespace));
		self.client.post(&url).send().await?.error_for_status()?;
		Ok(())
	}

	pub async fn remove_arrow(&self, namespace: &str) -> Result<(), HttpError> {
		let url = format!("{}/v0/arrow/{}", self.base_url, urlencoded(namespace));
		self.client.delete(&url).send().await?.error_for_status()?;
		Ok(())
	}

	pub async fn follow_collection(&self, namespace: &str) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/collection/{}/follow",
			self.base_url,
			urlencoded(namespace)
		);
		self.client.post(&url).send().await?.error_for_status()?;
		Ok(())
	}

	pub async fn unfollow_collection(&self, namespace: &str) -> Result<(), HttpError> {
		let url = format!(
			"{}/v0/collection/{}/follow",
			self.base_url,
			urlencoded(namespace)
		);
		self.client.delete(&url).send().await?.error_for_status()?;
		Ok(())
	}
}

pub fn urlencoded(namespace: &str) -> String {
	namespace.replace('/', "%2F")
}

pub fn to_arrow_list_items(items: Vec<ArrowListResponseItem>) -> Vec<ArrowListItem> {
	items.into_iter()
		.flat_map(|arrow| {
			arrow.versions.into_iter().map(move |v| {
				let namespace = format!("{}@{}", arrow.namespace, v.installed_ref);
				ArrowListItem {
					namespace,
					name: arrow.name.clone(),
					version: v.version,
					state: v.state,
					active_run: None,
					last_outcome: None,
				}
			})
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core_client::types::{ArrowState, InstalledVersion};

	#[test]
	fn urlencoded_replaces_slashes() {
		assert_eq!(
			urlencoded("github.com/user/repo"),
			"github.com%2Fuser%2Frepo"
		);
	}

	#[test]
	fn to_arrow_list_items_builds_versioned_keys() {
		let items = vec![ArrowListResponseItem {
			namespace: "github.com/user/repo".into(),
			name: "My Arrow".into(),
			versions: vec![InstalledVersion {
				installed_ref: "v1.0.0".into(),
				version: "1.0.0".into(),
				state: ArrowState::Ready,
			}],
		}];
		let result = to_arrow_list_items(items);
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].namespace, "github.com/user/repo@v1.0.0");
		assert_eq!(result[0].name, "My Arrow");
		assert_eq!(result[0].version, "1.0.0");
	}

	#[test]
	fn to_arrow_list_items_expands_multiple_versions() {
		let items = vec![ArrowListResponseItem {
			namespace: "github.com/user/repo".into(),
			name: "Arrow".into(),
			versions: vec![
				InstalledVersion {
					installed_ref: "v1.0.0".into(),
					version: "1.0.0".into(),
					state: ArrowState::Ready,
				},
				InstalledVersion {
					installed_ref: "v2.0.0".into(),
					version: "2.0.0".into(),
					state: ArrowState::Ready,
				},
			],
		}];
		let result = to_arrow_list_items(items);
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].namespace, "github.com/user/repo@v1.0.0");
		assert_eq!(result[1].namespace, "github.com/user/repo@v2.0.0");
	}
}

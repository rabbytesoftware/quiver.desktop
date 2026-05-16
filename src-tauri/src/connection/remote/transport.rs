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
}

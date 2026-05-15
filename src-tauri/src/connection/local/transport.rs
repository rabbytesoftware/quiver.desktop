use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixConnector, Uri as UnixUri};
use tokio_tungstenite::tungstenite::Error as WsError;

use crate::connection::http::{HttpError, HttpTransport};

pub struct LocalTransport {
	socket_path: String,
}

impl LocalTransport {
	pub fn new(socket_path: impl Into<String>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
	}

	fn uri(&self, path: &str) -> hyper::Uri {
		UnixUri::new(&self.socket_path, path).into()
	}
}

#[async_trait]
impl HttpTransport for LocalTransport {
	async fn get_bytes(&self, path: &str) -> Result<Bytes, HttpError> {
		let client: Client<UnixConnector, Empty<Bytes>> =
			Client::builder(hyper_util::rt::TokioExecutor::new()).build(UnixConnector);
		let req = Request::get(self.uri(path))
			.body(Empty::new())
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let resp = client
			.request(req)
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let body = resp
			.into_body()
			.collect()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		Ok(body.to_bytes())
	}

	async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), HttpError> {
		let json = serde_json::to_vec(&body).map_err(HttpError::Parse)?;
		let client: Client<UnixConnector, Full<Bytes>> =
			Client::builder(hyper_util::rt::TokioExecutor::new()).build(UnixConnector);
		let req = Request::post(self.uri(path))
			.header("content-type", "application/json")
			.body(Full::new(Bytes::from(json)))
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let resp = client
			.request(req)
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		if !resp.status().is_success() {
			return Err(HttpError::Request(format!("HTTP {}", resp.status())));
		}
		Ok(())
	}

	async fn delete(&self, path: &str) -> Result<(), HttpError> {
		let client: Client<UnixConnector, Empty<Bytes>> =
			Client::builder(hyper_util::rt::TokioExecutor::new()).build(UnixConnector);
		let req = Request::delete(self.uri(path))
			.body(Empty::new())
			.map_err(|e| HttpError::Request(e.to_string()))?;
		let resp = client
			.request(req)
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?;
		if !resp.status().is_success() {
			return Err(HttpError::Request(format!("HTTP {}", resp.status())));
		}
		Ok(())
	}
}

// ── WS over Unix socket ───────────────────────────────────────────────────────

pub async fn connect_unix_ws(
	socket_path: &str,
	path: &str,
) -> Result<tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>, WsError> {
	let stream = tokio::net::UnixStream::connect(socket_path)
		.await
		.map_err(WsError::Io)?;
	let url = format!("ws://localhost{}", path);
	let (ws, _) = tokio_tungstenite::client_async(url, stream).await?;
	Ok(ws)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn local_transport_constructs() {
		let _ = LocalTransport::new("/tmp/quiver.sock");
	}
}

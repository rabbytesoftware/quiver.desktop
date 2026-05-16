use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixConnector, Uri as UnixUri};
use tokio_tungstenite::tungstenite::Error as WsError;

use crate::connection::http::{parse_api_error, HttpError, HttpTransport};

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
		let code = resp.status().as_u16();
		let body = resp
			.into_body()
			.collect()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?
			.to_bytes();
		if code >= 400 {
			return Err(HttpError::Api {
				code,
				message: parse_api_error(&body),
			});
		}
		Ok(body)
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
		let code = resp.status().as_u16();
		if resp.status().is_success() {
			return Ok(());
		}
		let body_bytes = resp
			.into_body()
			.collect()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?
			.to_bytes();
		Err(HttpError::Api {
			code,
			message: parse_api_error(&body_bytes),
		})
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
		let code = resp.status().as_u16();
		if resp.status().is_success() {
			return Ok(());
		}
		let body_bytes = resp
			.into_body()
			.collect()
			.await
			.map_err(|e| HttpError::Request(e.to_string()))?
			.to_bytes();
		Err(HttpError::Api {
			code,
			message: parse_api_error(&body_bytes),
		})
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

use std::sync::Arc;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::http::HttpClient;
use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::{Emitter, WsTarget};

#[derive(Deserialize)]
struct ArrowWsMessage {
	namespace: String,
	removed: Option<bool>,
}

pub async fn run_arrow_ws<E: Emitter>(target: WsTarget, http: Arc<HttpClient>, emitter: Arc<E>) {
	loop {
		match &target {
			WsTarget::Tcp(base_url) => {
				let url = format!("{}/v0/arrow?user_installed=true", base_url);
				if let Ok((mut ws, _)) =
					tokio_tungstenite::connect_async(&url).await
				{
					run_arrow_loop(&mut ws, &http, emitter.as_ref()).await;
				}
			}
			WsTarget::Unix(socket_path) => {
				if let Ok(mut ws) = connect_unix_ws(
					socket_path,
					"/v0/arrow?user_installed=true",
				)
				.await
				{
					run_arrow_loop(&mut ws, &http, emitter.as_ref()).await;
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}

async fn run_arrow_loop<S, E>(
	ws: &mut tokio_tungstenite::WebSocketStream<S>,
	http: &HttpClient,
	emitter: &E,
) where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
	E: Emitter,
{
	while let Some(msg) = ws.next().await {
		match msg {
			Ok(Message::Text(text)) => {
				if let Ok(parsed) =
					serde_json::from_str::<ArrowWsMessage>(&text.to_string())
				{
					handle_arrow_message(parsed, http, emitter).await;
				}
			}
			Ok(Message::Close(_)) | Err(_) => break,
			_ => {}
		}
	}
}

async fn handle_arrow_message<E: Emitter>(msg: ArrowWsMessage, http: &HttpClient, emitter: &E) {
	if msg.removed.unwrap_or(false) {
		emitter.emit_arrow_remove(msg.namespace);
		return;
	}
	resync_catalog(http, emitter).await;
}

async fn resync_catalog<E: Emitter>(http: &HttpClient, emitter: &E) {
	if let Ok(items) = http.fetch_arrows().await {
		for chunk in items.chunks(100) {
			emitter.emit_arrow_hydrate(chunk.to_vec());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::http::{HttpError, HttpTransport};
	use crate::connection::types::{
		ActiveRun, ArrowListItem, CoreStatus, LastOutcome, RuntimeUpdatePayload,
	};
	use async_trait::async_trait;
	use bytes::Bytes;
	use std::sync::Mutex;

	struct MockEmitter {
		hydrated: Mutex<Vec<Vec<ArrowListItem>>>,
		removed: Mutex<Vec<String>>,
	}

	impl MockEmitter {
		fn new() -> Arc<Self> {
			Arc::new(Self {
				hydrated: Mutex::new(vec![]),
				removed: Mutex::new(vec![]),
			})
		}
	}

	impl Emitter for MockEmitter {
		fn emit_core_status(&self, _: CoreStatus) {}
		fn emit_arrow_hydrate(&self, items: Vec<ArrowListItem>) {
			self.hydrated.lock().unwrap().push(items);
		}
		fn emit_arrow_remove(&self, ns: String) {
			self.removed.lock().unwrap().push(ns);
		}
		fn emit_runtime_update(&self, _: RuntimeUpdatePayload) {}
	}

	struct NullTransport;

	#[async_trait]
	impl HttpTransport for NullTransport {
		async fn get_bytes(&self, _: &str) -> Result<Bytes, HttpError> {
			Err(HttpError::Request("no server".into()))
		}
		async fn post_json(&self, _: &str, _: serde_json::Value) -> Result<(), HttpError> {
			Ok(())
		}
		async fn delete(&self, _: &str) -> Result<(), HttpError> {
			Ok(())
		}
	}

	#[tokio::test]
	async fn removed_flag_emits_remove_event() {
		let msg = ArrowWsMessage {
			namespace: "ns@v1".into(),
			removed: Some(true),
		};
		let emitter = MockEmitter::new();
		let http = Arc::new(HttpClient::new(Arc::new(NullTransport)));
		handle_arrow_message(msg, &http, emitter.as_ref()).await;
		assert_eq!(emitter.removed.lock().unwrap().len(), 1);
		assert!(emitter.hydrated.lock().unwrap().is_empty());
	}

	#[tokio::test]
	async fn non_removed_triggers_resync_attempt() {
		let msg = ArrowWsMessage {
			namespace: "ns@v1".into(),
			removed: None,
		};
		let emitter = MockEmitter::new();
		let http = Arc::new(HttpClient::new(Arc::new(NullTransport)));
		handle_arrow_message(msg, &http, emitter.as_ref()).await;
		assert!(emitter.removed.lock().unwrap().is_empty());
	}
}

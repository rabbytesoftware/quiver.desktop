use crate::core_client::http::HttpClient;
use crate::core_client::types::{ArrowWsMessage, Emitter};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn run_arrow_ws<E: Emitter>(ws_base_url: String, http: Arc<HttpClient>, emitter: Arc<E>) {
	let url = format!("{}/v0/arrow?user_installed=true", ws_base_url);
	loop {
		if let Ok((mut ws, _)) = connect_async(&url).await {
			while let Some(msg) = ws.next().await {
				match msg {
					Ok(Message::Text(text)) => {
						if let Ok(parsed) = serde_json::from_str::<
							ArrowWsMessage,
						>(&text.to_string())
						{
							handle_arrow_message(
								parsed,
								&http,
								emitter.as_ref(),
							)
							.await;
						}
					}
					Ok(Message::Close(_)) | Err(_) => break,
					_ => {}
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}

pub async fn handle_arrow_message<E: Emitter>(msg: ArrowWsMessage, http: &HttpClient, emitter: &E) {
	if msg.removed.unwrap_or(false) {
		emitter.emit_arrow_remove(msg.namespace);
		return;
	}

	resync_catalog(http, emitter).await;
}

pub async fn resync_catalog<E: Emitter>(http: &HttpClient, emitter: &E) {
	if let Ok(items) = http.fetch_arrows().await {
		let list_items = crate::core_client::http::to_arrow_list_items(items);
		for chunk in list_items.chunks(100) {
			emitter.emit_arrow_hydrate(chunk.to_vec());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core_client::types::{
		ArrowListItem, ArrowState, CoreStatus, RuntimeUpdatePayload,
	};
	use std::sync::Mutex;

	struct MockEmitter {
		hydrated: Mutex<Vec<Vec<ArrowListItem>>>,
		removed: Mutex<Vec<String>>,
	}

	impl MockEmitter {
		fn new() -> Self {
			Self {
				hydrated: Mutex::new(vec![]),
				removed: Mutex::new(vec![]),
			}
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

	#[test]
	fn removed_flag_emits_remove_event() {
		let msg = ArrowWsMessage {
			namespace: "github.com/user/repo@v1.0.0".into(),
			name: "X".into(),
			version: "1.0.0".into(),
			removed: Some(true),
		};
		let emitter = Arc::new(MockEmitter::new());
		let http = Arc::new(HttpClient::new("http://localhost:9999"));

		tokio::runtime::Runtime::new().unwrap().block_on(async {
			handle_arrow_message(msg, &http, emitter.as_ref()).await;
		});

		let removed = emitter.removed.lock().unwrap();
		assert_eq!(removed.len(), 1);
		assert_eq!(removed[0], "github.com/user/repo@v1.0.0");
		assert!(emitter.hydrated.lock().unwrap().is_empty());
	}

	#[test]
	fn non_removed_message_triggers_resync() {
		let msg = ArrowWsMessage {
			namespace: "github.com/user/repo@v1.0.0".into(),
			name: "X".into(),
			version: "1.0.0".into(),
			removed: None,
		};
		let emitter = Arc::new(MockEmitter::new());
		let http = Arc::new(HttpClient::new("http://localhost:9999"));

		tokio::runtime::Runtime::new().unwrap().block_on(async {
			handle_arrow_message(msg, &http, emitter.as_ref()).await;
		});

		assert!(emitter.removed.lock().unwrap().is_empty());
	}
}

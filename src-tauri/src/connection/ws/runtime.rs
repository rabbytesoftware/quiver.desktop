use std::sync::Arc;

use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::connection::types::{Emitter, WsTarget};
use crate::connection::ws::connector;

pub async fn run_runtime_ws<E: Emitter>(target: WsTarget, emitter: Arc<E>) {
	loop {
		log::debug!("[ws/runtime] connecting");
		if let Some(mut ws) = connector::open(&target, "/v0/runtime").await {
			log::info!("[ws/runtime] connected");
			run_ws_loop(&mut ws, emitter.as_ref()).await;
			log::info!("[ws/runtime] disconnected — reconnecting in 500ms");
		} else {
			log::debug!("[ws/runtime] connection failed — retrying in 500ms");
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}

async fn run_ws_loop<S, E>(ws: &mut S, emitter: &E)
where
	S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
	E: Emitter,
{
	while let Some(msg) = ws.next().await {
		match msg {
			Ok(Message::Text(text)) => {
				let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
				else {
					continue;
				};
				emitter.emit_runtime_update(value);
			}
			Ok(Message::Close(_)) | Err(_) => break,
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::types::CoreStatus;
	use futures_util::stream;
	use std::sync::Mutex;
	use tokio_tungstenite::tungstenite::Error;

	struct MockEmitter {
		updates: Mutex<Vec<serde_json::Value>>,
	}

	impl MockEmitter {
		fn new() -> Arc<Self> {
			Arc::new(Self {
				updates: Mutex::new(vec![]),
			})
		}
	}

	impl Emitter for MockEmitter {
		fn emit_core_status(&self, _: CoreStatus) {}
		fn emit_arrow_event(&self, _: serde_json::Value) {}
		fn emit_runtime_update(&self, value: serde_json::Value) {
			self.updates.lock().unwrap().push(value);
		}
		fn emit_connection_changed(&self, _: serde_json::Value) {}
	}

	#[tokio::test]
	async fn runtime_update_is_forwarded_to_emitter() {
		let emitter = MockEmitter::new();
		let json = r#"{"namespace":"ns@v1","state":"running","active_run":null,"last_return":null}"#;
		let messages: Vec<Result<Message, Error>> = vec![Ok(Message::Text(json.into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		let updates = emitter.updates.lock().unwrap();
		assert_eq!(updates.len(), 1);
		assert_eq!(updates[0]["namespace"], "ns@v1");
		assert_eq!(updates[0]["state"], "running");
	}

	#[tokio::test]
	async fn invalid_json_is_silently_dropped() {
		let emitter = MockEmitter::new();
		let messages: Vec<Result<Message, Error>> =
			vec![Ok(Message::Text("not json".into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		assert!(emitter.updates.lock().unwrap().is_empty());
	}

	#[tokio::test]
	async fn close_message_stops_the_loop() {
		let emitter = MockEmitter::new();
		let json = r#"{"namespace":"ns@v1","state":"running"}"#;
		let messages: Vec<Result<Message, Error>> =
			vec![Ok(Message::Close(None)), Ok(Message::Text(json.into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		assert!(emitter.updates.lock().unwrap().is_empty());
	}
}

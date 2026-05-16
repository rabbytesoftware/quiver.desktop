use std::sync::Arc;

use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::{Emitter, WsTarget};

type WsStream = Box<
	dyn Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
		+ Unpin
		+ Send,
>;

pub async fn run_arrow_ws<E: Emitter>(target: WsTarget, emitter: Arc<E>) {
	loop {
		log::debug!("[ws/arrow] connecting");
		if let Some(mut ws) = open(&target, "/v0/arrow").await {
			log::info!("[ws/arrow] connected");
			run_ws_loop(&mut ws, emitter.as_ref()).await;
			log::info!("[ws/arrow] disconnected — reconnecting in 500ms");
		} else {
			log::debug!("[ws/arrow] connection failed — retrying in 500ms");
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}

async fn open(target: &WsTarget, path: &str) -> Option<WsStream> {
	match target {
		WsTarget::Tcp(base) => {
			let url = format!("{}{}", base, path);
			tokio_tungstenite::connect_async(&url)
				.await
				.ok()
				.map(|(ws, _)| Box::new(ws) as WsStream)
		}
		WsTarget::Unix(p) => {
			connect_unix_ws(p, path)
				.await
				.ok()
				.map(|ws| Box::new(ws) as WsStream)
		}
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
				emitter.emit_arrow_event(value);
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
		events: Mutex<Vec<serde_json::Value>>,
	}

	impl MockEmitter {
		fn new() -> Arc<Self> {
			Arc::new(Self {
				events: Mutex::new(vec![]),
			})
		}
	}

	impl Emitter for MockEmitter {
		fn emit_core_status(&self, _: CoreStatus) {}
		fn emit_arrow_event(&self, value: serde_json::Value) {
			self.events.lock().unwrap().push(value);
		}
		fn emit_runtime_update(&self, _: serde_json::Value) {}
		fn emit_connection_changed(&self, _: serde_json::Value) {}
	}

	#[tokio::test]
	async fn upserted_event_is_forwarded_to_emitter() {
		let emitter = MockEmitter::new();
		let json = r#"{"event":"upserted","namespace":"github.com/foo/bar@v1.0.0","name":"bar","version":"v1.0.0"}"#;
		let messages: Vec<Result<Message, Error>> =
			vec![Ok(Message::Text(json.into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		let events = emitter.events.lock().unwrap();
		assert_eq!(events.len(), 1);
		assert_eq!(events[0]["event"], "upserted");
		assert_eq!(events[0]["namespace"], "github.com/foo/bar@v1.0.0");
	}

	#[tokio::test]
	async fn removed_event_is_forwarded_to_emitter() {
		let emitter = MockEmitter::new();
		let json =
			r#"{"event":"removed","namespace":"github.com/foo/bar@v1.0.0"}"#;
		let messages: Vec<Result<Message, Error>> =
			vec![Ok(Message::Text(json.into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		let events = emitter.events.lock().unwrap();
		assert_eq!(events.len(), 1);
		assert_eq!(events[0]["event"], "removed");
	}

	#[tokio::test]
	async fn invalid_json_is_silently_dropped() {
		let emitter = MockEmitter::new();
		let messages: Vec<Result<Message, Error>> =
			vec![Ok(Message::Text("not json".into()))];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		assert!(emitter.events.lock().unwrap().is_empty());
	}

	#[tokio::test]
	async fn close_message_stops_the_loop() {
		let emitter = MockEmitter::new();
		let json = r#"{"event":"upserted"}"#;
		let messages: Vec<Result<Message, Error>> = vec![
			Ok(Message::Close(None)),
			Ok(Message::Text(json.into())),
		];
		run_ws_loop(&mut stream::iter(messages), emitter.as_ref()).await;
		assert!(emitter.events.lock().unwrap().is_empty());
	}
}

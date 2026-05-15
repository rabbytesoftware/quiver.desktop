use std::sync::Arc;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::{
	ActiveRun, ArrowState, Emitter, LastOutcome, RuntimeUpdatePayload, StepProgress, WsTarget,
};

#[derive(Deserialize)]
struct RuntimeWsMessage {
	namespace: String,
	state: ArrowState,
	active_run: Option<ActiveRunWs>,
	last_return: Option<LastReturnWs>,
}

#[derive(Deserialize)]
struct ActiveRunWs {
	method: String,
	pid: Option<u32>,
	#[serde(default)]
	variables: std::collections::HashMap<String, String>,
	#[serde(default)]
	steps: Vec<StepProgress>,
}

#[derive(Deserialize)]
struct LastReturnWs {
	method: String,
	outcome: String,
}

pub async fn run_runtime_ws<E: Emitter>(target: WsTarget, emitter: Arc<E>) {
	loop {
		match &target {
			WsTarget::Tcp(base) => {
				let url = format!("{}/v0/runtime", base);
				if let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&url).await {
					run_ws_loop(&mut ws, emitter.as_ref()).await;
				}
			}
			WsTarget::Unix(path) => {
				if let Ok(mut ws) = connect_unix_ws(path, "/v0/runtime").await {
					run_ws_loop(&mut ws, emitter.as_ref()).await;
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}

async fn run_ws_loop<S, E>(ws: &mut tokio_tungstenite::WebSocketStream<S>, emitter: &E)
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
	E: Emitter,
{
	while let Some(msg) = ws.next().await {
		match msg {
			Ok(Message::Text(text)) => {
				if let Ok(parsed) = serde_json::from_str::<RuntimeWsMessage>(&text.to_string()) {
					emitter.emit_runtime_update(to_runtime_update(parsed));
				}
			}
			Ok(Message::Close(_)) | Err(_) => break,
			_ => {}
		}
	}
}

fn to_runtime_update(msg: RuntimeWsMessage) -> RuntimeUpdatePayload {
	RuntimeUpdatePayload {
		namespace: msg.namespace,
		state: msg.state,
		active_run: msg.active_run.map(|r| ActiveRun {
			method: r.method,
			pid: r.pid,
			variables: r.variables,
			steps: r.steps,
		}),
		last_outcome: msg.last_return.map(|r| LastOutcome {
			method: r.method,
			outcome: r.outcome,
		}),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::types::ArrowState;

	#[test]
	fn to_runtime_update_maps_fields() {
		let msg = RuntimeWsMessage {
			namespace: "ns@v1".into(),
			state: ArrowState::Running,
			active_run: None,
			last_return: Some(LastReturnWs {
				method: "_execute".into(),
				outcome: "success".into(),
			}),
		};
		let p = to_runtime_update(msg);
		assert_eq!(p.namespace, "ns@v1");
		let o = p.last_outcome.unwrap();
		assert_eq!(o.outcome, "success");
	}
}

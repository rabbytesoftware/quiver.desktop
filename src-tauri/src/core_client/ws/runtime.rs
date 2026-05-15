use crate::core_client::types::{
	ActiveRun, Emitter, LastOutcome, RuntimeUpdatePayload, RuntimeWsMessage,
};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn run_runtime_ws<E: Emitter>(ws_base_url: String, emitter: Arc<E>) {
	let url = format!("{}/v0/runtime", ws_base_url);
	loop {
		if let Ok((mut ws, _)) = connect_async(&url).await {
			while let Some(msg) = ws.next().await {
				match msg {
					Ok(Message::Text(text)) => {
						if let Ok(parsed) = serde_json::from_str::<
							RuntimeWsMessage,
						>(&text.to_string())
						{
							let payload = to_runtime_update(parsed);
							emitter.emit_runtime_update(payload);
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

pub fn to_runtime_update(msg: RuntimeWsMessage) -> RuntimeUpdatePayload {
	let active_run = msg.active_run.map(|r| ActiveRun {
		method: r.method,
		pid: r.pid,
		variables: r.variables,
		steps: r.steps,
	});
	let last_outcome = msg.last_return.map(|r| LastOutcome {
		method: r.method,
		outcome: r.outcome,
	});
	RuntimeUpdatePayload {
		namespace: msg.namespace,
		state: msg.state,
		active_run,
		last_outcome,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core_client::types::{
		ActiveRunWs, ArrowState, LastReturnWs, StepProgress, StepStatus,
	};

	#[test]
	fn to_runtime_update_maps_active_run() {
		let msg = RuntimeWsMessage {
			namespace: "github.com/user/repo@v1.0.0".into(),
			state: ArrowState::Installing,
			active_run: Some(ActiveRunWs {
				method: "_install".into(),
				pid: None,
				variables: Default::default(),
				steps: vec![StepProgress {
					index: 0,
					title: "Fetch".into(),
					status: StepStatus::Running,
					step_type: "fetch".into(),
					error: None,
				}],
			}),
			last_return: None,
		};
		let payload = to_runtime_update(msg);
		assert_eq!(payload.namespace, "github.com/user/repo@v1.0.0");
		assert_eq!(payload.state, ArrowState::Installing);
		let run = payload.active_run.unwrap();
		assert_eq!(run.method, "_install");
		assert_eq!(run.steps.len(), 1);
		assert!(payload.last_outcome.is_none());
	}

	#[test]
	fn to_runtime_update_extracts_last_outcome_only() {
		let msg = RuntimeWsMessage {
			namespace: "ns@v1".into(),
			state: ArrowState::Ready,
			active_run: None,
			last_return: Some(LastReturnWs {
				method: "_install".into(),
				outcome: "success".into(),
			}),
		};
		let payload = to_runtime_update(msg);
		let outcome = payload.last_outcome.unwrap();
		assert_eq!(outcome.method, "_install");
		assert_eq!(outcome.outcome, "success");
	}
}

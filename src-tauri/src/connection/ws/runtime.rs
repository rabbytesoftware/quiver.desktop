use std::sync::Arc;

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::{Emitter, WsTarget};

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
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    emitter.emit_runtime_update(value);
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_json_is_forwarded_as_value() {
        let json = r#"{"namespace":"ns@v1","state":"running","active_run":null,"last_return":null}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["namespace"], "ns@v1");
        assert_eq!(value["state"], "running");
    }
}

use std::sync::Arc;

use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::{Emitter, WsTarget};

pub async fn run_arrow_ws<E: Emitter>(target: WsTarget, emitter: Arc<E>) {
    loop {
        match &target {
            WsTarget::Tcp(base) => {
                let url = format!("{}/v0/arrow", base);
                if let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&url).await {
                    run_ws_loop(&mut ws, emitter.as_ref()).await;
                }
            }
            WsTarget::Unix(path) => {
                if let Ok(mut ws) = connect_unix_ws(path, "/v0/arrow").await {
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
                    emitter.emit_arrow_event(value);
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
    use crate::connection::types::CoreStatus;
    use std::sync::Mutex;

    struct MockEmitter {
        events: Mutex<Vec<serde_json::Value>>,
    }

    impl MockEmitter {
        fn new() -> Arc<Self> {
            Arc::new(Self { events: Mutex::new(vec![]) })
        }
    }

    impl Emitter for MockEmitter {
        fn emit_core_status(&self, _: CoreStatus) {}
        fn emit_arrow_hydrate(&self, _: Vec<serde_json::Value>) {}
        fn emit_arrow_event(&self, value: serde_json::Value) {
            self.events.lock().unwrap().push(value);
        }
        fn emit_runtime_update(&self, _: serde_json::Value) {}
    }

    #[test]
    fn upserted_event_is_forwarded_as_value() {
        let json = r#"{"event":"upserted","namespace":"github.com/foo/bar@v1.0.0","name":"bar","version":"v1.0.0"}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["event"], "upserted");
        assert_eq!(value["namespace"], "github.com/foo/bar@v1.0.0");
    }

    #[test]
    fn removed_event_is_forwarded_as_value() {
        let json = r#"{"event":"removed","namespace":"github.com/foo/bar@v1.0.0"}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["event"], "removed");
        assert_eq!(value["namespace"], "github.com/foo/bar@v1.0.0");
    }
}

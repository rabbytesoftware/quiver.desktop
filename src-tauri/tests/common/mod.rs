#![allow(dead_code)]

use futures_util::SinkExt;
use std::net::TcpListener;
use tokio::net::TcpListener as AsyncTcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub fn free_port() -> u16 {
	TcpListener::bind("127.0.0.1:0")
		.unwrap()
		.local_addr()
		.unwrap()
		.port()
}

pub fn base_url(port: u16) -> String {
	format!("http://127.0.0.1:{}", port)
}

pub fn ws_base_url(port: u16) -> String {
	format!("ws://127.0.0.1:{}", port)
}

/// Spawns a minimal WS server that sends `messages` to the first client then closes.
pub async fn spawn_ws_server(port: u16, messages: Vec<String>) {
	let listener = AsyncTcpListener::bind(format!("127.0.0.1:{}", port))
		.await
		.unwrap();
	tokio::spawn(async move {
		if let Ok((stream, _)) = listener.accept().await {
			if let Ok(mut ws) = accept_async(stream).await {
				for msg in messages {
					ws.send(Message::Text(msg.into())).await.ok();
				}
				ws.close(None).await.ok();
			}
		}
	});
}

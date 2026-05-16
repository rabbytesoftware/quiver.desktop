use futures_util::Stream;
use tokio_tungstenite::tungstenite::Message;

use crate::connection::local::transport::connect_unix_ws;
use crate::connection::types::WsTarget;

pub type WsStream = Box<
	dyn Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin + Send,
>;

pub async fn open(target: &WsTarget, path: &str) -> Option<WsStream> {
	match target {
		WsTarget::Tcp(base) => {
			let url = format!("{}{}", base, path);
			tokio_tungstenite::connect_async(&url)
				.await
				.ok()
				.map(|(ws, _)| Box::new(ws) as WsStream)
		}
		WsTarget::Unix(p) => connect_unix_ws(p, path)
			.await
			.ok()
			.map(|ws| Box::new(ws) as WsStream),
	}
}

pub mod arrow;
pub mod runtime;

use crate::core_client::http::HttpClient;
use crate::core_client::types::Emitter;
use std::sync::Arc;

pub struct WsManager<E: Emitter> {
	ws_base_url: String,
	http: Arc<HttpClient>,
	emitter: Arc<E>,
}

impl<E: Emitter + 'static> WsManager<E> {
	pub fn new(ws_base_url: String, http: Arc<HttpClient>, emitter: Arc<E>) -> Self {
		Self {
			ws_base_url,
			http,
			emitter,
		}
	}

	pub fn start(self) {
		let arrow_url = self.ws_base_url.clone();
		let runtime_url = self.ws_base_url.clone();
		let http = Arc::clone(&self.http);
		let emitter_a = Arc::clone(&self.emitter);
		let emitter_r = Arc::clone(&self.emitter);

		tokio::spawn(arrow::run_arrow_ws(arrow_url, http, emitter_a));
		tokio::spawn(runtime::run_runtime_ws(runtime_url, emitter_r));
	}
}

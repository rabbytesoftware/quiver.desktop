pub mod arrow;
pub mod runtime;

use std::sync::Arc;

use crate::connection::http::HttpClient;
use crate::connection::types::{Emitter, WsTarget};

pub struct WsManager<E: Emitter> {
	target: WsTarget,
	http: Arc<HttpClient>,
	emitter: Arc<E>,
}

impl<E: Emitter + 'static> WsManager<E> {
	pub fn new(target: WsTarget, http: Arc<HttpClient>, emitter: Arc<E>) -> Self {
		Self {
			target,
			http,
			emitter,
		}
	}

	pub fn start(self) {
		let arrow_target = self.target.clone();
		let runtime_target = self.target.clone();
		let http = Arc::clone(&self.http);
		let emitter_a = Arc::clone(&self.emitter);
		let emitter_r = Arc::clone(&self.emitter);
		tokio::spawn(arrow::run_arrow_ws(arrow_target, http, emitter_a));
		tokio::spawn(runtime::run_runtime_ws(runtime_target, emitter_r));
	}
}

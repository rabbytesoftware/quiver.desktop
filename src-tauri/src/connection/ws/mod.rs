pub mod arrow;
pub mod connector;
pub mod runtime;

use std::sync::Arc;

use crate::connection::types::{Emitter, WsTarget};

pub struct WsManager<E: Emitter> {
	target: WsTarget,
	emitter: Arc<E>,
}

impl<E: Emitter + 'static> WsManager<E> {
	pub fn new(target: WsTarget, emitter: Arc<E>) -> Self {
		Self { target, emitter }
	}

	pub fn start(self) {
		let arrow_target = self.target.clone();
		let runtime_target = self.target.clone();
		let emitter_a = Arc::clone(&self.emitter);
		let emitter_r = Arc::clone(&self.emitter);
		tokio::spawn(arrow::run_arrow_ws(arrow_target, emitter_a));
		tokio::spawn(runtime::run_runtime_ws(runtime_target, emitter_r));
	}
}

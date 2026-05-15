use std::sync::Arc;

use crate::connection::http::HttpClient;
use crate::connection::types::{Emitter, WsTarget};

pub struct WsManager<E: Emitter> {
	_target: WsTarget,
	_http: Arc<HttpClient>,
	_emitter: Arc<E>,
}

impl<E: Emitter + 'static> WsManager<E> {
	pub fn new(target: WsTarget, http: Arc<HttpClient>, emitter: Arc<E>) -> Self {
		Self { _target: target, _http: http, _emitter: emitter }
	}

	pub fn start(self) {}
}

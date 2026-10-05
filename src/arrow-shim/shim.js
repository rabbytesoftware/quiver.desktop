/* Quiver arrow shim. Runs inside an arrow's page (an iframe of the shell).
 *
 * The page has no Tauri IPC and no way to reach Quiver. The only thing this
 * script does is replace WebSocket with a version that relays text frames,
 * by postMessage, to the shell, which forwards them to THIS arrow's own
 * server through the daemon. Plain script (no modules): it ships inside the
 * app binary and is injected into the arrow's HTML. */
(function () {
	'use strict';

	var TAG = '__arrowShim';
	var CONNECTING = 0;
	var OPEN = 1;
	var CLOSING = 2;
	var CLOSED = 3;
	var parentWindow = window.parent;
	var sockets = {};
	var nextId = 1;
	// Ids restart in every document of this frame but the shell tracks them
	// per frame, so a per-document nonce keeps a new document's ids from
	// colliding with a stale socket of the previous one.
	var nonce = Math.random().toString(36).slice(2, 8) || '0';

	// The shell origin differs per platform and the payload is only this
	// arrow's own frames, so the target origin is not restricted.
	function post(message) {
		message[TAG] = 1;
		parentWindow.postMessage(message, '*');
	}

	// Lets the shell drop whatever the previous document of this frame left.
	post({ type: 'hello' });

	class ArrowWebSocket extends EventTarget {
		constructor(url) {
			super();
			var parsed = new URL(String(url), window.location.href);
			var scheme = parsed.protocol;
			// A relative url resolves to the page's own (custom) scheme.
			var pageScheme = new URL(window.location.href).protocol;
			if (
				scheme !== 'ws:' &&
				scheme !== 'wss:' &&
				scheme !== 'http:' &&
				scheme !== 'https:' &&
				scheme !== pageScheme
			) {
				throw new DOMException('The URL scheme must be ws or wss.', 'SyntaxError');
			}
			this.url = parsed.href.replace(/^http/, 'ws');
			this.readyState = CONNECTING;
			this.bufferedAmount = 0;
			this.extensions = '';
			this.protocol = '';
			this.binaryType = 'blob';
			this.onopen = null;
			this.onmessage = null;
			this.onerror = null;
			this.onclose = null;
			this._id = nonce + '-' + nextId++;
			sockets[this._id] = this;
			post({ type: 'ws-open', id: this._id, path: parsed.pathname + parsed.search });
		}

		send(data) {
			if (this.readyState === CONNECTING) {
				throw new DOMException('Still in CONNECTING state.', 'InvalidStateError');
			}
			if (this.readyState !== OPEN) return;
			if (typeof data !== 'string') {
				console.error('[arrow] only text WebSocket frames are supported');
				return;
			}
			post({ type: 'ws-send', id: this._id, data: data });
		}

		close() {
			if (this.readyState === CLOSING || this.readyState === CLOSED) return;
			this.readyState = CLOSING;
			post({ type: 'ws-close', id: this._id });
		}

		_fire(name, event) {
			var handler = this['on' + name];
			if (typeof handler === 'function') handler.call(this, event);
			this.dispatchEvent(event);
		}
	}

	ArrowWebSocket.CONNECTING = CONNECTING;
	ArrowWebSocket.OPEN = OPEN;
	ArrowWebSocket.CLOSING = CLOSING;
	ArrowWebSocket.CLOSED = CLOSED;
	ArrowWebSocket.prototype.CONNECTING = CONNECTING;
	ArrowWebSocket.prototype.OPEN = OPEN;
	ArrowWebSocket.prototype.CLOSING = CLOSING;
	ArrowWebSocket.prototype.CLOSED = CLOSED;

	window.addEventListener('message', function (event) {
		if (event.source !== parentWindow) return;
		var m = event.data;
		if (!m || m[TAG] !== 1) return;
		var ws = sockets[m.id];
		if (!ws) return;

		switch (m.type) {
			case 'ws-open':
				ws.readyState = OPEN;
				ws._fire('open', new Event('open'));
				break;
			case 'ws-message':
				if (ws.readyState === OPEN) {
					ws._fire('message', new MessageEvent('message', { data: m.data }));
				}
				break;
			case 'ws-error':
				ws._fire('error', new Event('error'));
				break;
			case 'ws-close':
				ws.readyState = CLOSED;
				delete sockets[m.id];
				ws._fire(
					'close',
					new CloseEvent('close', {
						code: m.code || 1005,
						reason: m.reason || '',
						wasClean: m.code === 1000,
					})
				);
				break;
		}
	});

	window.WebSocket = ArrowWebSocket;
})();

/* Quiver arrow shim. Runs inside an arrow's page (a webview embedded in the app window).
 *
 * The page has no access to Quiver. The only thing this script does is
 * replace WebSocket with a version that relays text frames to the app
 * (window.ipc.postMessage), which forwards them to THIS arrow's own server
 * through the daemon and hands the answers back through
 * window.__arrowShimReceive. Plain script (no modules): it ships inside the
 * app binary and is injected into the arrow's HTML. */
(function () {
	'use strict';

	var TAG = '__arrowShim';
	var CONNECTING = 0;
	var OPEN = 1;
	var CLOSING = 2;
	var CLOSED = 3;
	var sockets = {};
	var nextId = 1;
	// Ids restart in every document of the page but the app tracks them
	// per page, so a per-document nonce keeps a new document's ids from
	// colliding with a stale socket of the previous one.
	var nonce = crypto.getRandomValues(new Uint32Array(1))[0].toString(36);

	function post(message) {
		message[TAG] = 1;
		if (!window.ipc) return;
		window.ipc.postMessage(JSON.stringify(message));
	}

	// Lets the app drop whatever the previous document of this page left.
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

	// The app calls this with each message for the page (a JSON string).
	Object.defineProperty(window, '__arrowShimReceive', { value: function (json) {
		var m;
		try {
			m = typeof json === 'string' ? JSON.parse(json) : json;
		} catch (e) {
			return;
		}
		if (!m || m[TAG] !== 1) return;
		var ws = sockets[m.id];
		if (!ws) return;

		switch (m.type) {
			case 'ws-open':
				if (ws.readyState !== CONNECTING) break;
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
	} });

	window.WebSocket = ArrowWebSocket;
})();

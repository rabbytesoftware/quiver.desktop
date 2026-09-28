//! Everything a local transport does once it has a connected stream.
//!
//! A unix socket and a Windows named pipe differ only in how the stream is
//! opened, so each transport connects and hands the stream here.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
use tauri::http::{self, Request, Response};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AsyncReadWrite, TransportError, WsStream};

/// Forward one request over `stream` and return the daemon's whole response.
pub(super) async fn request_over<S>(
	stream: S,
	req: Request<Vec<u8>>,
) -> Result<Response<Vec<u8>>, TransportError>
where
	S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	// hyper only needs the path-and-query for the request line; the
	// authority in `quiver://localhost/...` is meaningless over a socket.
	let path_and_query = req
		.uri()
		.path_and_query()
		.map(|pq| pq.as_str().to_string())
		.unwrap_or_else(|| req.uri().path().to_string());

	let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	tokio::spawn(async move {
		let _ = conn.await;
	});

	let (parts, body) = req.into_parts();
	let mut builder = HyperRequest::builder()
		.method(parts.method)
		.uri(path_and_query);

	if let Some(headers) = builder.headers_mut() {
		for (name, value) in parts.headers.iter() {
			headers.insert(name, value.clone());
		}
		// HTTP/1.1 requires a Host; a local socket has no meaningful one.
		if !headers.contains_key(http::header::HOST) {
			headers.insert(
				http::header::HOST,
				http::HeaderValue::from_static("localhost"),
			);
		}
	}

	let upstream = builder
		.body(Full::<Bytes>::new(body.into()))
		.map_err(|e| TransportError::Protocol(e.to_string()))?;

	let resp = sender
		.send_request(upstream)
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	let (rp, rb) = resp.into_parts();
	let collected = rb
		.collect()
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?
		.to_bytes()
		.to_vec();

	let mut out = Response::builder().status(rp.status);
	if let Some(headers) = out.headers_mut() {
		for (name, value) in rp.headers.iter() {
			headers.insert(name, value.clone());
		}
	}
	out.body(collected)
		.map_err(|e| TransportError::Protocol(e.to_string()))
}

/// Complete the WebSocket upgrade for `path` (a full `/v0/...` route) over
/// `stream`.
pub(super) async fn open_ws_over<S>(stream: S, path: &str) -> Result<WsStream, TransportError>
where
	S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	let request = format!("ws://localhost{path}")
		.into_client_request()
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	let boxed: Box<dyn AsyncReadWrite> = Box::new(stream);
	// The same door `http.rs` uses, so every transport resolves to one
	// `WsStream` type. The request is always `ws://` here — a local socket
	// carries no TLS — so `uri_mode` picks `Mode::Plain` and this is
	// `client_async` with an extra enum wrapper: no handshake, no
	// certificate work, nothing on the wire that was not there before.
	let (ws, _) = tokio_tungstenite::client_async_tls_with_config(request, boxed, None, None)
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	Ok(ws)
}

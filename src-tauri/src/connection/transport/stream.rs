//! Everything a local transport does once it has a connected stream.
//!
//! A unix socket and a Windows named pipe differ only in how the stream is
//! opened, so each transport connects and hands the stream here.

use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
use tauri::http::{self, Request, Response};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AsyncReadWrite, StreamResponse, TransportError, WsStream, STREAM_BUFFER};

/// The hyper request for `req`: request line from its path-and-query, the
/// headers carried over, a `Host` supplied when absent.
fn upstream_request(req: Request<Vec<u8>>) -> Result<HyperRequest<Full<Bytes>>, TransportError> {
	// hyper only needs the path-and-query for the request line; the
	// authority in `quiver://localhost/...` is meaningless over a socket.
	let path_and_query = req
		.uri()
		.path_and_query()
		.map(|pq| pq.as_str().to_string())
		.unwrap_or_else(|| req.uri().path().to_string());

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

	builder.body(Full::<Bytes>::new(body.into()))
		.map_err(|e| TransportError::Protocol(e.to_string()))
}

/// Forward one request over `stream` and return the daemon's whole response.
pub(super) async fn request_over<S>(
	stream: S,
	req: Request<Vec<u8>>,
) -> Result<Response<Vec<u8>>, TransportError>
where
	S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	let upstream = upstream_request(req)?;

	let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	tokio::spawn(async move {
		let _ = conn.await;
	});

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

/// Forward one request over `stream` and hand back the response as it
/// arrives, not once it is complete.
///
/// The status is known as soon as the head is, so it is returned directly; the
/// body is pumped into the returned channel by a task that stops as soon as the
/// receiver is dropped -- which drops the connection, and with it the
/// request on the daemon's side. `deadline` bounds the whole exchange.
pub(super) async fn request_stream_over<S>(
	stream: S,
	req: Request<Vec<u8>>,
	deadline: Duration,
) -> Result<StreamResponse, TransportError>
where
	S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
	let upstream = upstream_request(req)?;

	let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
		.await
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	tokio::spawn(async move {
		let _ = conn.await;
	});

	let resp = tokio::time::timeout(deadline, sender.send_request(upstream))
		.await
		.map_err(|_| TransportError::Protocol("timed out waiting for the response".into()))?
		.map_err(|e| TransportError::Protocol(e.to_string()))?;
	let status = resp.status().as_u16();
	let mut body = resp.into_body();

	let (tx, rx) = mpsc::channel(STREAM_BUFFER);
	tokio::spawn(async move {
		let pump = async {
			// `tx.closed()` is what makes an abandoned request prompt: a command
			// that is running silently would otherwise leave this task parked on
			// the next frame, holding the connection, until the daemon wrote
			// something -- and the daemon only cancels the command when it sees
			// the connection close.
			loop {
				let frame = tokio::select! {
					_ = tx.closed() => return,
					frame = body.frame() => match frame {
						Some(frame) => frame,
						None => return,
					},
				};
				let item = match frame {
					Ok(frame) => match frame.into_data() {
						Ok(data) => Ok(data.to_vec()),
						// Trailers carry nothing this app reads.
						Err(_) => continue,
					},
					Err(e) => Err(TransportError::Protocol(e.to_string())),
				};
				let failed = item.is_err();
				if tx.send(item).await.is_err() || failed {
					return;
				}
			}
		};
		if tokio::time::timeout(deadline, pump).await.is_err() {
			let _ = tx
				.send(Err(TransportError::Protocol("timed out".into())))
				.await;
		}
	});

	Ok(StreamResponse { status, chunks: rx })
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

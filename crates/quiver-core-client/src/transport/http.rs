//! The TCP transport, for a remote quiver.core over https with a bearer token.
//!
//! The local daemon does not come through here: it is reached over a unix
//! socket (`super::unix`) or, on Windows, a named pipe (`super::pipe`), both of
//! which only the current user can open.

use std::time::Duration;

use ::http::{self, Request, Response};
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AsyncReadWrite, Error, StreamResponse, Transport, WsStream, STREAM_BUFFER};

/// How long the TCP (and, for `https://`, TLS) handshake may take.
///
/// A peer that swallows SYNs — a machine that went away, a firewall that drops
/// rather than rejects — otherwise leaves the connect phase hanging with no
/// bound at all, because `reqwest::Client::new()` sets none.
const SHIPPED_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Upper bound on one whole request, connect included.
///
/// Deliberately LONGER than `proxy::PROXY_TIMEOUT`'s 300s, so ordinary proxied
/// traffic is bounded by the proxy's own ceiling and this is only ever the
/// backstop for the paths that do not go through it (`RemoteConnection`'s
/// health probe, `SidecarManager`'s readiness poll). A shorter value here would
/// silently shorten every proxied request to it, and the proxy would never get
/// to return the marked 504 the frontend knows how to retry.
///
/// One client with one timeout cannot serve both callers: this has to sit above
/// 300s for proxied traffic, and a health probe that takes five and a half
/// minutes to fail is not a health probe. So the probe gets its own, much
/// shorter deadline at its call site (`remote::HEALTH_TIMEOUT`) rather than a
/// second client here — a deadline at the call site also covers
/// `unix::UnixTransport`, which has no reqwest client to configure.
const SHIPPED_REQUEST_TIMEOUT: Duration = Duration::from_secs(330);

/// What the client is actually built with. Named apart from the two above so
/// that a test can still assert on the SHIPPED numbers while running against
/// the millisecond stand-ins below — a `#[cfg(not(test))]` constant is
/// invisible to the tests that most need to check it.
///
/// `CONNECT_TIMEOUT` is `pub(crate)` because `proxy`'s own ceiling has to stay
/// ABOVE it — see `proxy::PROXY_TIMEOUT` — and a test can only pin that
/// ordering against the real value.
#[cfg(not(test))]
pub(crate) const CONNECT_TIMEOUT: Duration = SHIPPED_CONNECT_TIMEOUT;
#[cfg(not(test))]
const REQUEST_TIMEOUT: Duration = SHIPPED_REQUEST_TIMEOUT;

/// The same two, shortened under test. `tokio`'s `test-util` feature (which
/// would let the suite jump straight to a timer) is not enabled in this crate,
/// so these are real waits — the same trade `proxy::PROXY_TIMEOUT` makes, and
/// for the same reason. The ordering that matters is preserved: connect is
/// still well under the whole-request bound, so a test can tell which fired.
#[cfg(test)]
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(test)]
const REQUEST_TIMEOUT: Duration = Duration::from_millis(750);

/// The TCP address to dial for a WebSocket request: the host, and the port the
/// URI names — or the scheme's default when it names none.
///
/// This function exists because the obvious spelling is wrong.
/// `TcpStream::connect(authority)` looks like it works, and does for
/// `ws://127.0.0.1:5555`, but `ToSocketAddrs for &str` REQUIRES a `:port`: a
/// remote configured as `https://core.example.com` produces the authority
/// `core.example.com`, which fails at address parsing before a packet is sent —
/// so `wss://` never even reached the TLS it also was not doing.
///
/// Brackets come off an IPv6 literal host: they are URI syntax, not part of the
/// address, and `("[::1]", 443)` does not resolve.
fn ws_target(uri: &http::Uri) -> Result<(String, u16), Error> {
    // `into_client_request` (the only caller's only source of URIs) already
    // rejects a URI with no host — tungstenite's `IntoClientRequest` returns
    // `Error::Url(UrlError::NoHostName)` — so this arm has no reachable failure
    // path through the public API, the way unix.rs documents its own.
    let host = uri
        .host()
        .ok_or_else(|| Error::Protocol("ws url has no host".into()))?;
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    let port = uri.port_u16().unwrap_or(match uri.scheme_str() {
        Some("wss") | Some("https") => 443,
        _ => 80,
    });
    Ok((host.to_string(), port))
}

pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl HttpTransport {
    // The one `expect` in the crate: see the comment at the build call. A TLS
    // backend that will not initialise is a broken build, not a runtime state.
    #[allow(clippy::expect_used)]
    pub fn new(
        base_url: impl Into<String>,
        token: Option<String>,
    ) -> Self {
        // `tcp://` is quiver.core's own spelling for "plain HTTP here".
        let base_url = base_url.into().replace("tcp://", "http://");
        Self {
            // `reqwest::Client::new()` sets NO timeouts whatsoever. A peer
            // that accepts a connection and then goes silent pins the calling
            // task forever — including `RemoteConnection::start`, which used
            // to run under `ConnectionManager`'s write lock and so took every
            // `quiver://` request in the app down with it.
            client: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .build()
                // The only documented failure is a TLS backend that will not
                // initialise, which is a broken build rather than a runtime
                // condition — and a transport with no client cannot do
                // anything useful anyway.
                .expect("reqwest's default TLS backend must initialise"),
            base_url,
            token,
        }
    }

    fn url(
        &self,
        path: &str,
    ) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn ws_url(
        &self,
        path: &str,
    ) -> String {
        let base = self
            .base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        format!("{base}{path}")
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn request(
        &self,
        req: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error> {
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_else(|| req.uri().path().to_string());

        let (parts, body) = req.into_parts();
        let mut out = self
            .client
            .request(parts.method, self.url(&path_and_query))
            .body(body);

        for (name, value) in parts.headers.iter() {
            // The inbound Host (e.g. from the `quiver://localhost` scheme
            // callers dial through) names the *local* proxy, not the remote
            // peer this transport actually talks to — forwarding it verbatim
            // would address the request to the wrong virtual host on any
            // vhosted/proxied deployment, and reqwest will not override an
            // explicitly-set Host. Let reqwest derive the correct one from
            // the target URL instead.
            if *name == http::header::HOST {
                continue;
            }
            out = out.header(name, value);
        }
        if let Some(auth) = self.auth_header() {
            out = out.header(reqwest::header::AUTHORIZATION, auth);
        }

        let resp = out.send().await.map_err(|e| {
            // `send()` spans both the connect phase and everything after a
            // connection is established (e.g. a peer that accepts then hangs
            // up before responding). Only the former is actually a Connect
            // failure — collapsing both into Connect would make this
            // transport disagree with unix.rs, whose equivalent split
            // (`UnixStream::connect` vs. `send_request`) already keeps them
            // distinct.
            if e.is_connect() {
                Error::Connect(e.to_string())
            } else {
                Error::Protocol(e.to_string())
            }
        })?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;

        let mut builder = Response::builder().status(status);
        if let Some(h) = builder.headers_mut() {
            for (name, value) in headers.iter() {
                h.insert(name, value.clone());
            }
        }
        builder
            .body(bytes.to_vec())
            .map_err(|e| Error::Protocol(e.to_string()))
    }

    async fn request_stream(
        &self,
        req: Request<Vec<u8>>,
        deadline: Duration,
    ) -> Result<StreamResponse, Error> {
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_else(|| req.uri().path().to_string());

        let (parts, body) = req.into_parts();
        // The client-wide `REQUEST_TIMEOUT` would cut a long stream off, so this
        // request carries its own deadline: reqwest lets a per-request timeout
        // override the client's.
        let mut out = self
            .client
            .request(parts.method, self.url(&path_and_query))
            .timeout(deadline)
            .body(body);
        for (name, value) in parts.headers.iter() {
            if *name == http::header::HOST {
                continue;
            }
            out = out.header(name, value);
        }
        if let Some(auth) = self.auth_header() {
            out = out.header(reqwest::header::AUTHORIZATION, auth);
        }

        let mut resp = out.send().await.map_err(|e| {
            if e.is_connect() {
                Error::Connect(e.to_string())
            } else {
                Error::Protocol(e.to_string())
            }
        })?;
        let status = resp.status().as_u16();

        let (tx, rx) = mpsc::channel(STREAM_BUFFER);
        tokio::spawn(async move {
            loop {
                // `tx.closed()` so an abandoned request ends promptly even while the
                // daemon is silent -- see `stream::request_stream_over`.
                let item = tokio::select! {
                    _ = tx.closed() => return,
                    chunk = resp.chunk() => match chunk {
                        Ok(Some(bytes)) => Ok(bytes.to_vec()),
                        Ok(None) => return,
                        Err(e) => Err(Error::Protocol(e.to_string())),
                    },
                };
                let failed = item.is_err();
                // A closed channel means the consumer abandoned the request;
                // returning drops `resp` and with it the connection.
                if tx.send(item).await.is_err() || failed {
                    return;
                }
            }
        });

        Ok(StreamResponse { status, chunks: rx })
    }

    async fn open_ws(
        &self,
        path: &str,
    ) -> Result<WsStream, Error> {
        let url = self.ws_url(path);
        let mut request = url
            .into_client_request()
            .map_err(|e| Error::Protocol(e.to_string()))?;
        if let Some(auth) = self.auth_header() {
            request.headers_mut().insert(
                tokio_tungstenite::tungstenite::http::header::AUTHORIZATION,
                auth.parse()
                    .map_err(|_| Error::Protocol("bad token".into()))?,
            );
        }

        let (host, port) = ws_target(request.uri())?;
        let stream = TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|e| Error::Connect(e.to_string()))?;
        let boxed: Box<dyn AsyncReadWrite> = Box::new(stream);
        // `client_async_tls_with_config` rather than `client_async`: it reads
        // the scheme off the request (`uri_mode`) and, for `wss://`, runs a
        // real native-tls handshake against the socket — SNI and certificate
        // verification from the system trust store included — before the
        // WebSocket upgrade is written. `client_async` skipped all of that and
        // spoke plaintext WS at a TLS listener.
        //
        // `None` connector means native-tls's own defaults, which is what we
        // want: no custom roots, no verification turned off.
        let (ws, _) = tokio_tungstenite::client_async_tls_with_config(request, boxed, None, None)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;
        Ok(ws)
    }
}

#[cfg(test)]
mod tests;

//! The `arrow-app` URI scheme: every request an arrow's page makes.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tauri::http::{header, HeaderName, Method, Request, Response, StatusCode};

use super::csp::harden;
use super::hosts::ArrowHosts;
use super::inject::inject_shim;
use super::uri::{daemon_path, host_of};
use super::{SHIM_JS, SHIM_PATH};
use crate::connection::transport::Transport;

/// Same bound as the `quiver://` proxy; shorter under test so a hung
/// transport does not stall the suite.
#[cfg(not(test))]
const ARROW_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(test)]
const ARROW_TIMEOUT: Duration = Duration::from_millis(500);

/// Headers that must not travel from an arrow's page to the daemon.
const STRIPPED: [HeaderName; 6] = [
	header::ORIGIN,
	header::REFERER,
	header::COOKIE,
	header::AUTHORIZATION,
	header::HOST,
	header::ACCEPT_ENCODING,
];

fn plain(status: u16, body: &str) -> Response<Vec<u8>> {
	let mut resp = Response::builder()
		.status(status)
		.header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
		.body(body.as_bytes().to_vec())
		.expect("static response");
	harden(&mut resp);
	resp
}

fn shim_response(method: &Method) -> Response<Vec<u8>> {
	let body = if method == Method::HEAD {
		Vec::new()
	} else {
		SHIM_JS.as_bytes().to_vec()
	};
	let mut resp = Response::builder()
		.status(StatusCode::OK)
		.header(header::CONTENT_TYPE, "text/javascript")
		.header(header::CACHE_CONTROL, "no-store")
		.body(body)
		.expect("static response");
	harden(&mut resp);
	resp
}

fn is_html(resp: &Response<Vec<u8>>) -> bool {
	resp.headers()
		.get(header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.is_some_and(|v| v.to_ascii_lowercase().starts_with("text/html"))
}

/// True when a path segment could climb out of the arrow's namespace once the
/// daemon (or anything between) decodes or normalises it. Checked on the path
/// only: `..` is ordinary in a query string.
fn has_dot_segment(path: &str) -> bool {
	path.contains('\\')
		|| path.split('/').any(|segment| {
			let lower = segment.to_ascii_lowercase();
			segment == ".." || lower.contains("%2e") || lower.contains("%5c")
		})
}

fn forward(namespace: &str, req: Request<Vec<u8>>) -> Result<Request<Vec<u8>>, String> {
	let path_and_query = req
		.uri()
		.path_and_query()
		.map(|p| p.as_str())
		.unwrap_or("/");
	let target = daemon_path(namespace, path_and_query);
	let (parts, body) = req.into_parts();

	let mut builder = Request::builder().method(parts.method).uri(target);
	for (name, value) in parts.headers.iter() {
		if !STRIPPED.contains(name) {
			builder = builder.header(name, value);
		}
	}
	builder.header(header::ACCEPT_ENCODING, "identity")
		.body(body)
		.map_err(|e| e.to_string())
}

/// Serves one `arrow-app` request. The transport is resolved inside the
/// timeout (like `quiver://`), because resolving it takes a fair lock.
pub async fn handle_request<F>(
	transport: F,
	hosts: &ArrowHosts,
	req: Request<Vec<u8>>,
) -> Response<Vec<u8>>
where
	F: Future<Output = Arc<dyn Transport>>,
{
	let Some(host) = host_of(req.uri()) else {
		return plain(400, "bad arrow host");
	};
	let Some(namespace) = hosts.namespace_for(&host) else {
		return plain(404, "unknown arrow");
	};

	let path = req.uri().path().to_string();
	if has_dot_segment(&path) {
		return plain(400, "bad arrow path");
	}
	if path == SHIM_PATH {
		return shim_response(req.method());
	}
	if path.starts_with("/__arrow/") {
		return plain(404, "not found");
	}

	let forwarded = match forward(&namespace, req) {
		Ok(r) => r,
		Err(e) => return plain(400, &e),
	};
	let proxied = async move { transport.await.request(forwarded).await };
	let mut resp = match tokio::time::timeout(ARROW_TIMEOUT, proxied).await {
		Ok(Ok(resp)) => resp,
		Ok(Err(e)) => return plain(502, &format!("quiver proxy: {e}")),
		Err(_) => return plain(504, "the arrow did not answer in time"),
	};

	if resp.status().is_success() && is_html(&resp) {
		let (mut parts, body) = resp.into_parts();
		parts.headers.remove(header::CONTENT_LENGTH);
		resp = Response::from_parts(parts, inject_shim(&body));
	}
	harden(&mut resp);
	resp
}

#[cfg(test)]
mod tests {
	use std::future::ready;
	use std::sync::{Arc, Mutex};

	use async_trait::async_trait;
	use tauri::http::{header, Method, Request, Response, StatusCode};

	use super::*;
	use crate::connection::transport::{Transport, TransportError, WsStream};

	/// Answers every request with `reply` and remembers what it was asked.
	struct Stub {
		reply: Response<Vec<u8>>,
		seen: Mutex<Vec<Request<Vec<u8>>>>,
	}

	impl Stub {
		fn new(reply: Response<Vec<u8>>) -> Arc<Self> {
			Arc::new(Self {
				reply,
				seen: Mutex::new(Vec::new()),
			})
		}
		fn last(&self) -> Request<Vec<u8>> {
			self.seen
				.lock()
				.unwrap()
				.last()
				.cloned()
				.expect("a request")
		}
		fn calls(&self) -> usize {
			self.seen.lock().unwrap().len()
		}
	}

	#[async_trait]
	impl Transport for Stub {
		async fn request(
			&self,
			req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			self.seen.lock().unwrap().push(req);
			Ok(self.reply.clone())
		}
		async fn open_ws(&self, _: &str) -> Result<WsStream, TransportError> {
			Err(TransportError::Protocol("not used".into()))
		}
	}

	/// Must never be reached: a request that gets here escaped a check.
	struct Dead;

	#[async_trait]
	impl Transport for Dead {
		async fn request(
			&self,
			_: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			panic!("transport must not be used")
		}
		async fn open_ws(&self, _: &str) -> Result<WsStream, TransportError> {
			panic!("transport must not be used")
		}
	}

	struct Hangs;

	#[async_trait]
	impl Transport for Hangs {
		async fn request(
			&self,
			_: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			std::future::pending().await
		}
		async fn open_ws(&self, _: &str) -> Result<WsStream, TransportError> {
			Err(TransportError::Protocol("not used".into()))
		}
	}

	struct Refuses;

	#[async_trait]
	impl Transport for Refuses {
		async fn request(
			&self,
			_: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			Err(TransportError::Connect("refused".into()))
		}
		async fn open_ws(&self, _: &str) -> Result<WsStream, TransportError> {
			Err(TransportError::Protocol("not used".into()))
		}
	}

	fn ok(body: &str, content_type: &str) -> Response<Vec<u8>> {
		Response::builder()
			.status(200)
			.header(header::CONTENT_TYPE, content_type)
			.header(header::CONTENT_LENGTH, body.len().to_string())
			.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
			.header("x-frame-options", "DENY")
			.body(body.as_bytes().to_vec())
			.unwrap()
	}

	fn request(uri: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method(Method::GET)
			.uri(uri)
			.body(Vec::new())
			.unwrap()
	}

	fn setup() -> (ArrowHosts, String) {
		let hosts = ArrowHosts::new();
		let host = hosts.register("github.com/user/chat");
		(hosts, host)
	}

	async fn run(
		transport: Arc<dyn Transport>,
		hosts: &ArrowHosts,
		req: Request<Vec<u8>>,
	) -> Response<Vec<u8>> {
		handle_request(ready(transport), hosts, req).await
	}

	#[tokio::test]
	async fn unknown_host_is_404_and_never_reaches_the_transport() {
		let (hosts, _) = setup();
		let resp = run(Arc::new(Dead), &hosts, request("arrow-app://deadbeef/")).await;
		assert_eq!(resp.status(), StatusCode::NOT_FOUND);
		assert!(resp.headers().contains_key(header::CONTENT_SECURITY_POLICY));
	}

	#[tokio::test]
	async fn forwards_to_the_daemon_ui_route_with_path_and_query() {
		let (hosts, host) = setup();
		let stub = Stub::new(ok("x", "text/javascript"));
		run(
			stub.clone(),
			&hosts,
			request(&format!("arrow-app://{host}/assets/app.js?v=1")),
		)
		.await;
		assert_eq!(
			stub.last().uri().path_and_query().unwrap().as_str(),
			"/v0/ui/github.com%2Fuser%2Fchat/assets/app.js?v=1"
		);
	}

	#[tokio::test]
	async fn windows_style_host_resolves() {
		let (hosts, host) = setup();
		let stub = Stub::new(ok("x", "text/plain"));
		let resp = run(
			stub.clone(),
			&hosts,
			request(&format!("http://arrow-app.{host}/a")),
		)
		.await;
		assert_eq!(resp.status(), StatusCode::OK);
		assert_eq!(stub.calls(), 1);
	}

	/// The exact form wry produces on Windows for a registered host resolves,
	/// and the bare label (the old, non-secure-context form) no longer does.
	#[tokio::test]
	async fn windows_localhost_form_resolves_and_the_bare_label_does_not() {
		let (hosts, _) = setup();
		let label = super::super::hosts::host_label("github.com/user/chat");
		let stub = Stub::new(ok("x", "text/plain"));
		let resp = run(
			stub.clone(),
			&hosts,
			request(&format!("http://arrow-app.{label}.localhost/x")),
		)
		.await;
		assert_eq!(resp.status(), StatusCode::OK);
		assert_eq!(stub.calls(), 1);

		for bare in [
			format!("http://arrow-app.{label}/x"),
			format!("arrow-app://{label}/x"),
		] {
			let resp = run(Arc::new(Dead), &hosts, request(&bare)).await;
			assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{bare}");
		}
	}

	/// The page must not be able to ride the user's credentials or leak its
	/// origin to the daemon, and bodies must arrive uncompressed so the shim
	/// can be injected.
	#[tokio::test]
	async fn strips_credentials_and_forces_identity_encoding() {
		let (hosts, host) = setup();
		let stub = Stub::new(ok("x", "text/plain"));
		let mut req = request(&format!("arrow-app://{host}/"));
		for (k, v) in [
			("origin", "arrow-app://x"),
			("referer", "arrow-app://x/"),
			("cookie", "a=b"),
			("authorization", "Bearer evil"),
			("accept-encoding", "gzip, br"),
			("x-custom", "kept"),
		] {
			req.headers_mut().insert(k, v.parse().unwrap());
		}
		run(stub.clone(), &hosts, req).await;

		let seen = stub.last();
		let h = seen.headers();
		for gone in ["origin", "referer", "cookie", "authorization"] {
			assert!(h.get(gone).is_none(), "{gone}");
		}
		assert_eq!(h.get("accept-encoding").unwrap(), "identity");
		assert_eq!(h.get("x-custom").unwrap(), "kept");
	}

	#[tokio::test]
	async fn html_gets_the_shim_and_a_fresh_length() {
		let (hosts, host) = setup();
		let html = "<html><head></head><body>hi</body></html>";
		let stub = Stub::new(ok(html, "text/html; charset=utf-8"));
		let resp = run(stub, &hosts, request(&format!("arrow-app://{host}/"))).await;

		let body = String::from_utf8(resp.body().clone()).unwrap();
		assert!(body.contains("<head><script src=\"/__arrow/shim.js\"></script>"));
		assert!(resp.headers().get(header::CONTENT_LENGTH).is_none());
	}

	#[tokio::test]
	async fn non_html_is_left_alone_and_error_pages_are_not_injected() {
		let (hosts, host) = setup();
		let js = Stub::new(ok("var a;", "text/javascript"));
		let resp = run(js, &hosts, request(&format!("arrow-app://{host}/a.js"))).await;
		assert_eq!(resp.body().as_slice(), b"var a;");

		let mut not_found = ok("<head>nope", "text/html");
		*not_found.status_mut() = StatusCode::NOT_FOUND;
		let resp = run(
			Stub::new(not_found),
			&hosts,
			request(&format!("arrow-app://{host}/x")),
		)
		.await;
		assert_eq!(resp.body().as_slice(), b"<head>nope");
	}

	#[tokio::test]
	async fn every_response_is_hardened() {
		let (hosts, host) = setup();
		let resp = run(
			Stub::new(ok("x", "text/plain")),
			&hosts,
			request(&format!("arrow-app://{host}/")),
		)
		.await;
		let h = resp.headers();
		assert_eq!(
			h.get(header::CONTENT_SECURITY_POLICY).unwrap(),
			crate::arrow_app::csp::ARROW_CSP
		);
		assert!(h.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
		assert!(h.get("x-frame-options").is_none());
	}

	#[tokio::test]
	async fn the_shim_is_served_locally_and_never_forwarded() {
		let (hosts, host) = setup();
		let resp = run(
			Arc::new(Dead),
			&hosts,
			request(&format!("arrow-app://{host}/__arrow/shim.js")),
		)
		.await;
		assert_eq!(resp.status(), StatusCode::OK);
		assert_eq!(
			resp.headers().get(header::CONTENT_TYPE).unwrap(),
			"text/javascript"
		);
		assert_eq!(resp.body().as_slice(), crate::arrow_app::SHIM_JS.as_bytes());

		let other = run(
			Arc::new(Dead),
			&hosts,
			request(&format!("arrow-app://{host}/__arrow/other")),
		)
		.await;
		assert_eq!(other.status(), StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn transport_failures_become_502_and_504_with_a_csp() {
		let (hosts, host) = setup();
		let refused = run(
			Arc::new(Refuses),
			&hosts,
			request(&format!("arrow-app://{host}/")),
		)
		.await;
		assert_eq!(refused.status(), StatusCode::BAD_GATEWAY);
		assert!(refused
			.headers()
			.contains_key(header::CONTENT_SECURITY_POLICY));

		let slow = run(
			Arc::new(Hangs),
			&hosts,
			request(&format!("arrow-app://{host}/")),
		)
		.await;
		assert_eq!(slow.status(), StatusCode::GATEWAY_TIMEOUT);
	}

	#[tokio::test]
	async fn dot_segments_in_the_path_are_400_and_never_reach_the_transport() {
		let (hosts, host) = setup();
		for path in [
			"/%2e%2e/x",
			"/a/%2E%2E/b",
			"/a/..%5cb",
			"/a/%5Cb",
			"/a/b%2fc/%2e",
			"/a/../b",
			"/..",
			"/a\\b",
		] {
			let resp = run(
				Arc::new(Dead),
				&hosts,
				request(&format!("arrow-app://{host}{path}")),
			)
			.await;
			assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{path}");
			assert!(
				resp.headers().contains_key(header::CONTENT_SECURITY_POLICY),
				"{path}"
			);
		}
	}

	#[tokio::test]
	async fn dots_in_the_query_or_a_segment_name_are_not_dot_segments() {
		let (hosts, host) = setup();
		let stub = Stub::new(ok("x", "text/plain"));
		let resp = run(
			stub.clone(),
			&hosts,
			request(&format!("arrow-app://{host}/a..b/c?x=a..b")),
		)
		.await;
		assert_eq!(resp.status(), StatusCode::OK);
		assert_eq!(stub.calls(), 1);
	}
}

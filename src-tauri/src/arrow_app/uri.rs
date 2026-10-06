//! Mapping between `arrow-app` URIs and the daemon's `/v0/ui` route.

use tauri::http::Uri;

/// wry turns `arrow-app://<host>/` into `http://arrow-app.<host>/` on Windows.
const WINDOWS_HOST_PREFIX: &str = "arrow-app.";

/// The arrow host a request was addressed to, in either platform form.
pub fn host_of(uri: &Uri) -> Option<String> {
	let host = uri.host()?;
	let host = host.strip_prefix(WINDOWS_HOST_PREFIX).unwrap_or(host);
	Some(host.to_ascii_lowercase())
}

/// Percent-encodes everything but RFC 3986 unreserved characters, so a
/// namespace is exactly one path segment (the daemon decodes `%2F`).
pub fn encode_namespace(namespace: &str) -> String {
	let mut out = String::with_capacity(namespace.len());
	for b in namespace.bytes() {
		if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
			out.push(b as char);
		} else {
			out.push_str(&format!("%{b:02X}"));
		}
	}
	out
}

/// `/v0/ui/<encoded namespace><path and query>`.
pub fn daemon_path(namespace: &str, path_and_query: &str) -> String {
	let rest = if path_and_query.is_empty() {
		"/"
	} else {
		path_and_query
	};
	let rest = if rest.starts_with('/') {
		rest.to_string()
	} else {
		format!("/{rest}")
	};
	format!("/v0/ui/{}{}", encode_namespace(namespace), rest)
}

/// Whether a WebSocket path asked for by an arrow's page is safe to forward.
pub fn is_safe_ws_path(path: &str) -> bool {
	if !path.starts_with('/')
		|| path.starts_with("//")
		|| path.contains("..")
		|| path.contains('\\')
	{
		return false;
	}
	let lower = path.to_ascii_lowercase();
	if lower.contains("%2e") || lower.contains("%5c") {
		return false;
	}
	!path.bytes().any(|b| b < 0x21 || b == 0x7f || b == b'#')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn host_of_handles_both_platform_forms_and_lowercases() {
		let mac: Uri = "arrow-app://ABCDEF/x".parse().unwrap();
		let win: Uri = "http://arrow-app.abcdef/x".parse().unwrap();
		assert_eq!(host_of(&mac).as_deref(), Some("abcdef"));
		assert_eq!(host_of(&win).as_deref(), Some("abcdef"));
	}

	#[test]
	fn host_of_keeps_the_localhost_suffix_in_both_forms() {
		let mac: Uri = "arrow-app://ABCDEF.localhost/x".parse().unwrap();
		let win: Uri = "http://arrow-app.abcdef.localhost/x".parse().unwrap();
		assert_eq!(host_of(&mac).as_deref(), Some("abcdef.localhost"));
		assert_eq!(host_of(&win).as_deref(), Some("abcdef.localhost"));
	}

	#[test]
	fn namespace_is_percent_encoded_for_one_path_segment() {
		assert_eq!(
			encode_namespace("github.com/user/chat@v1"),
			"github.com%2Fuser%2Fchat%40v1"
		);
		assert_eq!(encode_namespace("a-b_c.d~e"), "a-b_c.d~e");
	}

	#[test]
	fn daemon_path_keeps_path_and_query() {
		assert_eq!(
			daemon_path("github.com/user/chat", "/assets/app.js?v=1"),
			"/v0/ui/github.com%2Fuser%2Fchat/assets/app.js?v=1"
		);
		assert_eq!(daemon_path("a/b", ""), "/v0/ui/a%2Fb/");
		assert_eq!(daemon_path("a/b", "/"), "/v0/ui/a%2Fb/");
	}

	/// Paths come from the arrow's page, so they are hostile input.
	#[test]
	fn ws_paths_are_screened() {
		for ok in ["/ws", "/ws?username=bob", "/a/b/c"] {
			assert!(is_safe_ws_path(ok), "{ok}");
		}
		for bad in [
			"",
			"ws",
			"//evil",
			"/../v0/health",
			"/a/..",
			"/a\\b",
			"/a b",
			"/a\nb",
			"/%2e%2e/x",
			"/%2E%2E/x",
			"/a%5cb",
			"/a#frag",
			"/\u{7f}",
		] {
			assert!(!is_safe_ws_path(bad), "{bad:?}");
		}
	}
}

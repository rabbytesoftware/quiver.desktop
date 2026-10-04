//! The security headers every `arrow-app` response carries.

use tauri::http::{header, HeaderName, HeaderValue, Response};

/// The page can only talk to its own origin. `connect-src 'self'` is what
/// keeps an arrow from calling `quiver://localhost` (which answers CORS with
/// `*` and adds the bearer token in Rust). Inline scripts and styles are
/// allowed because Next static exports need them.
pub const ARROW_CSP: &str = "default-src 'self'; script-src 'self' 'unsafe-inline'; \
style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; \
connect-src 'self'; frame-src 'none'; form-action 'self'; base-uri 'self'";

const REMOVED: [HeaderName; 6] = [
	header::ACCESS_CONTROL_ALLOW_ORIGIN,
	header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
	header::ACCESS_CONTROL_EXPOSE_HEADERS,
	header::SET_COOKIE,
	HeaderName::from_static("x-frame-options"),
	HeaderName::from_static("content-security-policy-report-only"),
];

/// Replaces whatever policy the arrow sent with ours and drops headers that
/// would let the arrow loosen it or block our iframe.
pub fn harden(resp: &mut Response<Vec<u8>>) {
	let headers = resp.headers_mut();
	for name in &REMOVED {
		headers.remove(name);
	}
	headers.insert(
		header::CONTENT_SECURITY_POLICY,
		HeaderValue::from_static(ARROW_CSP),
	);
	headers.insert(
		header::X_CONTENT_TYPE_OPTIONS,
		HeaderValue::from_static("nosniff"),
	);
	headers.insert(
		header::REFERRER_POLICY,
		HeaderValue::from_static("no-referrer"),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use tauri::http::{header, HeaderValue, Response};

	fn dirty() -> Response<Vec<u8>> {
		Response::builder()
			.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
			.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
			.header(header::SET_COOKIE, "s=1")
			.header("x-frame-options", "DENY")
			.header(header::CONTENT_SECURITY_POLICY, "frame-ancestors 'none'")
			.header("content-security-policy-report-only", "default-src 'none'")
			.body(Vec::new())
			.unwrap()
	}

	/// An arrow's own `X-Frame-Options: DENY` or `frame-ancestors` would blank
	/// the iframe, and its CORS or cookie headers have no meaning here.
	#[test]
	fn harden_replaces_the_arrows_policy_with_ours() {
		let mut resp = dirty();
		harden(&mut resp);
		let h = resp.headers();
		assert_eq!(
			h.get(header::CONTENT_SECURITY_POLICY),
			Some(&HeaderValue::from_static(ARROW_CSP))
		);
		assert!(h.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
		assert!(h.get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).is_none());
		assert!(h.get(header::SET_COOKIE).is_none());
		assert!(h.get("x-frame-options").is_none());
		assert!(h.get("content-security-policy-report-only").is_none());
		assert_eq!(
			h.get("x-content-type-options"),
			Some(&HeaderValue::from_static("nosniff"))
		);
	}

	#[test]
	fn csp_blocks_reaching_the_daemon_and_nesting() {
		for directive in [
			"default-src 'self'",
			"connect-src 'self'",
			"frame-src 'none'",
			"form-action 'self'",
			"base-uri 'self'",
		] {
			assert!(ARROW_CSP.contains(directive), "{directive}");
		}
		// Next static exports need inline scripts and styles.
		assert!(ARROW_CSP.contains("script-src 'self' 'unsafe-inline'"));
		assert!(!ARROW_CSP.contains("quiver:"));
	}
}

//! The security headers every `arrow-app` response carries.

use crate::message::ArrowResponse;

/// The page can only talk to its own origin. `connect-src 'self'` is what
/// keeps an arrow from calling `quiver://localhost` (which answers CORS with
/// `*` and adds the bearer token in Rust). Inline scripts and styles are
/// allowed because Next static exports need them. `'wasm-unsafe-eval'` lets an
/// arrow compile its own WebAssembly modules (it does not allow `eval`).
pub const ARROW_CSP: &str =
    "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; \
style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; \
connect-src 'self'; frame-src 'none'; form-action 'self'; base-uri 'self'";

const REMOVED: [&str; 6] = [
    "access-control-allow-origin",
    "access-control-allow-credentials",
    "access-control-expose-headers",
    "set-cookie",
    "x-frame-options",
    "content-security-policy-report-only",
];

/// Replaces whatever policy the arrow sent with ours and drops headers that
/// would let the arrow loosen it or block our iframe.
pub fn harden(resp: &mut ArrowResponse) {
    for name in REMOVED {
        resp.headers.remove(name);
    }
    resp.headers.set("content-security-policy", ARROW_CSP);
    resp.headers.set("x-content-type-options", "nosniff");
    resp.headers.set("referrer-policy", "no-referrer");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirty() -> ArrowResponse {
        let mut resp = ArrowResponse::new(200, Vec::new());
        for (k, v) in [
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Credentials", "true"),
            ("Set-Cookie", "s=1"),
            ("X-Frame-Options", "DENY"),
            ("Content-Security-Policy", "frame-ancestors 'none'"),
            ("Content-Security-Policy-Report-Only", "default-src 'none'"),
        ] {
            resp.headers.append(k, v);
        }
        resp
    }

    /// An arrow's own `X-Frame-Options: DENY` or `frame-ancestors` would blank
    /// the iframe, and its CORS or cookie headers have no meaning here.
    #[test]
    fn harden_replaces_the_arrows_policy_with_ours() {
        let mut resp = dirty();
        harden(&mut resp);
        let h = &resp.headers;
        assert_eq!(h.get("content-security-policy"), Some(ARROW_CSP));
        assert!(h.get("access-control-allow-origin").is_none());
        assert!(h.get("access-control-allow-credentials").is_none());
        assert!(h.get("set-cookie").is_none());
        assert!(h.get("x-frame-options").is_none());
        assert!(h.get("content-security-policy-report-only").is_none());
        assert_eq!(h.get("x-content-type-options"), Some("nosniff"));
        assert_eq!(h.get("referrer-policy"), Some("no-referrer"));
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
        assert!(ARROW_CSP.contains("script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'"));
        assert!(!ARROW_CSP.contains("'unsafe-eval'"));
        assert!(!ARROW_CSP.contains("quiver:"));
    }
}

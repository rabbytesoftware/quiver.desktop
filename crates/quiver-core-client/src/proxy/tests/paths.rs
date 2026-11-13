use super::*;
use crate::proxy::policy::origin_allowed;

/// The shell never loads arrow interfaces through `quiver://` (they have
/// their own `arrow-app://` origin), so any `/v0/ui/` request here is an
/// arrow trying to reach its own pages under the shell's scheme. Refused
/// before a transport is resolved, in every spelling the HTTP transport
/// would normalise to `/v0/ui/...`: reqwest's url parser turns `\` into `/`
/// and resolves `.` and `..` (also percent-encoded) for `http://`, while
/// `http::Uri` hands them through unchanged. Also in the Windows form wry
/// hands the handler.
#[tokio::test]
async fn quiver_scheme_refuses_arrow_interface_paths_before_resolving() {
    let paths = [
        "/v0/ui/ns/evil.html",
        "/v0/ui",
        "/v0/ui/",
        "/V0/UI/ns",
        "/v0//ui/ns",
        "//v0//ui/ns/x",
        "/v0/%75i/ns",
        "/%76%30/ui/ns/x",
        "/v0\\ui/ns/e.html",
        "/v0/x/../ui/ns/e.html",
        "/v0/x/%2e%2e/ui/ns/e.html",
        "/v0/x/%2E%2E/ui/ns/e.html",
        "/v0/./ui/ns",
        "/v0/%2e/ui/ns",
        "/v0%5cui/ns",
        "/v0/x%2f..%2fui/ns",
    ];
    for host in ["quiver://localhost", "http://quiver.localhost"] {
        for path in paths {
            let uri = format!("{host}{path}");
            for method in ["GET", "POST", "OPTIONS"] {
                let resp = proxy_once(resolved(NeverDialled), shell_req(method, &uri)).await;
                assert_eq!(resp.status(), 403, "{method} {uri}");
                assert_eq!(
                    resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
                    Some(&b"error"[..]),
                    "{method} {uri}"
                );
                assert_eq!(
                    frame_guard(&resp),
                    (Some("frame-ancestors 'none'"), Some("DENY")),
                    "{method} {uri}"
                );
            }
        }
    }
}

/// The shell never sends a backslash or a dot segment, and refusing them
/// outright leaves no normalisation difference between this check and the
/// transport to exploit, whatever the rest of the path says.
#[tokio::test]
async fn quiver_scheme_refuses_backslashes_and_dot_segments_anywhere() {
    for path in [
        "/v0/arrow/..",
        "/v0/./arrow",
        "/v0/arrow/%2e%2e/health",
        "/v0\\arrow",
        "/v0/arrow%5Cx",
        "/..",
    ] {
        let resp = proxy_once(resolved(NeverDialled), get(path)).await;
        assert_eq!(resp.status(), 403, "{path}");
    }
}

/// The shell's own paths, including encoded namespaces, dots inside a
/// segment and query strings, must still reach the daemon.
#[tokio::test]
async fn the_shells_own_paths_still_pass() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = Arc::new(serving("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").await);
    for path in [
        "/v0/arrow",
        "/v0/health",
        "/v0/uix",
        "/v0/units",
        "/v0/arrow/ui/x",
        "/v0/runtime/github.com%2Fuser%2Frepo",
        "/v0/arrow.runtime",
        "/v0/arrow?namespace=a%2Fb&x=..",
        "/v0/arrow/a..b/.hidden",
    ] {
        let resp = proxy_once(ready(t.clone() as Arc<dyn Transport>), get(path)).await;
        assert_eq!(resp.status(), 200, "{path}");
    }
}

#[test]
fn only_the_apps_own_origins_are_allowed() {
    for origin in [
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
        "http://localhost:1420",
    ] {
        let mut r = req("GET", "/v0/arrow");
        r.headers_mut()
            .insert(header::ORIGIN, origin.parse().unwrap());
        assert!(origin_allowed(&r), "{origin}");
    }
    assert!(!origin_allowed(&bare("GET", "quiver://localhost/v0/arrow")));
}

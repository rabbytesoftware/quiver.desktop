use super::*;

/// `DELETE`, and `POST` with a JSON body, are not simple requests, so the
/// browser preflights them. The proxy answers; the daemon never hears about
/// it. `NeverDialled` panics if it does.
#[tokio::test]
async fn a_preflight_is_answered_locally_without_dialling_the_transport() {
    let resp = proxy_once(resolved(NeverDialled), req("OPTIONS", "/v0/arrow/x")).await;
    assert_eq!(resp.status(), 204);

    let allow = |name: header::HeaderName| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string()
    };
    let methods = allow(header::ACCESS_CONTROL_ALLOW_METHODS);
    for m in ["GET", "POST", "DELETE", "PATCH"] {
        assert!(
            methods.contains(m),
            "the frontend sends {m}; the preflight must allow it. Got {methods:?}"
        );
    }
    assert!(
        allow(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .to_lowercase()
            .contains("content-type"),
        "a JSON POST sets Content-Type, which is not a safelisted request header"
    );
    assert_eq!(
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|v| v.as_bytes()),
        Some(&b"*"[..]),
        "a preflight without an allow-origin fails the preflight itself"
    );
}

/// Without `Access-Control-Expose-Headers`, `x-quiver-proxy` is invisible to
/// the page — `res.headers.get(...)` reads `null` — so `isProxyFailure`
/// never fires and the whole cold-start retry path in
/// `src/lib/transport/api.ts` is dead. It is read on ERROR responses, so an
/// error that omits the expose header is exactly as dead as one that omits
/// the marker: every response has to carry it.
#[tokio::test]
async fn every_response_exposes_the_proxy_marker_to_the_page() {
    let _serialised = crate::FD_TESTS.lock().await;

    let ok = serving("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").await;
    let daemon_404 = serving("HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}").await;

    let cases: Vec<(&str, Response<Vec<u8>>)> = vec![
        (
            "a relayed 200",
            proxy_once(resolved(ok), get("/v0/health")).await,
        ),
        (
            "a relayed daemon 404",
            proxy_once(resolved(daemon_404), get("/v0/arrow/nope")).await,
        ),
        (
            "a marked 502",
            proxy_once(resolved(dead().await), get("/v0/health")).await,
        ),
        (
            "a marked 504",
            proxy_once(resolved(NeverResponds), get("/v0/health")).await,
        ),
        (
            "a preflight",
            proxy_once(resolved(NeverDialled), req("OPTIONS", "/v0/arrow")).await,
        ),
    ];

    for (what, resp) in cases {
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|v| v.as_bytes()),
            Some(&b"*"[..]),
            "{what} must be readable cross-origin at all"
        );
        assert_eq!(
            expose_header(&resp),
            Some(PROXY_ERROR_HEADER),
            "{what} must expose {PROXY_ERROR_HEADER}, or the page reads null for it"
        );
    }
}

/// A daemon that sets its own `Access-Control-Allow-Origin` must not leave
/// the response with two of them — a browser rejects a duplicated
/// allow-origin outright, which would break exactly the responses that were
/// working before.
#[tokio::test]
async fn a_daemons_own_allow_origin_is_replaced_not_appended() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = serving(
			"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: https://elsewhere.example\r\nContent-Length: 2\r\n\r\n{}",
		)
		.await;
    let resp = proxy_once(resolved(t), get("/v0/health")).await;
    let origins: Vec<_> = resp
        .headers()
        .get_all(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .iter()
        .map(|v| v.to_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(origins, vec!["*".to_string()], "got {origins:?}");
}

/// Arrow pages live in iframes of this very webview, so `quiver://` is
/// reachable from them. A request they make carries their own Origin and
/// must be refused before any connection is resolved.
#[tokio::test]
async fn quiver_scheme_refuses_foreign_origins_before_resolving() {
    for host in HOSTS {
        for method in METHODS {
            for origin in [
                "arrow-app://abc",
                "http://arrow-app.abc",
                "http://arrow-app.abc.localhost",
                "arrow-app://abc.localhost",
                "https://evil.example",
                "quiver://localhost",
                "http://quiver.localhost",
                "null",
                "",
            ] {
                let mut r = bare(method, &format!("{host}/v0/arrow"));
                r.headers_mut()
                    .insert(header::ORIGIN, origin.parse().unwrap());

                let resp = proxy_once(resolved(NeverDialled), r).await;
                assert_refused(&resp, &format!("{method} {host} from {origin:?}"));
            }
        }
    }
}

/// A frame that navigates itself here, and any `<img>`, `<script>` or
/// `<link>` it points here, sends no Origin; so does a same-origin GET
/// from a `quiver://` document that WebKitGTK rendered inside a frame
/// despite the framing headers. The shell's own fetches are cross-origin
/// and always carry one, so a missing Origin is never the shell.
#[tokio::test]
async fn quiver_scheme_refuses_a_request_without_an_origin_before_resolving() {
    for host in HOSTS {
        for method in METHODS {
            for path in ["/v0/arrow", "/v0/health", "/v0/arrow/x", "/"] {
                let uri = format!("{host}{path}");
                let resp = proxy_once(resolved(NeverDialled), bare(method, &uri)).await;
                assert_refused(&resp, &format!("{method} {uri} without an Origin"));
            }
        }
    }
}

#[tokio::test]
async fn a_missing_origin_is_refused_exactly_like_a_foreign_one() {
    let mut foreign = req("GET", "/v0/arrow");
    foreign
        .headers_mut()
        .insert(header::ORIGIN, "arrow-app://abc".parse().unwrap());
    let foreign = proxy_once(resolved(NeverDialled), foreign).await;
    let missing = proxy_once(
        resolved(NeverDialled),
        bare("GET", "quiver://localhost/v0/arrow"),
    )
    .await;

    assert_eq!(missing.status(), foreign.status());
    assert_eq!(missing.headers(), foreign.headers());
    assert_eq!(missing.body(), foreign.body());
}

/// Defence in depth where the engine sends Fetch Metadata: a navigation
/// or an embedding is refused even when it claims the shell's Origin.
#[tokio::test]
async fn quiver_scheme_refuses_navigations_by_fetch_metadata() {
    let cases = [
        ("sec-fetch-dest", "document"),
        ("sec-fetch-dest", "iframe"),
        ("sec-fetch-dest", "frame"),
        ("sec-fetch-dest", "embed"),
        ("sec-fetch-dest", "object"),
        ("sec-fetch-dest", "IFrame"),
        ("sec-fetch-mode", "navigate"),
        ("sec-fetch-mode", "Navigate"),
    ];
    for host in HOSTS {
        for method in METHODS {
            for (name, value) in cases {
                let mut r = shell_req(method, &format!("{host}/v0/arrow"));
                r.headers_mut().insert(name, value.parse().unwrap());
                let resp = proxy_once(resolved(NeverDialled), r).await;
                assert_refused(&resp, &format!("{method} {host} {name}: {value}"));
            }
        }
    }
}

/// What the shell's own `fetch` sends when the engine adds Fetch Metadata.
#[tokio::test]
async fn the_shells_fetch_metadata_still_passes() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = Arc::new(serving("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").await);
    for host in HOSTS {
        let mut r = shell_req("GET", &format!("{host}/v0/arrow"));
        r.headers_mut()
            .insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        r.headers_mut()
            .insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        r.headers_mut()
            .insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        let resp = proxy_once(ready(t.clone() as Arc<dyn Transport>), r).await;
        assert_eq!(resp.status(), 200, "{host}");
    }
}

#[tokio::test]
async fn quiver_scheme_refuses_a_foreign_preflight_too() {
    let mut r = req("OPTIONS", "/v0/arrow");
    r.headers_mut()
        .insert(header::ORIGIN, "arrow-app://abc".parse().unwrap());
    let resp = proxy_once(resolved(NeverDialled), r).await;
    assert_eq!(resp.status(), 403);
}

/// An arrow page can navigate its own iframe to a `quiver://` URL: CSP
/// governs fetches, not a frame navigating itself. Rendered there, the page
/// would run as `quiver://localhost`. That navigation and the document's
/// same-origin GETs carry no Origin and are refused; on engines that honour
/// them, both framing headers stop it rendering at all, so every response
/// kind still carries them. WebKitGTK ignores them on custom schemes.
#[tokio::test]
async fn every_response_refuses_to_render_inside_a_frame() {
    let _serialised = crate::FD_TESTS.lock().await;

    let ok = serving("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").await;
    let daemon_404 = serving("HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}").await;
    let mut foreign = req("GET", "/v0/arrow");
    foreign
        .headers_mut()
        .insert(header::ORIGIN, "arrow-app://abc".parse().unwrap());

    let cases: Vec<(&str, Response<Vec<u8>>)> = vec![
        (
            "a relayed 200",
            proxy_once(resolved(ok), get("/v0/health")).await,
        ),
        (
            "a relayed daemon 404",
            proxy_once(resolved(daemon_404), get("/v0/arrow/nope")).await,
        ),
        (
            "a marked 502",
            proxy_once(resolved(dead().await), get("/v0/health")).await,
        ),
        (
            "a marked 504",
            proxy_once(resolved(NeverResponds), get("/v0/health")).await,
        ),
        (
            "a preflight",
            proxy_once(resolved(NeverDialled), req("OPTIONS", "/v0/arrow")).await,
        ),
        (
            "an origin refusal",
            proxy_once(resolved(NeverDialled), foreign).await,
        ),
        (
            "a /v0/ui refusal",
            proxy_once(resolved(NeverDialled), get("/v0/ui/ns/index.html")).await,
        ),
    ];

    for (what, resp) in cases {
        assert_eq!(
            frame_guard(&resp),
            (Some("frame-ancestors 'none'"), Some("DENY")),
            "{what} must refuse to render inside a frame"
        );
    }
}

/// A daemon response that sets its own framing headers must not keep them:
/// a looser `frame-ancestors` from upstream would undo the guard.
#[tokio::test]
async fn a_daemons_own_framing_headers_are_replaced() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = serving(
			"HTTP/1.1 200 OK\r\nContent-Security-Policy: frame-ancestors *\r\nX-Frame-Options: ALLOWALL\r\nContent-Length: 2\r\n\r\n{}",
		)
		.await;
    let resp = proxy_once(resolved(t), get("/v0/health")).await;
    assert_eq!(
        resp.headers()
            .get_all(header::CONTENT_SECURITY_POLICY)
            .iter()
            .count(),
        1
    );
    assert_eq!(
        frame_guard(&resp),
        (Some("frame-ancestors 'none'"), Some("DENY"))
    );
}

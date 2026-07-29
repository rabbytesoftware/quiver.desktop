//! `RemoteConnection::new` has no injection seam — it hardcodes
//! `HttpTransport::new(...)` — so its actual wiring to `negotiate_version`
//! can't be driven from a unit test with a stub `Transport`. This drives it
//! against a real wiremock server instead.
//!
//! The assertion here is deliberately on the *request being made*, not on the
//! returned `api_version` value. `known` in `negotiate_version` is currently
//! `["v0"]`, and every fallback branch also returns `"v0"` — so "v0" is
//! simultaneously the only version negotiation can ever produce and the
//! value returned when negotiation is skipped entirely. No assertion on the
//! return value can tell those two apart (confirmed: hardcoding `let
//! api_version = "v0".to_string();` in `RemoteConnection::new` still passes
//! an `api_version`-only check). `.expect(1)` on the mock is what actually
//! proves `new()` calls `negotiate_version`, which in turn calls through the
//! transport to `GET /versions` — deleting that call makes the mock server's
//! own drop-time verification fail, because the endpoint is never hit.
use quiverdesktop_lib::connection::remote::RemoteConnection;
use quiverdesktop_lib::connection::types::QuiverConnection;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn version_negotiation_picks_supported_version() {
	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/versions"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
		    "success": true,
		    "data": {
			"version": "26.5.0",
			"build_id": "33",
			"api": {
			    "supported": ["v0"],
			    "latest": "v0",
			    "min_client_version": "1.0.0"
			}
		    }
		})))
		.expect(1)
		.mount(&server)
		.await;

	let conn = RemoteConnection::new("id".into(), "name".into(), server.uri(), "".into())
		.await
		.unwrap();

	assert_eq!(conn.config().api_version, "v0");

	// Dropping `server` here runs wiremock's expectation check: `.expect(1)`
	// above fails the test if `/versions` was requested any number of times
	// other than exactly 1. That is the actual proof `new()` still calls
	// `negotiate_version` — the return-value assertion above cannot do it
	// (see the module doc comment).
	drop(server);
}

//! `RemoteConnection::new` has no injection seam — it hardcodes
//! `HttpTransport::new(...)` — so its actual wiring to `negotiate_version`
//! can't be driven from a unit test with a stub `Transport`. This drives it
//! against a real wiremock server instead, and would fail if `new()` ever
//! stopped storing `negotiate_version`'s result in `api_version` (e.g. if it
//! were replaced with a hardcoded `"v0".to_string()`).

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
		.mount(&server)
		.await;

	let conn = RemoteConnection::new("id".into(), "name".into(), server.uri(), "".into())
		.await
		.unwrap();

	assert_eq!(conn.config().api_version, "v0");
}

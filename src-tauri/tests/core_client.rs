mod common;

use quiverdesktop_lib::connection::http::{HttpClient, HttpTransport};
use quiverdesktop_lib::connection::remote::{transport::RemoteTransport, RemoteConnection};
use quiverdesktop_lib::connection::types::QuiverConnection;
use std::sync::Arc;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_client(uri: &str) -> HttpClient {
	let transport = Arc::new(RemoteTransport::new(uri, ""));
	HttpClient::new(transport as Arc<dyn HttpTransport>)
}

#[tokio::test]
async fn fetch_arrows_returns_list_items() {
	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/v0/arrow"))
		.and(query_param("user_installed", "true"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
		    "success": true,
		    "error": null,
		    "data": [{
			"namespace": "github.com/user/repo",
			"name": "My Arrow",
			"description": "",
			"tags": [],
			"versions": [{"ref": "v1.0.0", "version": "1.0.0", "state": "ready"}]
		    }]
		})))
		.mount(&server)
		.await;

	let client = make_client(&server.uri());
	let items = client.fetch_arrows().await.unwrap();

	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["namespace"], "github.com/user/repo@v1.0.0");
	assert_eq!(items[0]["name"], "My Arrow");
}

#[tokio::test]
async fn health_returns_ok_on_200() {
	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/health"))
		.respond_with(
			ResponseTemplate::new(200)
				.set_body_json(serde_json::json!({"status": "ok"})),
		)
		.mount(&server)
		.await;

	let client = make_client(&server.uri());
	assert!(client.health().await.is_ok());
}

#[tokio::test]
async fn install_posts_to_correct_encoded_path() {
	let server = MockServer::start().await;
	Mock::given(method("POST"))
		.and(path("/v0/runtime/github.com%2Fuser%2Frepo@v1.0.0/install"))
		.respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({
		    "success": true, "data": null, "error": null
		})))
		.mount(&server)
		.await;

	let client = make_client(&server.uri());
	assert!(client
		.install("github.com/user/repo@v1.0.0", Default::default())
		.await
		.is_ok());
}

#[tokio::test]
async fn register_arrow_posts_to_correct_path() {
	let server = MockServer::start().await;
	Mock::given(method("POST"))
		.and(path("/v0/arrow/github.com%2Fuser%2Frepo@v1.0.0"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
		    "success": true, "data": null, "error": null
		})))
		.mount(&server)
		.await;

	let client = make_client(&server.uri());
	assert!(client
		.register_arrow("github.com/user/repo@v1.0.0")
		.await
		.is_ok());
}

#[tokio::test]
async fn fetch_arrows_returns_error_on_api_failure() {
	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/v0/arrow"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
		    "success": false,
		    "error": "internal error",
		    "data": null
		})))
		.mount(&server)
		.await;

	let client = make_client(&server.uri());
	assert!(client.fetch_arrows().await.is_err());
}

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

#[tokio::test]
async fn version_negotiation_falls_back_on_404() {
	let server = MockServer::start().await;
	// No /versions mock mounted — wiremock returns 404 by default

	let conn = RemoteConnection::new("id".into(), "name".into(), server.uri(), "".into())
		.await
		.unwrap();

	assert_eq!(conn.config().api_version, "v0");
}

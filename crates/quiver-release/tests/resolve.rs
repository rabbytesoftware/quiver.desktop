//! End-to-end resolution against a local server standing in for github.com.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use quiver_release::{kind, resolve, DEFAULT_REPO};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SUM_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SUM_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

// ── the release page ─────────────────────────────────────────────────

const REPO_PATH: &str = "/rabbytesoftware/quiver.desktop";

/// An `expanded_assets` page: one list item per asset, the digest as text
/// beside the link, and the two source archives GitHub always adds.
fn page(tag: &str, assets: &[(&str, Option<&str>)]) -> String {
    let mut html = String::from("<div><ul class=\"list-style-none\">");
    for (name, digest) in assets {
        html.push_str(&format!(
			"<li class=\"Box-row\"><a href=\"{REPO_PATH}/releases/download/{tag}/{name}\" rel=\"nofollow\"><span>{name}</span></a>{}</li>",
			digest.map_or(String::new(), |d| format!("<span class=\"digest\">sha256:{d}</span>"))
		));
    }
    html.push_str(&format!(
		"<li><a href=\"{REPO_PATH}/archive/refs/tags/{tag}.zip\">Source code (zip)</a></li></ul></div>"
	));
    html
}

async fn serve(tag: &str, html: String) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("{REPO_PATH}/releases/expanded_assets/{tag}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(&server)
        .await;
    server
}

async fn latest_redirects_to(server: &MockServer, location: &str) {
    Mock::given(method("GET"))
        .and(path(format!("{REPO_PATH}/releases/latest")))
        .respond_with(ResponseTemplate::new(302).insert_header("location", location))
        .mount(server)
        .await;
}

#[tokio::test]
async fn the_normal_case_resolves_a_url_and_a_checksum() {
    let server = serve(
        "stable-26.9",
        page(
            "stable-26.9",
            &[("quiver-desktop_0.1.0_amd64.AppImage", Some(SUM_A))],
        ),
    )
    .await;
    latest_redirects_to(&server, &format!("{REPO_PATH}/releases/tag/stable-26.9")).await;

    let resolved = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
        .await
        .unwrap();

    assert_eq!(resolved.tag, "stable-26.9");
    assert_eq!(resolved.name, "quiver-desktop_0.1.0_amd64.AppImage");
    assert_eq!(
        resolved.url,
        format!(
            "{}{REPO_PATH}/releases/download/stable-26.9/quiver-desktop_0.1.0_amd64.AppImage",
            server.uri()
        )
    );
    assert_eq!(resolved.checksum.as_deref(), Some(SUM_A));
}

#[tokio::test]
async fn an_explicit_tag_never_asks_which_release_is_the_latest() {
    let server = serve(
        "nightly-rolling",
        page("nightly-rolling", &[("Quiver.AppImage", Some(SUM_A))]),
    )
    .await;

    let resolved = resolve(
        &server.uri(),
        DEFAULT_REPO,
        Some("nightly-rolling"),
        "linux",
        "x86_64",
    )
    .await
    .unwrap();

    assert_eq!(resolved.tag, "nightly-rolling");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// An asset without a digest on the page still verifies, off the manifest
/// `install.sh` has always read.
#[tokio::test]
async fn a_digestless_asset_falls_back_to_the_published_manifest() {
    let server = serve(
        "t",
        page(
            "t",
            &[("Quiver.AppImage", None), ("SHA256SUMS", Some(SUM_A))],
        ),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(format!("{REPO_PATH}/releases/download/t/SHA256SUMS")))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(format!("{SUM_B}  Quiver.AppImage\n")),
        )
        .mount(&server)
        .await;

    let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap();
    assert_eq!(resolved.checksum.as_deref(), Some(SUM_B));
}

/// Resolution SUCCEEDS with no checksum; refusing to install is the
/// caller's decision, not this function's.
#[tokio::test]
async fn a_release_with_no_checksum_anywhere_resolves_without_one() {
    let server = serve("t", page("t", &[("Quiver.AppImage", None)])).await;

    let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap();
    assert_eq!(resolved.checksum, None);
}

/// A manifest that is published but 404s, or carries no line for this
/// asset, is the same as no manifest -- never a hard failure, and never a
/// checksum invented to fill the hole.
#[tokio::test]
async fn an_unreadable_checksum_manifest_is_the_same_as_none() {
    let server = serve(
        "t",
        page("t", &[("Quiver.AppImage", None), ("checksums.txt", None)]),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "{REPO_PATH}/releases/download/t/checksums.txt"
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let resolved = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap();
    assert_eq!(resolved.checksum, None);
}

#[tokio::test]
async fn no_asset_for_this_platform_is_reported_as_such() {
    let server = serve(
        "t",
        page("t", &[("quiver-desktop_0.1.0_amd64.deb", Some(SUM_A))]),
    )
    .await;

    let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::NO_ASSET);
}

/// A repository whose only releases are pre-releases redirects `latest` to
/// the releases list, which is no release to install.
#[tokio::test]
async fn a_repository_with_no_release_is_distinguished_from_a_dead_network() {
    let server = MockServer::start().await;
    latest_redirects_to(&server, &format!("{REPO_PATH}/releases")).await;

    let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::NO_RELEASE);
}

#[tokio::test]
async fn a_page_that_is_not_a_release_page_is_reported_as_malformed() {
    let server = serve("t", "<html>nope</html>".to_string()).await;

    let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::MALFORMED);
}

#[tokio::test]
async fn a_latest_that_does_not_redirect_is_reported_as_malformed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("{REPO_PATH}/releases/latest")))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html></html>"))
        .mount(&server)
        .await;

    let err = resolve(&server.uri(), DEFAULT_REPO, None, "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::MALFORMED);
}

#[tokio::test]
async fn a_throttled_release_page_is_distinguished_from_everything_else() {
    for status in [403, 429] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string("slow down"))
            .mount(&server)
            .await;

        let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
            .await
            .unwrap_err();
        assert_eq!(err.kind(), kind::RATE_LIMITED, "status {status}");
    }
}

/// Everything that is neither a quota refusal nor a missing release: a
/// 500, a 502 from something in front of GitHub, a 401 from a proxy. The
/// UI has one sentence for all of them, but it must be THAT sentence and
/// not "check your connection", which would send the user to fix
/// something that is not broken.
#[tokio::test]
async fn any_other_bad_status_is_its_own_kind() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream is unwell"))
        .mount(&server)
        .await;

    let err = resolve(&server.uri(), DEFAULT_REPO, Some("t"), "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::HTTP);
}

#[tokio::test]
async fn an_unknown_tag_is_a_missing_release() {
    let server = MockServer::start().await;

    let err = resolve(&server.uri(), DEFAULT_REPO, Some("nope"), "linux", "x86_64")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), kind::NO_RELEASE);
}

/// No server at all. Port 1 is reserved and nothing binds it, so the
/// connection is refused rather than merely unanswered -- which is the
/// distinction that matters: a refusal must read as `offline`, not get
/// mistaken for an HTTP answer. A dropped `MockServer`'s port is NOT a
/// substitute; it is released back to the OS and can be reused, and this
/// test caught exactly that by reporting `no_release`.
#[tokio::test]
async fn an_unreachable_origin_reports_offline() {
    let err = resolve(
        "http://127.0.0.1:1",
        DEFAULT_REPO,
        Some("t"),
        "linux",
        "x86_64",
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), kind::OFFLINE);
}

//! The two kinds of request the resolver makes, and how their failures are
//! classified.

use crate::page::percent_decode;
use crate::{Error, Result};

/// GitHub rejects a request that arrives without one.
const USER_AGENT: &str = concat!("quiver.desktop/", env!("CARGO_PKG_VERSION"));

const REQUEST_TIMEOUT_SECS: u64 = 15;

/// A client configured the way GitHub expects to be called. Redirects are not
/// followed: the tag of the latest release is read off the redirect itself.
pub(crate) fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| Error::Offline(err.to_string()))
}

/// Turns a non-success status into the error the UI branches on.
fn status_error(status: reqwest::StatusCode, body: &str) -> Error {
    let detail = format!("GitHub answered {status}: {body}");
    match status.as_u16() {
        403 | 429 => Error::RateLimited(detail),
        404 => Error::NoRelease(detail),
        _ => Error::Http(detail),
    }
}

/// Sends one GET. Only a transport failure is an error here; the status is the
/// caller's to read.
pub(crate) async fn get_page(http: &reqwest::Client, url: &str) -> Result<reqwest::Response> {
    http.get(url)
        .send()
        .await
        .map_err(|err| Error::Offline(err.to_string()))
}

/// Reads a response as text. A non-success status becomes its error kind.
pub(crate) async fn page_body(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| Error::Offline(err.to_string()))?;
    if !status.is_success() {
        // Truncated: this ends up in a dialog next to a sentence a person has
        // to read.
        let excerpt: String = body.chars().take(200).collect();
        return Err(status_error(status, &excerpt));
    }
    Ok(body)
}

/// The tag of the newest published release: where `/releases/latest`
/// redirects to. A repository with no release redirects to `/releases`.
pub(crate) async fn latest_tag(http: &reqwest::Client, origin: &str, repo: &str) -> Result<String> {
    let response = get_page(http, &format!("{origin}/{repo}/releases/latest")).await?;
    let status = response.status();
    if !status.is_redirection() {
        let err = match page_body(response).await {
            Err(err) => err,
            Ok(_) => Error::Malformed("the latest release did not redirect".into()),
        };
        return Err(err);
    }
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|location| location.to_str().ok())
        .and_then(|location| location.split_once("/releases/tag/"))
        .map(|(_, tag)| percent_decode(tag))
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| Error::NoRelease("the repository has no published release".into()))
}

//! Version negotiation with a freshly dialled peer.

use serde::Deserialize;

use crate::transport::Transport;

pub(super) async fn negotiate_version(transport: &dyn Transport) -> String {
    #[derive(Deserialize)]
    struct VersionApi {
        supported: Vec<String>,
    }
    #[derive(Deserialize)]
    struct VersionData {
        api: VersionApi,
    }
    #[derive(Deserialize)]
    struct VersionEnvelope {
        data: Option<VersionData>,
    }

    let req = match ::http::Request::builder()
        .method("GET")
        .uri("quiver://localhost/versions")
        .body(Vec::new())
    {
        Ok(r) => r,
        Err(_) => return "v0".into(),
    };

    let resp = match transport.request(req).await {
        Ok(r) => r,
        Err(_) => return "v0".into(),
    };

    if !resp.status().is_success() {
        return "v0".into();
    }

    let env = match serde_json::from_slice::<VersionEnvelope>(resp.body()) {
        Ok(e) => e,
        Err(_) => return "v0".into(),
    };

    let data = match env.data {
        Some(d) => d,
        None => return "v0".into(),
    };

    // Only one known version exists so far, and every fallback branch above
    // also returns "v0" — so no test can tell "negotiated v0" apart from
    // "negotiation never ran, defaulted to v0" by inspecting this return
    // value alone until a second entry lands here. See
    // tests/remote_connection.rs for how that's worked around today (asserting
    // the request was made, not what it returned).
    let known = ["v0"];
    for version in known.iter().rev() {
        if data.api.supported.iter().any(|s| s == version) {
            return version.to_string();
        }
    }
    "v0".into()
}

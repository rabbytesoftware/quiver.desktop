//! Device-pairing: trading a one-time code for a bearer token.

use serde::{Deserialize, Serialize};

use crate::transport::Transport;
use crate::{Error, Result};

/// Exchanges a one-time device-pairing code for a long-lived bearer token.
///
/// Only meaningful against a `tcp://`-bound core -- a `unix://` daemon has no
/// auth surface at all, so nothing calls this on the local path. The caller
/// passes a throwaway, TOKENLESS transport pointed at the candidate URL:
/// there is nothing to authenticate with yet, since the token this call
/// returns is the only one that will ever exist for this pairing.
pub async fn redeem_pairing_code(
    transport: &dyn Transport,
    code: &str,
    device_id: &str,
    label: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct RedeemRequest<'a> {
        code: &'a str,
        device_id: &'a str,
        label: &'a str,
    }
    #[derive(Deserialize)]
    struct SessionTokenDTO {
        token: String,
    }
    #[derive(Deserialize)]
    struct RedeemEnvelope {
        data: Option<SessionTokenDTO>,
    }

    let body = serde_json::to_vec(&RedeemRequest {
        code,
        device_id,
        label,
    })
    .map_err(|e| Error::Pairing(format!("pairing: build request: {e}")))?;

    let req = ::http::Request::builder()
        .method("POST")
        .uri("quiver://localhost/v0/auth/pairing/redeem")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| Error::Pairing(format!("pairing: build request: {e}")))?;

    let resp = transport
        .request(req)
        .await
        .map_err(|e| Error::Pairing(format!("pairing: {e}")))?;

    if !resp.status().is_success() {
        return Err(Error::Pairing(format!(
            "pairing failed: status {}",
            resp.status()
        )));
    }

    let envelope: RedeemEnvelope = serde_json::from_slice(resp.body())
        .map_err(|e| Error::Pairing(format!("pairing: malformed response: {e}")))?;

    envelope
        .data
        .map(|d| d.token)
        .ok_or_else(|| Error::Pairing("pairing failed: no token in response".into()))
}

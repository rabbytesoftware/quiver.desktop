//! Messages between the main process and the helper process that shows one
//! arrow app in a native webview (`quiver --arrow-window`). One JSON object
//! per line over the helper's stdin (host to window) and stdout (window to
//! host). The window never decides anything: it forwards every request of
//! the `arrow-app` protocol and renders the answer.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::message::Headers;
use crate::Result;

/// Request bodies and response bodies are bytes; JSON carries them as base64.
mod base64_bytes {
    use super::{Engine, STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        STANDARD.decode(text).map_err(serde::de::Error::custom)
    }
}

/// A request the webview made to the `arrow-app` custom protocol.
/// `id` correlates it with its [`ProtocolResponse`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolRequest {
    pub id: u64,
    pub method: String,
    /// Absolute, in either platform form (see [`crate::host_of`]).
    pub uri: String,
    pub headers: Headers,
    #[serde(with = "base64_bytes")]
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolResponse {
    pub id: u64,
    pub status: u16,
    pub headers: Headers,
    #[serde(with = "base64_bytes")]
    pub body: Vec<u8>,
}

/// Main process to window process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostToWindow {
    /// Load this URL (see [`crate::app_url`]).
    Navigate {
        url: String,
    },
    /// Remount the page: its document and sockets are replaced.
    Reload,
    /// Close the window and exit.
    Close,
    Title {
        title: String,
    },
    /// The answer to a [`WindowToHost::ProtocolRequest`] with the same `id`.
    ProtocolResponse(ProtocolResponse),
}

/// Window process to main process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowToHost {
    /// The webview exists and accepts commands.
    Ready,
    /// The user closed the window (or the webview died).
    Closed,
    ProtocolRequest(ProtocolRequest),
}

/// One message as a single line (no trailing newline).
pub fn encode_line<T: Serialize>(msg: &T) -> Result<String> {
    Ok(serde_json::to_string(msg)?)
}

pub fn decode_line<'a, T: Deserialize<'a>>(line: &'a str) -> Result<T> {
    Ok(serde_json::from_str(line.trim_end())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ProtocolRequest {
        ProtocolRequest {
            id: 7,
            method: "POST".into(),
            uri: "arrow-app://abc.localhost/api?x=1".into(),
            headers: [("content-type", "application/json")].into_iter().collect(),
            body: vec![0, 159, 255, b'\n'],
        }
    }

    #[test]
    fn host_to_window_round_trips() {
        let msgs = [
            HostToWindow::Navigate {
                url: "arrow-app://abc.localhost/".into(),
            },
            HostToWindow::Reload,
            HostToWindow::Close,
            HostToWindow::Title {
                title: "Chat".into(),
            },
            HostToWindow::ProtocolResponse(ProtocolResponse {
                id: 7,
                status: 200,
                headers: Headers::new(),
                body: b"\xff\x00".to_vec(),
            }),
        ];
        for msg in msgs {
            let line = encode_line(&msg).unwrap();
            assert!(!line.contains('\n'), "{line}");
            assert_eq!(decode_line::<HostToWindow>(&line).unwrap(), msg);
        }
    }

    #[test]
    fn window_to_host_round_trips_with_binary_bodies_on_one_line() {
        for msg in [
            WindowToHost::Ready,
            WindowToHost::Closed,
            WindowToHost::ProtocolRequest(request()),
        ] {
            let line = encode_line(&msg).unwrap();
            assert!(!line.contains('\n'), "{line}");
            assert_eq!(
                decode_line::<WindowToHost>(&format!("{line}\n")).unwrap(),
                msg
            );
        }
    }

    #[test]
    fn wire_format_is_tagged_snake_case() {
        assert_eq!(
            encode_line(&WindowToHost::Ready).unwrap(),
            r#"{"type":"ready"}"#
        );
        assert_eq!(
            encode_line(&HostToWindow::Reload).unwrap(),
            r#"{"type":"reload"}"#
        );
        let line = encode_line(&WindowToHost::ProtocolRequest(request())).unwrap();
        assert!(
            line.starts_with(r#"{"type":"protocol_request","id":7"#),
            "{line}"
        );
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(decode_line::<WindowToHost>("not json").is_err());
        assert!(decode_line::<WindowToHost>(r#"{"type":"nope"}"#).is_err());
        assert!(decode_line::<WindowToHost>(
			r#"{"type":"protocol_request","id":1,"method":"GET","uri":"x","headers":[],"body":"!!"}"#
		)
		.is_err());
    }
}

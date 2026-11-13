//! Messages between the host and the embedded webview that shows one arrow app inside the main window.
//! The view never decides anything: it forwards every request of the `arrow-app` protocol and renders
//! the answer. The messages serialise as one JSON object per line, so they can also cross a pipe.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::message::Headers;
use crate::Result;

/// Request bodies and response bodies are bytes; JSON carries them as base64.
mod base64_bytes {
    use super::{Engine, STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        bytes: &[u8],
        s: S,
    ) -> Result<S::Ok, S::Error> {
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

/// The rectangle of the main window the page covers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PageBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Host to embedded page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Where the page sits, in logical pixels from the main window's top-left corner.
    Bounds(PageBounds),
    /// Show or hide the page without closing it (it keeps its state while hidden).
    Visible {
        visible: bool,
    },
    /// The answer to a [`WindowToHost::ProtocolRequest`] with the same `id`.
    ProtocolResponse(ProtocolResponse),
    /// A message for the shim in the page (`window.__arrowShimReceive`), as JSON.
    Deliver {
        json: String,
    },
}

/// Embedded page to host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowToHost {
    /// The webview exists and accepts commands.
    Ready,
    /// The user closed the window (or the webview died).
    Closed,
    ProtocolRequest(ProtocolRequest),
    /// What the shim in the page posted (`window.ipc.postMessage`), as JSON text.
    Shim {
        json: String,
    },
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
            HostToWindow::Bounds(PageBounds {
                x: 240.0,
                y: 56.5,
                width: 960.0,
                height: 744.0,
            }),
            HostToWindow::Visible { visible: false },
            HostToWindow::Deliver {
                json: r#"{"type":"ws-open","id":"a"}"#.into(),
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
            WindowToHost::Shim {
                json: r#"{"type":"hello"}"#.into(),
            },
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

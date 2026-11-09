//! Who may use the `quiver://` scheme, and which paths it refuses to serve.

use ::http::{header, Request};

/// Origins the app's own pages send. Arrow pages (`arrow-app://...`) share
/// this webview process, so anything else must not reach the daemon through
/// this scheme.
///
/// A request WITHOUT an Origin is refused too. The shell's page lives on a
/// different origin from this scheme, so every `fetch` it makes here is a
/// CORS request and the browser always attaches its Origin. What arrives
/// without one is a frame navigating itself here, an `<img>`/`<script>`/
/// `<link>` pointed here, or a same-origin GET from a `quiver://` document
/// rendered in a frame: none of them is the shell, and the last one is real
/// on WebKitGTK, which ignores the framing headers on custom schemes.
const OWN_ORIGINS: [&str; 4] = [
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
    "http://localhost:1420",
];

pub(super) fn origin_allowed(req: &Request<Vec<u8>>) -> bool {
    req.headers()
        .get(header::ORIGIN)
        .and_then(|o| o.to_str().ok())
        .is_some_and(|o| OWN_ORIGINS.contains(&o))
}

/// `Sec-Fetch-Dest` values that mean the response would become a document.
const DOCUMENT_DESTS: [&str; 5] = ["document", "iframe", "frame", "embed", "object"];

pub(super) fn fetch_metadata(req: &Request<Vec<u8>>, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_ascii_lowercase)
}

/// Fetch Metadata, when the engine sends it, as defence in depth: a
/// navigation or an embedding is never the shell, whatever Origin it claims.
/// Not every engine sends these headers, so the Origin rule stands alone.
pub(super) fn is_navigation(req: &Request<Vec<u8>>) -> bool {
    fetch_metadata(req, "sec-fetch-mode").is_some_and(|m| m == "navigate")
        || fetch_metadata(req, "sec-fetch-dest")
            .is_some_and(|d| DOCUMENT_DESTS.contains(&d.as_str()))
}

pub(super) fn request_allowed(req: &Request<Vec<u8>>) -> bool {
    origin_allowed(req) && !is_navigation(req)
}

pub(super) fn percent_decode(path: &str) -> String {
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                decoded.push(b);
                i += 3;
            }
            (b, _) => {
                decoded.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Whether `quiver://` must refuse `path` before it reaches a transport.
///
/// The daemon's arrow interfaces (`/v0/ui...`) are refused in every spelling
/// that could reach them: the daemon routes on the decoded path, and the HTTP
/// transport's url parser turns `\` into `/` and resolves `.` and `..`
/// (encoded too) where `http::Uri` leaves them alone. Backslashes and dot
/// segments are refused outright, since the shell never sends them and that
/// leaves no normalisation difference to exploit; what remains is compared
/// with empty segments dropped and case folded.
pub(super) fn is_refused_path(path: &str) -> bool {
    let decoded = percent_decode(path);
    if decoded.contains('\\') {
        return true;
    }
    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "." | ".." => return true,
            "" => {}
            s => segments.push(s.to_lowercase()),
        }
    }
    segments.starts_with(&["v0".to_string(), "ui".to_string()])
}

//! What the box can observe about an arrow app from outside both processes: the OS's TCP listeners, the daemon's
//! run directory, the `/v0/ui` proxy with chosen headers, and the X windows the helper opens.

use std::collections::BTreeSet;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::process::Command;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use super::daemon::enc;
use super::home::Home;

/// Every TCP listener (`/proc/net/tcp[6]`, state 0A), as `local-address-hex:port-hex`.
pub fn tcp_listeners() -> BTreeSet<String> {
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .flat_map(|file| {
            std::fs::read_to_string(file)
                .unwrap_or_default()
                .lines()
                .skip(1)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let local = cols.nth(1)?;
            (cols.nth(1)? == "0A").then(|| local.to_string())
        })
        .collect()
}

/// The first 12 hex digits of the sha256 of the namespace: the name of the arrow's socket in the run directory.
pub fn socket_name(identity: &str) -> String {
    let out = Command::new("sh")
        .arg("-c")
        .arg(r#"printf %s "$1" | sha256sum | cut -c1-12"#)
        .arg("_")
        .arg(identity)
        .output()
        .expect("sha256sum");
    format!("{}.sock", String::from_utf8_lossy(&out.stdout).trim())
}

/// `(mode, socket file names)` of `<QUIVER_HOME>/run`.
pub fn run_dir(home: &Home) -> (u32, Vec<String>) {
    let dir = home.quiver().join("run");
    let mode = std::fs::metadata(&dir)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(0);
    let mut sockets: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_socket()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    sockets.sort();
    (mode, sockets)
}

/// One request to `/v0/ui/<identity><rest>` on the daemon's socket with exactly these extra headers.
pub async fn ui_request(
    socket: &Path,
    identity: &str,
    rest: &str,
    headers: &[(&str, &str)],
) -> (u16, String) {
    let mut stream = UnixStream::connect(socket).await.expect("connect");
    let mut head = format!(
        "GET /v0/ui/{}{rest} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n",
        enc(identity)
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read");
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, body.to_string())
}

/// The X windows whose title contains `title` (`xdotool search --name`): the helper process opens one per app.
pub fn windows(title: &str) -> Vec<String> {
    let out = Command::new("xdotool")
        .args(["search", "--name", title])
        .output()
        .expect("run xdotool");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Moves the windows titled `title` off the right edge of the screen. The helper's window opens over the app's own
/// (the window manager places it), and the bridge's clicks are real mouse events: the sidebar has to stay
/// reachable for the scenarios that open several apps in turn.
pub fn park_windows(title: &str) {
    for id in windows(title) {
        let _ = Command::new("xdotool")
            .args(["windowmove", &id, "4000", "0"])
            .status();
    }
}

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

/// A window's rectangle on the display.
#[derive(Debug, Clone, Copy)]
pub struct PageRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The main window's position on the display and the mapped windows directly inside it that are big enough to
/// be a page (the arrow apps' webviews are child windows of the main window).
#[derive(Debug)]
pub struct PageWindows {
    pub main_x: i32,
    pub main_y: i32,
    pub pages: Vec<PageRect>,
}

/// Reads the X window tree (`xwininfo -root -tree`). Lines look like
/// `     0x400003 "Quiver": ("quiver" "Quiver")  1200x800+0+0  +10+30`: id, name, class, size and position
/// inside the parent, and the position on the display. Children are indented deeper than their parent.
pub fn page_windows() -> PageWindows {
    let out = Command::new("xwininfo")
        .args(["-root", "-tree"])
        .output()
        .expect("run xwininfo");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut seen = PageWindows {
        main_x: 0,
        main_y: 0,
        pages: Vec::new(),
    };
    let mut main_indent = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let Some((id, rest)) = line.trim().split_once(' ') else {
            continue;
        };
        if !id.starts_with("0x") {
            continue;
        }
        let mut geometry = rest.split_whitespace().rev();
        let (Some(absolute), Some(size)) = (geometry.next(), geometry.next()) else {
            continue;
        };
        let parse = |s: &str| -> Option<(i32, i32, i32, i32)> {
            let (wh, pos) = s.split_once('+')?;
            let (w, h) = wh.split_once('x')?;
            let (x, y) = pos.split_once('+')?;
            Some((
                x.parse().ok()?,
                y.parse().ok()?,
                w.parse().ok()?,
                h.parse().ok()?,
            ))
        };
        let Some((_, _, width, height)) = parse(size) else {
            continue;
        };
        let Some((ax, ay, _, _)) = parse(&format!("0x0+{}", absolute.trim_start_matches('+')))
        else {
            continue;
        };
        match main_indent {
            None if rest.contains("\"Quiver\"") && width >= 600 => {
                main_indent = Some(indent);
                seen.main_x = ax;
                seen.main_y = ay;
            }
            Some(main) if indent <= main => main_indent = None,
            Some(main) if indent == main + 3 && width >= 50 && height >= 50 => {
                let viewable = Command::new("xwininfo")
                    .args(["-id", id])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).contains("IsViewable"))
                    .unwrap_or(false);
                if viewable {
                    seen.pages.push(PageRect {
                        x: ax,
                        y: ay,
                        width,
                        height,
                    });
                }
            }
            _ => {}
        }
    }
    seen
}

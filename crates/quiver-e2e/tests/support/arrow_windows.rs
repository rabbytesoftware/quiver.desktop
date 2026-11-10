//! The pages arrow apps show inside the main window. Their DOM is not reachable from the bridge, so the
//! scenarios look at three things: the app's own account of them (`state.dump` > `arrowWindows`: open apps,
//! visible one, remount keys and the protocol requests the pages made, with the CSP of each answer), the X
//! window tree (the page is a child window of the main window, over the pane body, and never a window of its
//! own), and the daemon's `/v0/ui` proxy. The first two are not signalled, so they are re-read.

use serde_json::Value;

use super::app::App;
use super::sandbox::page_windows;
use super::wait::{eventually, SHORT};

/// The CSP every `arrow-app://` answer carries (`quiver_arrow_host::ARROW_CSP`).
pub const ARROW_CSP: &str = "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; \
style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; \
frame-src 'none'; form-action 'self'; base-uri 'self'";

fn bare(identity: &str) -> &str {
    identity.split('@').next().unwrap_or(identity)
}

/// `state.dump` > `arrowWindows`, with every namespace reduced to the bare one the scenarios name (the app keys
/// its windows by catalog identity, `ns@selector`).
pub async fn dump(app: &mut App) -> Value {
    let mut d = app.bridge.state().await.expect("state.dump")["arrowWindows"].clone();
    for list in ["open", "windows"] {
        if let Some(items) = d[list].as_array_mut() {
            for item in items {
                *item = Value::String(bare(item.as_str().unwrap_or_default()).to_string());
            }
        }
    }
    if let Some(keys) = d["reloadKeys"].as_object_mut() {
        *keys = keys
            .iter()
            .map(|(k, v)| (bare(k).to_string(), v.clone()))
            .collect();
    }
    if let Some(entries) = d["protocol"].as_array_mut() {
        for e in entries {
            e["namespace"] =
                Value::String(bare(e["namespace"].as_str().unwrap_or_default()).to_string());
        }
    }
    d
}

/// The open apps, least recently used first.
pub async fn open_apps(app: &mut App) -> Vec<String> {
    dump(app).await["open"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Waits until the app reports exactly these namespaces (any order) as open; returns them LRU first.
pub async fn wait_open(app: &mut App, expected: &[&str]) -> Vec<String> {
    let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    want.sort();
    let mut last = Vec::new();
    for _ in 0..100 {
        last = open_apps(app).await;
        let mut have = last.clone();
        have.sort();
        if have == want {
            return last;
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    app.fail(&format!("open arrow apps: wanted {want:?}, have {last:?}"))
        .await
}

/// Re-reads the dump until `ok` holds (the helper windows are not signalled), failing the test with the screen.
pub async fn until(app: &mut App, what: &str, ok: impl Fn(&Value) -> bool) -> Value {
    let mut last = Value::Null;
    for _ in 0..150 {
        last = dump(app).await;
        if ok(&last) {
            return last;
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    app.fail(&format!("{what}; arrowWindows: {last}")).await
}

/// The protocol requests made on behalf of `namespace`, oldest first.
pub async fn requests(app: &mut App, namespace: &str) -> Vec<Value> {
    dump(app).await["protocol"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| e["namespace"] == namespace)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Waits until the app has exactly `count` live pages (the supervisor's `windows`, hidden ones included).
pub async fn wait_pages(app: &mut App, count: usize) {
    until(app, &format!("{count} live page(s)"), |d| {
        d["windows"].as_array().is_some_and(|a| a.len() == count)
    })
    .await;
}

/// Waits until the X window tree shows `count` mapped page windows inside the main window, each exactly over
/// the pane body (`arrow-app.body`),; a page that opened as a window of its own would not be found there.
pub async fn wait_embedded(app: &mut App, count: usize) {
    let body = if count > 0 {
        let n = app.node("arrow-app.body").await;
        Some((n.x, n.y, n.w, n.h))
    } else {
        None
    };
    eventually(
        &format!("{count} page window(s) inside the main window"),
        SHORT,
        || async {
            let seen = page_windows();
            let placed = body.is_none_or(|(x, y, w, h)| {
                seen.pages.iter().all(|p| {
                    (p.x - (seen.main_x + x as i32)).abs() <= 2
                        && (p.y - (seen.main_y + y as i32)).abs() <= 2
                        && (p.width - w as i32).abs() <= 2
                        && (p.height - h as i32).abs() <= 2
                })
            });
            (seen.pages.len() == count && placed).then_some(())
        },
    )
    .await;
}

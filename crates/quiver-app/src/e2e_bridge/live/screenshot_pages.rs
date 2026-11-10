//! Arrow-app pages are child windows of the main window on a 32-bit visual: reading the main window (or the
//! screen) returns them black, because X has no compositor here. Each page is read on its own and laid over
//! the main capture, so a screenshot shows what a user sees.

use std::process::Command;

struct Page {
    id: String,
    x: i32,
    y: i32,
}

/// `main_png` with every page window of `main` composited on top; unchanged when there is none or a tool fails.
pub fn with_pages(main: &str, main_png: Vec<u8>) -> Vec<u8> {
    let pages = pages_of(main);
    if pages.is_empty() {
        return main_png;
    }
    let dir = std::env::temp_dir().join(format!("quiver-shot-{}", std::process::id()));
    if std::fs::create_dir_all(&dir).is_err() {
        return main_png;
    }
    let base = dir.join("main.png");
    let composed = std::fs::write(&base, &main_png)
        .ok()
        .and_then(|()| overlay(&dir, &base, &pages))
        .unwrap_or(main_png);
    let _ = std::fs::remove_dir_all(&dir);
    composed
}

fn overlay(dir: &std::path::Path, base: &std::path::Path, pages: &[Page]) -> Option<Vec<u8>> {
    for (i, page) in pages.iter().enumerate() {
        let shot = dir.join(format!("page-{i}.png"));
        let ok = Command::new("import")
            .args(["-window", &page.id])
            .arg(&shot)
            .status()
            .ok()?
            .success();
        if !ok {
            continue;
        }
        Command::new("composite")
            .args(["-geometry", &format!("+{}+{}", page.x, page.y)])
            .arg(&shot)
            .arg(base)
            .arg(base)
            .status()
            .ok()?;
    }
    std::fs::read(base).ok()
}

/// The viewable windows directly inside `main` that are big enough to be a page, from `xwininfo -children`.
fn pages_of(main: &str) -> Vec<Page> {
    let Some(tree) = xwininfo(&["-children", "-id", main]) else {
        return Vec::new();
    };
    tree.lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|id| id.starts_with("0x"))
        .filter_map(|id| {
            let info = xwininfo(&["-id", id])?;
            let field = |name: &str| {
                info.lines()
                    .find_map(|l| l.trim().strip_prefix(name))
                    .and_then(|v| v.trim().parse::<i32>().ok())
            };
            let big = field("Width:")? >= 50 && field("Height:")? >= 50;
            (big && info.contains("IsViewable")).then(|| Page {
                id: id.to_string(),
                x: field("Relative upper-left X:").unwrap_or(0),
                y: field("Relative upper-left Y:").unwrap_or(0),
            })
        })
        .collect()
}

fn xwininfo(args: &[&str]) -> Option<String> {
    let out = Command::new("xwininfo").args(args).output().ok()?;
    String::from_utf8(out.stdout).ok()
}

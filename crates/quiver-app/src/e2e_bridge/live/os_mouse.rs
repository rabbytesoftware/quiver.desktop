//! Real pointer input through the OS.
//!
//! GPUI 0.2.2 keeps `Window::dispatch_event` unusable from outside (its return type is private), so the bridge
//! injects pointer events the way a user's hardware would: through the window system. That also exercises the
//! platform input path, not just GPUI's dispatcher. Implemented for X11 via `xdotool` (the Linux e2e box);
//! other platforms report a clear error until a driver is added.

use std::process::Command;

fn xdotool(args: &[&str]) -> Result<String, String> {
    let out = Command::new("xdotool")
        .args(args)
        .output()
        .map_err(|e| format!("xdotool is required: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// The numbers `xwininfo` reports for one X window.
fn window_info(id: &str) -> Result<std::collections::HashMap<String, i32>, String> {
    let info = Command::new("xwininfo")
        .args(["-id", id])
        .output()
        .map_err(|e| format!("xwininfo is required: {e}"))?;
    Ok(String::from_utf8_lossy(&info.stdout)
        .lines()
        .filter_map(|l| {
            let (key, value) = l.trim().split_once(':')?;
            Some((key.to_owned(), value.trim().parse::<i32>().ok()?))
        })
        .collect())
}

/// Top-left of the app window on the display (window coordinates start there). The arrow apps' pages are
/// child windows of the same process, so the main window is the biggest one the process shows.
fn window_origin() -> Result<(i32, i32), String> {
    let pid = std::process::id().to_string();
    let ids = xdotool(&["search", "--onlyvisible", "--pid", &pid])?;
    // `xdotool getwindowgeometry` reports the position of the window's frame under a reparenting window manager
    // (openbox puts a 20px title bar above the client), which sent every click 20px too low; `xwininfo` reports
    // where the client area really is, and window coordinates start there.
    let main = ids
        .lines()
        .filter_map(|id| window_info(id.trim()).ok())
        .max_by_key(|w| w.get("Width").unwrap_or(&0) * w.get("Height").unwrap_or(&0))
        .ok_or("no visible window for this process")?;
    let read = |key: &str| {
        main.get(key)
            .copied()
            .ok_or_else(|| format!("no {key} in xwininfo output"))
    };
    Ok((
        read("Absolute upper-left X")?,
        read("Absolute upper-left Y")?,
    ))
}

fn to_screen(
    x: f32,
    y: f32,
) -> Result<(String, String), String> {
    let (ox, oy) = window_origin()?;
    Ok((
        (ox + x.round() as i32).to_string(),
        (oy + y.round() as i32).to_string(),
    ))
}

pub fn move_to(
    x: f32,
    y: f32,
) -> Result<(), String> {
    let (sx, sy) = to_screen(x, y)?;
    xdotool(&["mousemove", &sx, &sy]).map(drop)
}

pub fn click(
    x: f32,
    y: f32,
    double: bool,
) -> Result<(), String> {
    move_to(x, y)?;
    let repeat = if double { "2" } else { "1" };
    xdotool(&["click", "--repeat", repeat, "--delay", "20", "1"]).map(drop)
}

pub fn right_click(
    x: f32,
    y: f32,
) -> Result<(), String> {
    move_to(x, y)?;
    xdotool(&["click", "3"]).map(drop)
}

/// X11 keysyms (`"Down Return"`), each pressed and released in turn, to whichever window has the keyboard.
pub fn key(keys: &str) -> Result<(), String> {
    let mut args = vec!["key", "--delay", "80"];
    args.extend(keys.split_whitespace());
    xdotool(&args).map(drop)
}

/// Wheel notches: X11 buttons 4/5 are up/down, 6/7 left/right. One notch is about 40 logical pixels.
pub fn scroll(
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
) -> Result<(), String> {
    move_to(x, y)?;
    for (delta, negative, positive) in [(dy, "4", "5"), (dx, "6", "7")] {
        let notches = (delta.abs() / 40.0).ceil() as u32;
        if notches == 0 {
            continue;
        }
        let button = if delta < 0.0 { negative } else { positive };
        xdotool(&["click", "--repeat", &notches.to_string(), button]).map(drop)?;
    }
    Ok(())
}

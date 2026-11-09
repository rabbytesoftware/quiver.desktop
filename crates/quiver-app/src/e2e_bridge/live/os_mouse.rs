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

/// Top-left of the app window on the display (window coordinates start there).
fn window_origin() -> Result<(i32, i32), String> {
    let pid = std::process::id().to_string();
    let ids = xdotool(&["search", "--onlyvisible", "--pid", &pid])?;
    let id = ids
        .lines()
        .last()
        .ok_or("no visible window for this process")?
        .trim()
        .to_string();
    let geometry = xdotool(&["getwindowgeometry", "--shell", &id])?;
    let read = |key: &str| {
        geometry
            .lines()
            .find_map(|l| {
                l.strip_prefix(key)
                    .and_then(|v| v.trim().parse::<i32>().ok())
            })
            .ok_or_else(|| format!("no {key} in window geometry"))
    };
    Ok((read("X=")?, read("Y=")?))
}

fn to_screen(x: f32, y: f32) -> Result<(String, String), String> {
    let (ox, oy) = window_origin()?;
    Ok((
        (ox + x.round() as i32).to_string(),
        (oy + y.round() as i32).to_string(),
    ))
}

pub fn move_to(x: f32, y: f32) -> Result<(), String> {
    let (sx, sy) = to_screen(x, y)?;
    xdotool(&["mousemove", &sx, &sy]).map(drop)
}

pub fn click(x: f32, y: f32, double: bool) -> Result<(), String> {
    move_to(x, y)?;
    let repeat = if double { "2" } else { "1" };
    xdotool(&["click", "--repeat", repeat, "--delay", "20", "1"]).map(drop)
}

/// Wheel notches: X11 buttons 4/5 are up/down, 6/7 left/right. One notch is about 40 logical pixels.
pub fn scroll(x: f32, y: f32, dx: f32, dy: f32) -> Result<(), String> {
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

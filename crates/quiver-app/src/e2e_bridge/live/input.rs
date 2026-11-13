//! Synthesising real GPUI input events from stable element ids.

use super::os_mouse;
use gpui::{
    point, px, size, AnyWindowHandle, AppContext as _, AsyncApp, Keystroke, Modifiers, Pixels,
    Point,
};
use quiver_ui::inspect;

fn center_of(id: &str) -> Result<Point<Pixels>, String> {
    let node = inspect::find(id)
        .ok_or_else(|| format!("no element with id {id:?} was painted in the last frame"))?;
    if !node.enabled {
        return Err(format!("element {id:?} is disabled"));
    }
    Ok(point(px(node.x + node.w / 2.0), px(node.y + node.h / 2.0)))
}

fn target(id: &str) -> Result<(f32, f32), String> {
    let p = center_of(id)?;
    Ok((f32::from(p.x), f32::from(p.y)))
}

pub fn hover(
    _cx: &mut AsyncApp,
    _window: AnyWindowHandle,
    id: &str,
) -> Result<(), String> {
    let (x, y) = target(id)?;
    os_mouse::move_to(x, y)
}

pub fn click(
    _cx: &mut AsyncApp,
    _window: AnyWindowHandle,
    id: &str,
    double: bool,
) -> Result<(), String> {
    let (x, y) = target(id)?;
    os_mouse::click(x, y, double)
}

/// The element's centre with the secondary button, through the window system.
pub fn right_click(id: &str) -> Result<(), String> {
    let (x, y) = target(id)?;
    os_mouse::right_click(x, y)
}

pub fn press(
    cx: &mut AsyncApp,
    window: AnyWindowHandle,
    keys: &str,
) -> Result<(), String> {
    for chord in keys.split_whitespace() {
        let mut keystroke =
            Keystroke::parse(chord).map_err(|e| format!("bad key chord {chord:?}: {e}"))?;
        // GPUI's parsed `enter` carries a "\n" a field would insert as text; the window system's Return does not.
        // (Not enough here: `dispatch_keystroke` simulates an IME and puts the "\n" back, so a single-line field
        // that lets the key propagate still receives it after PressEnter. The search bar ignores that edit.)
        if keystroke.key == "enter" {
            keystroke.key_char = None;
        }
        cx.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx)
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn type_text(
    cx: &mut AsyncApp,
    window: AnyWindowHandle,
    id: Option<&str>,
    text: &str,
) -> Result<(), String> {
    if let Some(id) = id {
        click(cx, window, id, false)?;
    }
    for ch in text.chars() {
        let keystroke = Keystroke {
            modifiers: Modifiers::none(),
            key: ch.to_string(),
            key_char: Some(ch.to_string()),
        };
        cx.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx)
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn scroll(
    _cx: &mut AsyncApp,
    _window: AnyWindowHandle,
    id: &str,
    dx: f32,
    dy: f32,
) -> Result<(), String> {
    let (x, y) = target(id)?;
    os_mouse::scroll(x, y, dx, dy)
}

pub fn resize(
    cx: &mut AsyncApp,
    window: AnyWindowHandle,
    width: f32,
    height: f32,
) -> Result<(), String> {
    cx.update_window(window, |_, window, _| {
        window.resize(size(px(width), px(height)))
    })
    .map_err(|e| e.to_string())
}

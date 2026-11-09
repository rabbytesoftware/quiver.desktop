//! The 5px drag handle on the rail's inner edge. Drags use `quiver_state::sidebar::width_after`;
//! arrow keys nudge by 8px (`nudge_for_key`).

use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, DragMoveEvent, IntoElement, Render, Window};
use quiver_state::shell::SidebarSide;
use quiver_state::sidebar::{nudge_for_key, width_after};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

use super::super::Sidebar;

/// The value carried by a rail drag: the width it started from.
#[derive(Clone)]
pub struct RailDrag {
    from: f64,
}

/// What GPUI shows under the pointer while dragging: nothing.
struct Invisible;

impl Render for Invisible {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn resize_handle(sidebar: &Sidebar, cx: &mut Context<Sidebar>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let shell = sidebar.app.read(cx).shell;
    let from = shell.sidebar_width;
    let side = shell.sidebar_side;
    let (app_drag, app_key) = (sidebar.app.clone(), sidebar.app.clone());
    div()
        .id("sidebar-resize")
        .absolute()
        .top_0()
        .bottom_0()
        .when(side == SidebarSide::Left, |d| d.right(px(-2.)))
        .when(side == SidebarSide::Right, |d| d.left(px(-2.)))
        .w(px(5.))
        .cursor(CursorStyle::ResizeLeftRight)
        .hover(|s| s.bg(p.ring.opacity(0.4)))
        .focusable()
        .on_drag(RailDrag { from }, |_, _, _, cx| cx.new(|_| Invisible))
        .on_drag_move(move |e: &DragMoveEvent<RailDrag>, window, cx| {
            let from = e.drag(cx).from;
            let pointer = f64::from(f32::from(e.event.position.x));
            let window_w = f64::from(f32::from(window.viewport_size().width));
            let edge = if side == SidebarSide::Left {
                from
            } else {
                window_w - from
            };
            let width = width_after(side, pointer - edge, from);
            app_drag.update(cx, |m, cx| m.set_sidebar_width(width, cx));
        })
        .on_key_down(move |e, _, cx| {
            let key = match e.keystroke.key.as_str() {
                "left" => "ArrowLeft",
                "right" => "ArrowRight",
                other => other,
            };
            if let Some(dx) = nudge_for_key(key) {
                let width = width_after(side, dx, from);
                app_key.update(cx, |m, cx| m.set_sidebar_width(width, cx));
            }
        })
        .child(probe(
            "sidebar.resize",
            Kind::Other,
            "Resize sidebar",
            true,
            "",
        ))
}

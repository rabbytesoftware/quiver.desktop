//! `ScrollArea`: port of `src/components/ui/scroll-area.tsx`. GPUI draws no scrollbars, so this
//! paints the 6px rounded `bg-foreground/20` thumb itself (shown while the pointer is over the
//! area, draggable), and the optional `scrollFade` as background-coloured gradients at the edges
//! (GPUI has no mask-image; pass the surface colour with [`ScrollArea::fade_color`]).

use gpui::prelude::*;
use gpui::{
    canvas, div, linear_color_stop, linear_gradient, point, px, transparent_black, AnyElement, App,
    Axis, ElementId, Hsla, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollHandle,
    StyleRefinement, Window,
};

use crate::ids::child_id;
use crate::theme::ActiveQuiver;

const THUMB: f32 = 6.0;
const MARGIN: f32 = 4.0;
const FADE: f32 = 24.0;
const MIN_THUMB: f32 = 20.0;

/// Thumb length and offset along a track of `track` px for a viewport/content pair.
pub(crate) fn thumb_geometry(
    viewport: f32,
    content: f32,
    scrolled: f32,
    track: f32,
) -> Option<(f32, f32)> {
    if content <= viewport + 0.5 || track <= 0.0 {
        return None;
    }
    let len = (viewport / content * track).clamp(MIN_THUMB.min(track), track);
    let max_scroll = content - viewport;
    let pos = (scrolled / max_scroll).clamp(0.0, 1.0) * (track - len);
    Some((len, pos))
}

#[derive(Default)]
struct Drag {
    active: Option<Axis>,
    /// Pointer coordinate and scroll offset when the drag began.
    origin: (f32, f32),
}

#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    horizontal: bool,
    fade: Option<Hsla>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl ScrollArea {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            horizontal: false,
            fade: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Also scroll horizontally.
    pub fn horizontal(mut self, on: bool) -> Self {
        self.horizontal = on;
        self
    }

    /// `scrollFade`: fade the content into `surface` at scrollable edges.
    pub fn fade_color(mut self, surface: Hsla) -> Self {
        self.fade = Some(surface);
        self
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn edge_fade(color: Hsla, top: bool, strength: f32) -> impl IntoElement {
    let (from, to) = (color.opacity(strength), transparent_black());
    let angle = if top { 180.0 } else { 0.0 };
    let base = div()
        .absolute()
        .left_0()
        .right_0()
        .h(px(FADE))
        .bg(linear_gradient(
            angle,
            linear_color_stop(from, 0.0),
            linear_color_stop(to, 1.0),
        ));
    if top {
        base.top_0()
    } else {
        base.bottom_0()
    }
}

fn thumb(
    id: &ElementId,
    axis: Axis,
    (len, pos): (f32, f32),
    group: &str,
    color: Hsla,
    handle: &ScrollHandle,
    drag: &gpui::Entity<Drag>,
) -> impl IntoElement {
    let vertical = axis == Axis::Vertical;
    let (handle, drag) = (handle.clone(), drag.clone());
    let base = div()
        .id(child_id(id, if vertical { "thumb-y" } else { "thumb-x" }))
        .absolute()
        .rounded_full()
        .bg(color)
        .opacity(0.0)
        .group_hover(group.to_string(), |s| s.opacity(1.0));
    let base = if vertical {
        base.right(px(MARGIN))
            .top(px(MARGIN + pos))
            .w(px(THUMB))
            .h(px(len))
    } else {
        base.bottom(px(MARGIN))
            .left(px(MARGIN + pos))
            .h(px(THUMB))
            .w(px(len))
    };
    base.on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, _, cx| {
        let o = handle.offset();
        let (pointer, start) = if vertical {
            (f32::from(e.position.y), f32::from(o.y))
        } else {
            (f32::from(e.position.x), f32::from(o.x))
        };
        drag.update(cx, |s, _| {
            s.active = Some(axis);
            s.origin = (pointer, start);
        });
        cx.stop_propagation();
    })
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.quiver().palette;
        let handle = window
            .use_keyed_state(child_id(&self.id, "handle"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let drag = window.use_keyed_state(child_id(&self.id, "drag"), cx, |_, _| Drag::default());
        let group = format!("scroll-{}", self.id);
        let group_for_bars = group.clone();
        let (view, max, off) = (handle.bounds(), handle.max_offset(), handle.offset());
        let (vh, vw) = (f32::from(view.size.height), f32::from(view.size.width));
        let v_thumb = thumb_geometry(
            vh,
            vh + f32::from(max.height),
            -f32::from(off.y),
            vh - 2.0 * MARGIN,
        );
        let h_thumb = self
            .horizontal
            .then(|| {
                thumb_geometry(
                    vw,
                    vw + f32::from(max.width),
                    -f32::from(off.x),
                    vw - 2.0 * MARGIN,
                )
            })
            .flatten();
        let thumb_color = p.foreground.opacity(0.2);

        let bar = |axis: Axis, geo: Option<(f32, f32)>| {
            geo.map(|(len, pos)| {
                thumb(
                    &self.id,
                    axis,
                    (len, pos),
                    &group_for_bars,
                    thumb_color,
                    &handle,
                    &drag,
                )
            })
        };

        let tracker = canvas(|_, _, _| (), {
            let (drag, handle) = (drag.clone(), handle.clone());
            move |_, _, window: &mut Window, _: &mut App| {
                let (d2, h2) = (drag.clone(), handle.clone());
                window.on_mouse_event(move |e: &MouseMoveEvent, _, window, cx| {
                    let (active, origin) = {
                        let s = d2.read(cx);
                        (s.active, s.origin)
                    };
                    let Some(axis) = active else { return };
                    let (view, max) = (h2.bounds(), h2.max_offset());
                    let (len_v, max_v, pointer) = if axis == Axis::Vertical {
                        (
                            f32::from(view.size.height),
                            f32::from(max.height),
                            f32::from(e.position.y),
                        )
                    } else {
                        (
                            f32::from(view.size.width),
                            f32::from(max.width),
                            f32::from(e.position.x),
                        )
                    };
                    let Some((tl, _)) =
                        thumb_geometry(len_v, len_v + max_v, 0.0, len_v - 2.0 * MARGIN)
                    else {
                        return;
                    };
                    let ratio = max_v / (len_v - 2.0 * MARGIN - tl).max(1.0);
                    let next = (origin.1 - (pointer - origin.0) * ratio).clamp(-max_v, 0.0);
                    let cur = h2.offset();
                    h2.set_offset(if axis == Axis::Vertical {
                        point(cur.x, px(next))
                    } else {
                        point(px(next), cur.y)
                    });
                    window.refresh();
                });
                let d3 = drag.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                    d3.update(cx, |s, _| s.active = None)
                });
            }
        })
        .absolute()
        .size_full();

        let fade_top = self
            .fade
            .filter(|_| f32::from(off.y) < -0.5)
            .map(|c| edge_fade(c, true, (-f32::from(off.y) / FADE).min(1.0)));
        let fade_bottom = self
            .fade
            .filter(|_| f32::from(max.height) + f32::from(off.y) > 0.5)
            .map(|c| {
                edge_fade(
                    c,
                    false,
                    ((f32::from(max.height) + f32::from(off.y)) / FADE).min(1.0),
                )
            });
        let key = self.id.to_string();
        let mut el = div()
            .relative()
            .size_full()
            .min_h_0()
            .group(group)
            .debug_selector(move || key)
            .child(
                div()
                    .id(child_id(&self.id, "viewport"))
                    .size_full()
                    .track_scroll(&handle)
                    .overflow_y_scroll()
                    .when(self.horizontal, |d| d.overflow_x_scroll())
                    .on_scroll_wheel(move |_, window, _| window.refresh())
                    .child(div().min_w_0().children(self.children)),
            )
            .child(tracker)
            .children(fade_top)
            .children(fade_bottom)
            .children(bar(Axis::Vertical, v_thumb))
            .children(bar(Axis::Horizontal, h_thumb));
        el.style().refine(&self.style);
        el
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_thumb_when_content_fits() {
        assert_eq!(thumb_geometry(100.0, 100.0, 0.0, 92.0), None);
    }

    #[test]
    fn thumb_is_proportional_and_reaches_the_end() {
        let (len, pos) = thumb_geometry(100.0, 400.0, 0.0, 100.0).unwrap();
        assert_eq!((len, pos), (25.0, 0.0));
        let (len, pos) = thumb_geometry(100.0, 400.0, 300.0, 100.0).unwrap();
        assert_eq!(pos + len, 100.0);
    }

    #[test]
    fn thumb_never_shrinks_below_the_minimum() {
        let (len, _) = thumb_geometry(100.0, 100_000.0, 0.0, 100.0).unwrap();
        assert_eq!(len, MIN_THUMB);
    }
}

//! `Spinner`: Lucide's `Loader2` arc rotating once per second (`animate-spin`).

use gpui::prelude::*;
use std::time::Duration;

use gpui::{
    canvas, div, point, px, Animation, AnimationExt, App, Bounds, ElementId, Hsla, IntoElement,
    PathBuilder, Pixels, Point, RenderOnce, Window,
};

/// The arc `M21 12a9 9 0 1 1-6.219-8.56` of the 24x24 icon: 0 to 288 degrees clockwise.
const SWEEP_DEG: f32 = 288.0;
const STEPS: usize = 24;

fn arc_points(origin: Point<Pixels>, size: f32, rotation_deg: f32) -> Vec<Point<Pixels>> {
    let k = size / 24.0;
    (0..=STEPS)
        .map(|i| {
            let a = (rotation_deg + SWEEP_DEG * i as f32 / STEPS as f32).to_radians();
            point(
                origin.x + px((12.0 + 9.0 * a.cos()) * k),
                origin.y + px((12.0 + 9.0 * a.sin()) * k),
            )
        })
        .collect()
}

fn arc(size: f32, color: Option<Hsla>, rotation_deg: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let color = color.unwrap_or_else(|| window.text_style().color);
            let mut b = PathBuilder::stroke(px(2.0 * size / 24.0));
            let pts = arc_points(bounds.origin, size, rotation_deg);
            if let Some((first, rest)) = pts.split_first() {
                b.move_to(*first);
                for p in rest {
                    b.line_to(*p);
                }
            }
            if let Ok(path) = b.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(size))
    .flex_none()
}

#[derive(IntoElement)]
pub struct Spinner {
    id: ElementId,
    size: f32,
    color: Option<Hsla>,
}

impl Spinner {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            size: 16.0,
            color: None,
        }
    }

    pub fn size(mut self, px_: f32) -> Self {
        self.size = px_;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (size, color) = (self.size, self.color);
        div()
            .size(px(size))
            .flex_none()
            .child(arc(size, color, 0.0))
            .with_animation(
                self.id,
                Animation::new(Duration::from_secs(1)).repeat(),
                move |_, delta| {
                    div()
                        .size(px(size))
                        .flex_none()
                        .child(arc(size, color, delta * 360.0))
                },
            )
    }
}

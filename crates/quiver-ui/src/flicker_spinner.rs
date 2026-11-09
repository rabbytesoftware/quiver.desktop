//! `FlickerSpinner`: the dotted 5x5 spinner. 8 frames, `steps(8)` over 0.72s, infinite.
//! Port of `src/components/ui/flicker-spinner.tsx`; the timing and frames must not change.

use gpui::prelude::*;
use std::time::Duration;

use gpui::{px, Animation, AnimationExt, App, ElementId, Hsla, IntoElement, RenderOnce, Window};

use crate::dot_grid::{dot_grid, dots_from_bits};
use crate::flicker_frames::FRAMES;

pub const FLICKER_PERIOD: Duration = Duration::from_millis(720);

/// `steps(8)`: the frame shown at animation progress `delta` in `0..=1`.
pub fn frame_at(delta: f32) -> usize {
    ((delta * FRAMES.len() as f32) as usize).min(FRAMES.len() - 1)
}

#[derive(IntoElement)]
pub struct FlickerSpinner {
    id: ElementId,
    size: f32,
    color: Option<Hsla>,
}

impl FlickerSpinner {
    /// `size-4` (16px) like the original default.
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

    /// Overrides `currentColor`.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for FlickerSpinner {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (size, color) = (self.size, self.color);
        let first = dot_grid(dots_from_bits(&FRAMES[0]), size, color);
        gpui::div()
            .size(px(size))
            .flex_none()
            .child(first)
            .with_animation(
                self.id,
                Animation::new(FLICKER_PERIOD).repeat(),
                move |_, delta| {
                    gpui::div().size(px(size)).flex_none().child(dot_grid(
                        dots_from_bits(&FRAMES[frame_at(delta)]),
                        size,
                        color,
                    ))
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_8_covers_every_frame_once() {
        assert_eq!(frame_at(0.0), 0);
        assert_eq!(frame_at(0.124), 0);
        assert_eq!(frame_at(0.126), 1);
        assert_eq!(frame_at(0.999), 7);
        assert_eq!(frame_at(1.0), 7);
    }
}

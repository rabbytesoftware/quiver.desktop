//! Status glyphs from the design boards (SidebarAfter / TileAfter): the same 5x5 dot grid as the
//! FlickerSpinner, drawn as a state.

use gpui::prelude::*;
use std::time::Duration;

use gpui::{
    div, px, Animation, AnimationExt, App, ElementId, Hsla, IntoElement, RenderOnce, Styled, Window,
};

use crate::dot_grid::{dot_grid, dots_from_bits, Dots, DIM};
use crate::flicker_spinner::FlickerSpinner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind {
    /// 3x3 dotted square centred in the grid; the dots blink in turn along the diagonal.
    Running,
    /// A '!' made of dots.
    NeedsAttention,
    /// An up arrow made of dots.
    UpdateAvailable,
    /// The FlickerSpinner.
    Busy,
}

const BANG: [u8; 5] = [0b00100, 0b00100, 0b00100, 0b00000, 0b00100];
const ARROW_UP: [u8; 5] = [0b00100, 0b01110, 0b10101, 0b00100, 0b00100];

const RUN_PERIOD: f32 = 1.4;
const RUN_DELAY_BASE: f32 = 0.30;
const RUN_DELAY_STEP: f32 = 0.15;

/// `cubic-bezier(.42,0,.58,1)` (CSS `ease-in-out`), solved numerically.
fn ease_in_out(x: f32) -> f32 {
    let bez = |t: f32, a: f32, b: f32| {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t.powi(3)
    };
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if bez(mid, 0.42, 0.58) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bez((lo + hi) / 2.0, 0.0, 1.0)
}

/// `run-blink`: 1 -> 0 -> 1 over the period, eased per half.
fn blink(t: f32) -> f32 {
    let p = t.rem_euclid(RUN_PERIOD) / RUN_PERIOD;
    if p < 0.5 {
        1.0 - ease_in_out(p * 2.0)
    } else {
        ease_in_out((p - 0.5) * 2.0)
    }
}

/// Dot opacities of the Running glyph `elapsed` seconds into the loop.
pub fn running_dots(elapsed: f32) -> Dots {
    let mut d = [[DIM; 5]; 5];
    for (r, row) in d.iter_mut().enumerate().take(4).skip(1) {
        for (c, dot) in row.iter_mut().enumerate().take(4).skip(1) {
            let delay = RUN_DELAY_BASE + RUN_DELAY_STEP * ((r - 1) + (c - 1)) as f32;
            *dot = blink(elapsed - delay);
        }
    }
    d
}

#[derive(IntoElement)]
pub struct StatusGlyph {
    id: ElementId,
    kind: GlyphKind,
    size: f32,
    color: Option<Hsla>,
}

impl StatusGlyph {
    /// 16px, like the sidebar rows. Tiles use 12.
    pub fn new(id: impl Into<ElementId>, kind: GlyphKind) -> Self {
        Self {
            id: id.into(),
            kind,
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

impl RenderOnce for StatusGlyph {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (size, color) = (self.size, self.color);
        match self.kind {
            GlyphKind::Busy => {
                let mut s = FlickerSpinner::new(self.id).size(size);
                if let Some(c) = color {
                    s = s.color(c);
                }
                s.into_any_element()
            }
            GlyphKind::NeedsAttention => {
                dot_grid(dots_from_bits(&BANG), size, color).into_any_element()
            }
            GlyphKind::UpdateAvailable => {
                dot_grid(dots_from_bits(&ARROW_UP), size, color).into_any_element()
            }
            GlyphKind::Running => div()
                .size(px(size))
                .flex_none()
                .child(dot_grid(running_dots(0.0), size, color))
                .with_animation(
                    self.id,
                    Animation::new(Duration::from_secs_f32(RUN_PERIOD)).repeat(),
                    move |_, delta| {
                        div().size(px(size)).flex_none().child(dot_grid(
                            running_dots(delta * RUN_PERIOD),
                            size,
                            color,
                        ))
                    },
                )
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_keeps_the_ring_dim_and_the_centre_lit() {
        let d = running_dots(RUN_DELAY_BASE);
        assert_eq!(d[0][0], DIM);
        assert_eq!(d[4][2], DIM);
        assert!(d[1][1] > 0.99, "a dot is solid when its own blink starts");
    }

    #[test]
    fn dots_blink_along_the_diagonal() {
        let t = RUN_DELAY_BASE + RUN_PERIOD / 4.0;
        let d = running_dots(t);
        assert!(
            d[1][1] < d[3][3],
            "the first dot is further into its blink than the last"
        );
    }

    #[test]
    fn bang_and_arrow_match_the_boards() {
        let bang = dots_from_bits(&BANG);
        assert_eq!(bang[3][2], DIM);
        assert_eq!(bang[4][2], 1.0);
        let arrow = dots_from_bits(&ARROW_UP);
        assert_eq!((arrow[2][0], arrow[2][4], arrow[1][1]), (1.0, 1.0, 1.0));
    }
}

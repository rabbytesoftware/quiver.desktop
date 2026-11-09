//! `WavyLine`: a thin sine-wave rule (the "Load more" divider on the design boards).

use gpui::prelude::*;
use gpui::{canvas, point, px, App, Bounds, Hsla, PathBuilder, Pixels, StyleRefinement, Window};

use crate::theme::ActiveQuiver;

/// Sample points of one wave across `width`, `y` relative to the centre line.
pub(crate) fn wave_points(width: f32, amplitude: f32, wavelength: f32) -> Vec<(f32, f32)> {
    let steps = ((width / wavelength) * 12.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|i| {
            let x = width * i as f32 / steps as f32;
            (
                x,
                amplitude * (x / wavelength * std::f32::consts::TAU).sin(),
            )
        })
        .collect()
}

#[derive(IntoElement)]
pub struct WavyLine {
    color: Option<Hsla>,
    amplitude: f32,
    wavelength: f32,
    thickness: f32,
    style: StyleRefinement,
}

impl WavyLine {
    pub fn new() -> Self {
        Self {
            color: None,
            amplitude: 2.0,
            wavelength: 10.0,
            thickness: 1.0,
            style: StyleRefinement::default(),
        }
    }

    /// Defaults to the `border` token.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn amplitude(mut self, px_: f32) -> Self {
        self.amplitude = px_;
        self
    }

    pub fn wavelength(mut self, px_: f32) -> Self {
        self.wavelength = px_;
        self
    }

    pub fn thickness(mut self, px_: f32) -> Self {
        self.thickness = px_;
        self
    }
}

impl Default for WavyLine {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for WavyLine {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for WavyLine {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.quiver().palette.border);
        let (amp, len, thick) = (self.amplitude, self.wavelength, self.thickness);
        let mut el = canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
                let pts = wave_points(f32::from(bounds.size.width), amp, len);
                let mid = bounds.origin.y + bounds.size.height / 2.0;
                let mut b = PathBuilder::stroke(px(thick));
                for (i, (x, y)) in pts.iter().enumerate() {
                    let pt = point(bounds.origin.x + px(*x), mid + px(*y));
                    if i == 0 {
                        b.move_to(pt)
                    } else {
                        b.line_to(pt)
                    }
                }
                if let Ok(path) = b.build() {
                    window.paint_path(path, color);
                }
            },
        )
        .w_full()
        .h(px(2.0 * amp + thick + 2.0));
        el.style().refine(&self.style);
        el
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_starts_on_the_centre_line_and_spans_the_width() {
        let pts = wave_points(100.0, 2.0, 10.0);
        assert_eq!(pts.first().map(|p| p.0), Some(0.0));
        assert!((pts.last().map(|p| p.0).unwrap_or(0.0) - 100.0).abs() < 1e-3);
        assert!(pts.iter().all(|p| p.1.abs() <= 2.0 + 1e-4));
    }
}

//! The 5x5 dot grid shared by the FlickerSpinner and the status glyphs.
//!
//! Geometry is the original SVG's: a 30x30 view box, dot radius 2, pitch 6, first centre at 3.

use gpui::prelude::*;
use gpui::{
    canvas, point, px, quad, size, transparent_black, App, BorderStyle, Bounds, Hsla, IntoElement,
    Pixels, Window,
};

/// Opacity of a dot that is "off" (`fill-opacity="0.15"`).
pub const DIM: f32 = 0.15;

pub type Dots = [[f32; 5]; 5];

pub fn dots_from_bits(rows: &[u8; 5]) -> Dots {
    let mut d = [[DIM; 5]; 5];
    for (row, bits) in d.iter_mut().zip(rows) {
        for (c, dot) in row.iter_mut().enumerate() {
            if bits >> (4 - c) & 1 == 1 {
                *dot = 1.0;
            }
        }
    }
    d
}

/// Paints `dots` in a square of `side` px. `color` of `None` means the inherited text colour.
pub fn dot_grid(dots: Dots, side: f32, color: Option<Hsla>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let base = color.unwrap_or_else(|| window.text_style().color);
            let k = side / 30.0;
            for (r, row) in dots.iter().enumerate() {
                for (c, opacity) in row.iter().enumerate() {
                    let (cx, cy) = ((3.0 + 6.0 * c as f32) * k, (3.0 + 6.0 * r as f32) * k);
                    let b = Bounds::new(
                        point(
                            bounds.origin.x + px(cx - 2.0 * k),
                            bounds.origin.y + px(cy - 2.0 * k),
                        ),
                        size(px(4.0 * k), px(4.0 * k)),
                    );
                    window.paint_quad(quad(
                        b,
                        px(2.0 * k),
                        base.opacity(*opacity),
                        0.,
                        transparent_black(),
                        BorderStyle::default(),
                    ));
                }
            }
        },
    )
    .size(px(side))
    .flex_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_map_left_to_right() {
        let d = dots_from_bits(&[0b10000, 0, 0, 0, 0b00001]);
        assert_eq!(d[0][0], 1.0);
        assert_eq!(d[0][4], DIM);
        assert_eq!(d[4][4], 1.0);
    }
}

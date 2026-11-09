//! The few Lucide glyphs the components need, painted as stroked paths (GPUI draws no SVG without assets).

use gpui::prelude::*;
use gpui::{canvas, point, px, App, Bounds, Hsla, IntoElement, Pixels, Window};
use gpui::{PathBuilder, Point};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Check,
    ChevronDown,
    ChevronUp,
    ChevronsUpDown,
    Close,
}

/// Polylines in Lucide's 24x24 view box.
fn strokes(glyph: Glyph) -> &'static [&'static [(f32, f32)]] {
    match glyph {
        Glyph::Check => &[&[(5.252, 12.7), (10.2, 18.63), (18.748, 5.37)]],
        Glyph::ChevronDown => &[&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]],
        Glyph::ChevronUp => &[&[(18.0, 15.0), (12.0, 9.0), (6.0, 15.0)]],
        Glyph::ChevronsUpDown => &[
            &[(7.0, 15.0), (12.0, 20.0), (17.0, 15.0)],
            &[(7.0, 9.0), (12.0, 4.0), (17.0, 9.0)],
        ],
        Glyph::Close => &[&[(18.0, 6.0), (6.0, 18.0)], &[(6.0, 6.0), (18.0, 18.0)]],
    }
}

fn paint_polyline(
    window: &mut Window,
    pts: &[Point<Pixels>],
    width: f32,
    color: Hsla,
) -> Result<()> {
    let mut b = PathBuilder::stroke(px(width));
    let mut it = pts.iter();
    if let Some(first) = it.next() {
        b.move_to(*first);
    }
    for p in it {
        b.line_to(*p);
    }
    let path = b.build().map_err(|e| Error::Path(e.to_string()))?;
    window.paint_path(path, color);
    Ok(())
}

/// A `size`px square glyph, stroke 2 (scaled), in `color`.
pub fn icon(glyph: Glyph, size: f32, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let k = size / 24.0;
            for line in strokes(glyph) {
                let pts: Vec<Point<Pixels>> = line
                    .iter()
                    .map(|(x, y)| point(bounds.origin.x + px(x * k), bounds.origin.y + px(y * k)))
                    .collect();
                // A failed tessellation only drops one glyph for a frame; nothing to recover.
                paint_polyline(window, &pts, 2.0 * k, color).ok();
            }
        },
    )
    .size(px(size))
    .flex_none()
}

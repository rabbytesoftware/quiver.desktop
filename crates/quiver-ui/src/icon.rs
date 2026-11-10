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
    Search,
    Home,
    Server,
    Settings,
    Play,
    Terminal,
    ArrowLeft,
    ArrowRight,
    Ellipsis,
    /// The Quiver arrow mark (an up chevron over a dot), as on the tile art.
    Mark,
}

/// Points of a circle in the 24x24 view box.
fn circle(
    cx: f32,
    cy: f32,
    r: f32,
) -> Vec<(f32, f32)> {
    (0..=24)
        .map(|i| {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// A closed 8-tooth cog outline around (12, 12).
fn cog() -> Vec<(f32, f32)> {
    let mut pts = Vec::new();
    for tooth in 0..8 {
        let a = tooth as f32 * std::f32::consts::TAU / 8.0;
        for (da, r) in [(-12.0_f32, 7.4), (-8.0, 9.8), (8.0, 9.8), (12.0, 7.4)] {
            let t = a + da.to_radians();
            pts.push((12.0 + r * t.cos(), 12.0 + r * t.sin()));
        }
    }
    pts.push(pts[0]);
    pts
}

/// Polylines in Lucide's 24x24 view box.
fn strokes(glyph: Glyph) -> Vec<Vec<(f32, f32)>> {
    match glyph {
        Glyph::Check => vec![vec![(5.252, 12.7), (10.2, 18.63), (18.748, 5.37)]],
        Glyph::ChevronDown => vec![vec![(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]],
        Glyph::ChevronUp => vec![vec![(18.0, 15.0), (12.0, 9.0), (6.0, 15.0)]],
        Glyph::ChevronsUpDown => vec![
            vec![(7.0, 15.0), (12.0, 20.0), (17.0, 15.0)],
            vec![(7.0, 9.0), (12.0, 4.0), (17.0, 9.0)],
        ],
        Glyph::Close => vec![
            vec![(18.0, 6.0), (6.0, 18.0)],
            vec![(6.0, 6.0), (18.0, 18.0)],
        ],
        Glyph::Search => vec![circle(10.5, 10.5, 7.0), vec![(15.6, 15.6), (21.0, 21.0)]],
        Glyph::Home => vec![
            vec![(3.0, 11.0), (12.0, 3.0), (21.0, 11.0)],
            vec![
                (5.0, 10.0),
                (5.0, 20.0),
                (10.0, 20.0),
                (10.0, 14.0),
                (14.0, 14.0),
                (14.0, 20.0),
                (19.0, 20.0),
                (19.0, 10.0),
            ],
        ],
        Glyph::Server => vec![
            vec![
                (4.0, 4.0),
                (20.0, 4.0),
                (20.0, 11.0),
                (4.0, 11.0),
                (4.0, 4.0),
            ],
            vec![
                (4.0, 13.0),
                (20.0, 13.0),
                (20.0, 20.0),
                (4.0, 20.0),
                (4.0, 13.0),
            ],
            circle(8.0, 7.5, 0.7),
            circle(8.0, 16.5, 0.7),
        ],
        Glyph::Settings => vec![cog(), circle(12.0, 12.0, 3.0)],
        Glyph::Play => vec![vec![(7.0, 4.5), (19.0, 12.0), (7.0, 19.5), (7.0, 4.5)]],
        Glyph::Terminal => vec![
            vec![(4.0, 17.0), (10.0, 11.0), (4.0, 5.0)],
            vec![(12.0, 19.0), (20.0, 19.0)],
        ],
        Glyph::ArrowLeft => vec![
            vec![(11.0, 6.0), (5.0, 12.0), (11.0, 18.0)],
            vec![(5.0, 12.0), (19.0, 12.0)],
        ],
        Glyph::ArrowRight => vec![
            vec![(13.0, 6.0), (19.0, 12.0), (13.0, 18.0)],
            vec![(19.0, 12.0), (5.0, 12.0)],
        ],
        Glyph::Ellipsis => vec![
            circle(5.0, 12.0, 0.7),
            circle(12.0, 12.0, 0.7),
            circle(19.0, 12.0, 0.7),
        ],
        Glyph::Mark => vec![
            vec![(5.5, 15.2), (12.0, 4.5), (18.5, 15.2)],
            circle(12.0, 19.6, 0.7),
        ],
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
pub fn icon(
    glyph: Glyph,
    size: f32,
    color: Hsla,
) -> impl IntoElement {
    icon_with_stroke(glyph, size, color, 2.0)
}

/// [`icon`] with an explicit stroke width in the 24px view box.
pub fn icon_with_stroke(
    glyph: Glyph,
    size: f32,
    color: Hsla,
    stroke: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let k = size / 24.0;
            for line in &strokes(glyph) {
                let pts: Vec<Point<Pixels>> = line
                    .iter()
                    .map(|(x, y)| point(bounds.origin.x + px(x * k), bounds.origin.y + px(y * k)))
                    .collect();
                // A failed tessellation only drops one glyph for a frame; nothing to recover.
                paint_polyline(window, &pts, stroke * k, color).ok();
            }
        },
    )
    .size(px(size))
    .flex_none()
}

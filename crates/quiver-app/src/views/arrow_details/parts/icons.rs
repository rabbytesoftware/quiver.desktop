//! The few Lucide pictures this page draws that `quiver_ui::Glyph` does not carry, as stroked
//! polylines in the 24x24 view box (GPUI draws no SVG without assets).

use gpui::prelude::*;
use gpui::{canvas, point, px, App, Bounds, Hsla, IntoElement, PathBuilder, Pixels, Point, Window};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pict {
    Stop,
    Restart,
    Info,
    Methods,
    Download,
    Upload,
    Trash,
    Plus,
    External,
    Eye,
    EyeOff,
    Cube,
    Alert,
}

fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    (0..=24)
        .map(|i| {
            let a = (from + (to - from) * i as f32 / 24.0).to_radians();
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)]
}

fn strokes(pict: Pict) -> Vec<Vec<(f32, f32)>> {
    match pict {
        Pict::Stop => vec![rect(6., 6., 12., 12.)],
        Pict::Restart => vec![
            arc(12., 12., 8., 20., 330.),
            vec![(20., 4.), (20., 11.), (13., 11.)],
        ],
        Pict::Info => vec![
            arc(12., 12., 9., 0., 360.),
            vec![(12., 11.), (12., 16.)],
            vec![(12., 7.6), (12., 7.7)],
        ],
        Pict::Methods => vec![
            vec![(8., 6.), (20., 6.)],
            vec![(8., 12.), (20., 12.)],
            vec![(8., 18.), (20., 18.)],
            vec![(4., 6.), (4.01, 6.)],
            vec![(4., 12.), (4.01, 12.)],
            vec![(4., 18.), (4.01, 18.)],
        ],
        Pict::Download => vec![
            vec![(12., 4.), (12., 15.)],
            vec![(7., 10.), (12., 15.), (17., 10.)],
            vec![(5., 20.), (19., 20.)],
        ],
        Pict::Upload => vec![
            vec![(12., 20.), (12., 9.)],
            vec![(7., 14.), (12., 9.), (17., 14.)],
            vec![(5., 4.), (19., 4.)],
        ],
        Pict::Trash => vec![
            vec![(4., 7.), (20., 7.)],
            vec![(9., 7.), (9., 4.), (15., 4.), (15., 7.)],
            vec![(6., 7.), (7., 20.), (17., 20.), (18., 7.)],
        ],
        Pict::Plus => vec![vec![(12., 5.), (12., 19.)], vec![(5., 12.), (19., 12.)]],
        Pict::External => vec![
            vec![(14., 4.), (20., 4.), (20., 10.)],
            vec![(20., 4.), (11., 13.)],
            vec![(18., 14.), (18., 20.), (4., 20.), (4., 6.), (10., 6.)],
        ],
        Pict::Eye => vec![
            vec![
                (2., 12.),
                (6., 7.),
                (12., 5.),
                (18., 7.),
                (22., 12.),
                (18., 17.),
                (12., 19.),
                (6., 17.),
                (2., 12.),
            ],
            arc(12., 12., 3., 0., 360.),
        ],
        Pict::EyeOff => vec![
            vec![
                (2., 12.),
                (6., 7.),
                (12., 5.),
                (18., 7.),
                (22., 12.),
                (18., 17.),
                (12., 19.),
                (6., 17.),
                (2., 12.),
            ],
            vec![(4., 4.), (20., 20.)],
        ],
        Pict::Cube => vec![rect(6., 6., 12., 12.)],
        Pict::Alert => vec![
            vec![(12., 3.5), (22., 20.5), (2., 20.5), (12., 3.5)],
            vec![(12., 10.), (12., 14.5)],
            vec![(12., 17.5), (12., 17.6)],
        ],
    }
}

/// A `size`px square picture in `color`; stroke 1.8 of 24 unless the caller passes its own.
pub fn pict(pict: Pict, size: f32, color: Hsla) -> impl IntoElement {
    pict_with_stroke(pict, size, color, 1.8)
}

pub fn pict_with_stroke(pict: Pict, size: f32, color: Hsla, stroke: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let k = size / 24.0;
            for line in strokes(pict) {
                let pts: Vec<Point<Pixels>> = line
                    .iter()
                    .map(|(x, y)| point(bounds.origin.x + px(x * k), bounds.origin.y + px(y * k)))
                    .collect();
                let mut b = PathBuilder::stroke(px(stroke * k));
                let mut it = pts.iter();
                if let Some(first) = it.next() {
                    b.move_to(*first);
                }
                for p in it {
                    b.line_to(*p);
                }
                // A failed tessellation only drops one stroke for a frame.
                if let Ok(path) = b.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .size(px(size))
    .flex_none()
}

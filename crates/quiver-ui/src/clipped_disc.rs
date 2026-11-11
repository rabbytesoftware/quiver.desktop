//! Shapes drawn inside a rounded box and cut by its corners.
//!
//! GPUI's `overflow_hidden` clips to the box's rectangle, not to its rounded corners, so a child that reaches a
//! corner (the banner art's disc, a progress fill) pokes out of it. These paint the shape already cut: both
//! shapes are convex, so the shape's polygon is clipped against the rounded box's polygon (Sutherland–Hodgman)
//! and the convex result is painted as one path.

use gpui::{canvas, point, px, Bounds, Hsla, IntoElement, Path, Pixels, Point, Styled as _};

const DISC_STEPS: usize = 96;
const CORNER_STEPS: usize = 12;

type P = (f32, f32);

/// The rounded box `w`×`h` (corner radius `r`) as a convex polygon, clockwise in screen space.
fn rounded_box(
    w: f32,
    h: f32,
    r: f32,
) -> Vec<P> {
    let r = r.min(w / 2.).min(h / 2.);
    // Corner centres with the angle each quarter starts at (screen space: y grows downwards).
    let corners = [
        (w - r, r, -90f32),
        (w - r, h - r, 0f32),
        (r, h - r, 90f32),
        (r, r, 180f32),
    ];
    let mut out = Vec::with_capacity(4 * (CORNER_STEPS + 1));
    for (cx, cy, start) in corners {
        for i in 0..=CORNER_STEPS {
            let a = (start + 90. * i as f32 / CORNER_STEPS as f32).to_radians();
            out.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    out
}

fn disc(
    cx: f32,
    cy: f32,
    radius: f32,
) -> Vec<P> {
    (0..DISC_STEPS)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / DISC_STEPS as f32;
            (cx + radius * a.cos(), cy + radius * a.sin())
        })
        .collect()
}

/// Sutherland–Hodgman: `subject` clipped by the convex, clockwise (screen space) `clip`.
fn clip_convex(
    subject: Vec<P>,
    clip: &[P],
) -> Vec<P> {
    let inside = |a: P, b: P, p: P| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0) >= 0.;
    let cross = |a: P, b: P, p: P, q: P| {
        let (x1, y1, x2, y2) = (a.0, a.1, b.0, b.1);
        let (x3, y3, x4, y4) = (p.0, p.1, q.0, q.1);
        let d = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
        if d.abs() < f32::EPSILON {
            return p;
        }
        let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / d;
        (x1 + t * (x2 - x1), y1 + t * (y2 - y1))
    };
    let mut out = subject;
    for i in 0..clip.len() {
        if out.is_empty() {
            break;
        }
        let (a, b) = (clip[i], clip[(i + 1) % clip.len()]);
        let input = std::mem::take(&mut out);
        for j in 0..input.len() {
            let (p, q) = (input[j], input[(j + 1) % input.len()]);
            match (inside(a, b, p), inside(a, b, q)) {
                (true, true) => out.push(q),
                (true, false) => out.push(cross(a, b, p, q)),
                (false, true) => {
                    out.push(cross(a, b, p, q));
                    out.push(q);
                }
                (false, false) => {}
            }
        }
    }
    out
}

/// The disc's outline inside a `w`×`h` box with corner radius `corner`, in the box's coordinates.
pub fn disc_in_box(
    (cx, cy, radius): (f32, f32, f32),
    (w, h, corner): (f32, f32, f32),
) -> Vec<(f32, f32)> {
    clip_convex(disc(cx, cy, radius), &rounded_box(w, h, corner))
}

/// The rectangle (`x`, `y`, `w`, `h`) inside a `w`×`h` box with corner radius `corner`, in the box's coordinates.
pub fn rect_in_box(
    (x, y, w, h): (f32, f32, f32, f32),
    parent: (f32, f32, f32),
) -> Vec<(f32, f32)> {
    if w <= 0. || h <= 0. {
        return Vec::new();
    }
    let (bw, bh, corner) = parent;
    let rect = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    clip_convex(rect, &rounded_box(bw, bh, corner))
}

/// An absolutely placed layer that fills its rounded parent (`w`×`h`, radius `corner`) and paints the disc
/// centred at (`cx`, `cy`) with `radius`, cut by the parent's corners.
pub fn clipped_disc(
    centre_and_radius: (f32, f32, f32),
    parent: (f32, f32, f32),
    color: Hsla,
) -> impl IntoElement {
    painted(disc_in_box(centre_and_radius, parent), color)
}

/// An absolutely placed layer that fills its rounded parent (`w`×`h`, radius `corner`) and paints the rectangle
/// (`x`, `y`, `w`, `h`) cut by the parent's corners: a progress fill that keeps the track's rounded ends.
pub fn clipped_rect(
    rect: (f32, f32, f32, f32),
    parent: (f32, f32, f32),
    color: Hsla,
) -> impl IntoElement {
    painted(rect_in_box(rect, parent), color)
}

fn painted(
    outline: Vec<P>,
    color: Hsla,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            let Some(&first) = outline.first() else {
                return;
            };
            let at = |(x, y): P| -> Point<Pixels> {
                point(bounds.origin.x + px(x), bounds.origin.y + px(y))
            };
            let mut path = Path::new(at(first));
            for &p in &outline[1..] {
                path.line_to(at(p));
            }
            window.paint_path(path, color);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::{disc_in_box, rect_in_box};

    #[test]
    fn the_disc_stays_inside_the_rounded_corner() {
        // The tile banner's disc reaching past the top-right corner of a 214×107 box with 12px corners.
        let w = 214.;
        let outline = disc_in_box((w * 0.8, w * 0.1, w * 0.2167), (w, w / 2., 12.));
        assert!(!outline.is_empty());
        let (ccx, ccy) = (w - 12., 12.);
        for (x, y) in outline {
            assert!((0.0..=w + 0.01).contains(&x) && (-0.01..=w / 2. + 0.01).contains(&y));
            // In the corner square, every point is within the arc.
            if x > ccx && y < ccy {
                let d = ((x - ccx).powi(2) + (y - ccy).powi(2)).sqrt();
                assert!(
                    d <= 12. + 0.05,
                    "({x}, {y}) is {d} from the corner's centre"
                );
            }
        }
    }

    #[test]
    fn a_fill_keeps_the_track_s_rounded_left_end() {
        // A 40% fill of a 240×40 track with 10px corners: no point of it lies outside the top-left arc.
        let outline = rect_in_box((0., 0., 96., 40.), (240., 40., 10.));
        assert!(!outline.is_empty());
        for (x, y) in outline {
            assert!((0.0..=96.01).contains(&x));
            if x < 10. && y < 10. {
                let d = ((x - 10.).powi(2) + (y - 10.).powi(2)).sqrt();
                assert!(d <= 10.05, "({x}, {y}) is {d} from the corner's centre");
            }
        }
        assert!(rect_in_box((0., 0., 0., 40.), (240., 40., 10.)).is_empty());
    }

    #[test]
    fn a_disc_outside_the_box_paints_nothing() {
        assert!(disc_in_box((500., 500., 10.), (100., 50., 12.)).is_empty());
    }
}

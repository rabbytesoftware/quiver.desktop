//! The tiles' hover lift, as the old app drew it (`features/search/styles/card.css`, spec 8.4-8.6): on hover
//! the art rises 30px in 40ms and uncovers a strip under it; on leave it falls back in 300ms with a small
//! rebound kick. Both curves are the CSS `linear()` stops measured off the reference recording; the strip
//! fades in over 40ms and out over 90ms (gone before the rebound starts, ~117ms).
//!
//! GPUI has no CSS transitions, so the tile keeps a [`Lift`] per element (`use_keyed_state`), sets it from
//! `on_hover`, and asks for the next frame while it is moving. An interrupted move starts from where the art
//! is, as a CSS transition does.

use std::time::{Duration, Instant};

use gpui::SharedString;

/// The lift distance and the strip's height at once, so the art never uncovers more or less than the strip.
pub const REVEAL: f32 = 30.;

const RISE: Duration = Duration::from_millis(40);
const FALL: Duration = Duration::from_millis(300);
const SHOW: Duration = Duration::from_millis(40);
const HIDE: Duration = Duration::from_millis(90);

/// `linear(0, 0.35 25%, 0.7 50%, 0.92 75%, 1)`.
const RISE_CURVE: &[(f32, f32)] = &[(0., 0.), (0.25, 0.35), (0.5, 0.7), (0.75, 0.92), (1., 1.)];

/// The fall with its rebound: nearly home at 36.7%, kicked back to 0.833 at 58.3%, settled at 100%.
const FALL_CURVE: &[(f32, f32)] = &[
    (0., 0.),
    (0.083, 0.15),
    (0.167, 0.32),
    (0.25, 0.53),
    (0.317, 0.76),
    (0.367, 0.96),
    (0.433, 0.9),
    (0.5, 0.855),
    (0.583, 0.833),
    (0.683, 0.86),
    (0.783, 0.91),
    (0.883, 0.955),
    (0.96, 0.988),
    (1., 1.),
];

/// CSS `linear()`: straight segments between the stops.
fn along(
    curve: &[(f32, f32)],
    t: f32,
) -> f32 {
    let t = t.clamp(0., 1.);
    for pair in curve.windows(2) {
        let ((t0, v0), (t1, v1)) = (pair[0], pair[1]);
        if t <= t1 {
            return v0 + (v1 - v0) * ((t - t0) / (t1 - t0));
        }
    }
    1.
}

fn fraction(
    elapsed: Duration,
    total: Duration,
) -> f32 {
    (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.)
}

/// Where a tile's art is: hovered or not, and since when.
#[derive(Clone, Copy, Debug, Default)]
pub struct Lift {
    hovered: bool,
    since: Option<Instant>,
    /// Lift and strip opacity when the last change started.
    from: (f32, f32),
}

/// One frame of the lift: how far up the art is (0..=[`REVEAL`]), the strip's opacity, and whether it is
/// still moving (the tile asks for another frame while it is).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiftFrame {
    pub lift: f32,
    pub strip: f32,
    pub moving: bool,
}

impl Lift {
    pub fn set_hovered(
        &mut self,
        hovered: bool,
        now: Instant,
    ) {
        if hovered == self.hovered {
            return;
        }
        let at = self.at(now);
        self.from = (at.lift, at.strip);
        self.hovered = hovered;
        self.since = Some(now);
    }

    pub fn at(
        &self,
        now: Instant,
    ) -> LiftFrame {
        let Some(since) = self.since else {
            return LiftFrame {
                lift: 0.,
                strip: 0.,
                moving: false,
            };
        };
        let elapsed = now.saturating_duration_since(since);
        let (move_for, curve, fade_for, to) = if self.hovered {
            (RISE, RISE_CURVE, SHOW, (REVEAL, 1.))
        } else {
            (FALL, FALL_CURVE, HIDE, (0., 0.))
        };
        let (moved, faded) = (fraction(elapsed, move_for), fraction(elapsed, fade_for));
        LiftFrame {
            lift: self.from.0 + (to.0 - self.from.0) * along(curve, moved),
            strip: self.from.1 + (to.1 - self.from.1) * faded,
            moving: moved < 1. || faded < 1.,
        }
    }
}

/// What the strip under a lifted tile says: a mono line, then (search results) the OS families with a build
/// and the stars.
#[derive(Clone, Debug, Default)]
pub struct TileReveal {
    pub text: SharedString,
    /// Linux, macOS, Windows.
    pub os: Option<[bool; 3]>,
    pub stars: Option<SharedString>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(
        lift: &Lift,
        start: Instant,
        ms: u64,
    ) -> LiftFrame {
        lift.at(start + Duration::from_millis(ms))
    }

    #[test]
    fn at_rest_nothing_moves() {
        let frame = Lift::default().at(Instant::now());
        assert_eq!((frame.lift, frame.strip, frame.moving), (0., 0., false));
    }

    #[test]
    fn hover_lifts_the_art_30px_in_40ms_and_shows_the_strip() {
        let start = Instant::now();
        let mut lift = Lift::default();
        lift.set_hovered(true, start);
        assert!((after(&lift, start, 10).lift - REVEAL * 0.35).abs() < 0.01);
        let done = after(&lift, start, 40);
        assert_eq!((done.lift, done.strip, done.moving), (REVEAL, 1., false));
    }

    #[test]
    fn leaving_falls_back_with_a_rebound_and_hides_the_strip_first() {
        let start = Instant::now();
        let mut lift = Lift::default();
        lift.set_hovered(true, start);
        let left = start + Duration::from_millis(100);
        lift.set_hovered(false, left);
        // Nearly home at 36.7% of 300ms, kicked back up at 58.3%, home at 300ms.
        let near = after(&lift, left, 110);
        let kicked = after(&lift, left, 175);
        assert!(near.lift < kicked.lift, "{near:?} then {kicked:?}");
        assert_eq!(after(&lift, left, 90).strip, 0.);
        let home = after(&lift, left, 300);
        assert_eq!((home.lift, home.moving), (0., false));
    }

    #[test]
    fn an_interrupted_fall_rises_from_where_the_art_is() {
        let start = Instant::now();
        let mut lift = Lift::default();
        lift.set_hovered(true, start);
        let left = start + Duration::from_millis(40);
        lift.set_hovered(false, left);
        let back = left + Duration::from_millis(50);
        let midway = lift.at(back).lift;
        lift.set_hovered(true, back);
        assert!((lift.at(back).lift - midway).abs() < 0.01);
    }
}

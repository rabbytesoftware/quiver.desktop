//! Rail side and width. Bounded at the nav collapse point and the design ceiling.

use super::geometry::SidebarSide;

pub const SIDEBAR_MIN: f64 = 160.0;
pub const SIDEBAR_MAX: f64 = 320.0;
pub const SIDEBAR_DEFAULT: f64 = 246.0;
pub const SHELL_STORAGE_KEY: &str = "quiver.shell";

/// Anything but "right" is the left edge.
pub fn normalise_side(value: Option<&str>) -> SidebarSide {
    if value == Some("right") {
        SidebarSide::Right
    } else {
        SidebarSide::Left
    }
}

/// A missing or non-finite width is the default; anything else is clamped into the bounds.
pub fn normalise_width(value: Option<f64>) -> f64 {
    match value {
        Some(w) if w.is_finite() => w.clamp(SIDEBAR_MIN, SIDEBAR_MAX),
        _ => SIDEBAR_DEFAULT,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShellSettings {
    pub sidebar_side: SidebarSide,
    pub sidebar_width: f64,
}

impl Default for ShellSettings {
    fn default() -> Self {
        Self {
            sidebar_side: SidebarSide::Left,
            sidebar_width: SIDEBAR_DEFAULT,
        }
    }
}

impl ShellSettings {
    /// Reads the two persisted values back, normalising whatever was on disk.
    pub fn from_stored(side: Option<&str>, width: Option<f64>) -> Self {
        Self {
            sidebar_side: normalise_side(side),
            sidebar_width: normalise_width(width),
        }
    }

    pub fn set_side(&mut self, side: SidebarSide) {
        self.sidebar_side = side;
    }

    /// A drag past either end clamps to the end.
    pub fn set_width(&mut self, width: f64) {
        self.sidebar_width = normalise_width(Some(width));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docks_left_at_the_design_width_and_bounds_match_the_nav_collapse() {
        let s = ShellSettings::default();
        assert_eq!(
            (s.sidebar_side, s.sidebar_width),
            (SidebarSide::Left, 246.0)
        );
        assert_eq!((SIDEBAR_MIN, SIDEBAR_MAX), (160.0, 320.0));
    }

    #[test]
    fn width_clamps_a_drag_from_both_ends() {
        let mut s = ShellSettings::default();
        s.set_width(200.0);
        assert_eq!(s.sidebar_width, 200.0);
        s.set_width(10.0);
        assert_eq!(s.sidebar_width, SIDEBAR_MIN);
        s.set_width(9999.0);
        assert_eq!(s.sidebar_width, SIDEBAR_MAX);
    }

    #[test]
    fn side_moves_the_rail() {
        let mut s = ShellSettings::default();
        s.set_side(SidebarSide::Right);
        assert_eq!(s.sidebar_side.as_stored(), "right");
    }

    #[test]
    fn a_stored_choice_reads_back_and_bad_values_are_repaired() {
        assert_eq!(
            ShellSettings::from_stored(Some("right"), Some(200.0)).sidebar_side,
            SidebarSide::Right
        );
        assert_eq!(
            ShellSettings::from_stored(Some("up"), None),
            ShellSettings::default()
        );
        assert_eq!(
            ShellSettings::from_stored(None, Some(f64::NAN)).sidebar_width,
            SIDEBAR_DEFAULT
        );
        assert_eq!(
            ShellSettings::from_stored(None, Some(50.0)).sidebar_width,
            SIDEBAR_MIN
        );
        assert_eq!(
            ShellSettings::from_stored(None, Some(900.0)).sidebar_width,
            SIDEBAR_MAX
        );
    }
}

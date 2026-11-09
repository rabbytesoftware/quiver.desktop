use crate::shell::{normalise_width, SidebarSide};

pub const NUDGE: f64 = 8.0;

/// The width a drag of `dx` pixels from `from` lands on. Dragging towards the content grows the rail
/// whichever edge it sits on, so the right-hand rail inverts the delta.
pub fn width_after(side: SidebarSide, dx: f64, from: f64) -> f64 {
    let signed = if side == SidebarSide::Left { dx } else { -dx };
    normalise_width(Some(from + signed))
}

/// The pixel delta of an arrow key on the separator; the side inversion happens in [`width_after`].
pub fn nudge_for_key(key: &str) -> Option<f64> {
    match key {
        "ArrowLeft" => Some(-NUDGE),
        "ArrowRight" => Some(NUDGE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{SIDEBAR_MAX, SIDEBAR_MIN};

    #[test]
    fn a_left_rail_grows_with_the_pointer_and_a_right_rail_against_it() {
        assert_eq!(width_after(SidebarSide::Left, 20.0, 240.0), 260.0);
        assert_eq!(width_after(SidebarSide::Right, 20.0, 240.0), 220.0);
    }

    #[test]
    fn the_width_is_clamped_to_the_bounds() {
        assert_eq!(width_after(SidebarSide::Left, 500.0, 240.0), SIDEBAR_MAX);
        assert_eq!(width_after(SidebarSide::Left, -500.0, 240.0), SIDEBAR_MIN);
    }

    #[test]
    fn arrow_keys_nudge_by_eight_and_other_keys_do_nothing() {
        assert_eq!(nudge_for_key("ArrowLeft"), Some(-8.0));
        assert_eq!(nudge_for_key("ArrowRight"), Some(8.0));
        assert_eq!(nudge_for_key("Enter"), None);
    }
}

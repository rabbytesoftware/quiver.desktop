//! How much of the Popular shelf Home shows: 3 first, 6 more per "Load more" press, never past 12 (the
//! rest is behind "View all", which leads to the Recommended page).

/// Tiles Home shows before any press.
pub const POPULAR_FIRST: usize = 3;
/// Tiles each press of "Load more" adds.
pub const POPULAR_STEP: usize = 6;
/// Home never shows more than this.
pub const POPULAR_CAP: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PopularSlice {
    pub shown: usize,
    pub can_load_more: bool,
}

/// `available` is how many arrows match the chip; `presses` how many times Load more was pressed.
pub fn popular_slice(available: usize, presses: usize) -> PopularSlice {
    let allowed = (POPULAR_FIRST + POPULAR_STEP.saturating_mul(presses)).min(POPULAR_CAP);
    let shown = available.min(allowed);
    PopularSlice {
        shown,
        can_load_more: shown < available && shown < POPULAR_CAP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_first_then_six_per_press_capped_at_twelve() {
        let shown: Vec<_> = (0..4).map(|p| popular_slice(40, p).shown).collect();
        assert_eq!(shown, [3, 9, 12, 12]);
    }

    #[test]
    fn load_more_disappears_at_the_cap_or_when_everything_shows() {
        assert!(popular_slice(40, 1).can_load_more);
        assert!(!popular_slice(40, 2).can_load_more, "cap reached");
        assert!(!popular_slice(3, 0).can_load_more);
        assert!(!popular_slice(5, 1).can_load_more, "all 5 shown");
        assert!(popular_slice(4, 0).can_load_more);
    }

    #[test]
    fn a_short_shelf_shows_what_it_has() {
        assert_eq!(popular_slice(0, 0).shown, 0);
        assert_eq!(popular_slice(2, 5).shown, 2);
    }
}

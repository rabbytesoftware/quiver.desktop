//! Column count follows the size of the answer, so a thin answer spends the frame on the art (spec 9.4).

/// Never below this, whatever the answer size or the window. Spec 9.4.
pub const MIN_TILE: u32 = 190;
pub const COLUMN_GAP: u32 = 12;

/// Expressed as a cap rather than a per-answer tile width: picking a floor and letting `auto-fill`
/// divide made the outcome depend on the exact grid width (the division lands either side of a whole
/// number, so one floor gave four columns at one window size and two huge tiles at another). This says
/// what is meant: at most `cap` columns, and never a tile below `MIN_TILE`.
pub fn column_cap(total: usize) -> u32 {
    match total {
        0..=3 => 2,
        4..=8 => 4,
        _ => 5,
    }
}

/// The CSS `grid-template-columns` rule (kept as text: it is the one place the cap and the minimum meet).
pub fn column_rule(total: usize) -> String {
    let cap = column_cap(total);
    let gaps = (cap - 1) * COLUMN_GAP;
    format!("repeat(auto-fill, minmax(max({MIN_TILE}px, calc((100% - {gaps}px) / {cap})), 1fr))")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thin_answer_spends_the_frame_on_the_art() {
        assert_eq!(column_cap(1), 2);
        assert_eq!(column_cap(3), 2);
    }

    #[test]
    fn the_cap_opens_up_as_the_answer_grows() {
        assert_eq!(column_cap(4), 4);
        assert_eq!(column_cap(8), 4);
        assert_eq!(column_cap(9), 5);
        assert_eq!(column_cap(100), 5);
    }

    #[test]
    fn the_rule_caps_the_count_rather_than_fixing_a_width() {
        assert_eq!(
            column_rule(12),
            "repeat(auto-fill, minmax(max(190px, calc((100% - 48px) / 5)), 1fr))"
        );
        assert_eq!(
            column_rule(2),
            "repeat(auto-fill, minmax(max(190px, calc((100% - 12px) / 2)), 1fr))"
        );
    }

    #[test]
    fn a_tile_never_goes_below_the_minimum_and_the_gaps_are_exact() {
        for total in [1, 4, 9, 40] {
            assert!(column_rule(total).contains("max(190px,"));
        }
        assert!(column_rule(6).contains("100% - 36px"));
    }
}

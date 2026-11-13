//! The one number a native grid needs from [`column_rule`](super::column_rule): the width of a tile.
//! `repeat(auto-fill, minmax(max(MIN_TILE, (100% - gaps) / cap), 1fr))` fills a row with as many
//! columns as fit at that minimum, then stretches them; this does the same sum for a known width.

use super::columns::{column_cap, COLUMN_GAP, MIN_TILE};

/// Width of each tile when the answer holds `total` arrows and the grid is `available` px wide.
pub fn tile_width(
    total: usize,
    available: f32,
) -> f32 {
    let (min, gap) = (MIN_TILE as f32, COLUMN_GAP as f32);
    let cap = column_cap(total) as f32;
    let wanted = ((available - (cap - 1.) * gap) / cap).max(min);
    let fit = ((available + gap) / (wanted + gap)).floor().max(1.);
    ((available - (fit - 1.) * gap) / fit).max(min)
}

/// Width of each of `columns` equal tiles that fill an `available` px row (the old
/// `repeat(columns, minmax(0, 1fr))`: a fixed column count, no minimum).
pub fn equal_columns_width(
    columns: usize,
    available: f32,
) -> f32 {
    let columns = columns.max(1) as f32;
    ((available - (columns - 1.) * COLUMN_GAP as f32) / columns).max(0.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_columns_fill_the_row_whatever_its_width() {
        assert_eq!(equal_columns_width(3, 890.), (890. - 2. * 12.) / 3.);
        assert_eq!(equal_columns_width(3, 1590.), 522.);
    }

    #[test]
    fn equal_columns_never_go_negative_in_a_collapsed_window() {
        assert_eq!(equal_columns_width(3, 10.), 0.);
        assert_eq!(
            equal_columns_width(0, 100.),
            100.,
            "zero columns counts as one"
        );
    }

    #[test]
    fn a_thin_answer_gets_two_wide_columns() {
        assert_eq!(tile_width(2, 900.), 444.);
    }

    #[test]
    fn a_big_answer_gets_up_to_five_columns_that_fill_the_row() {
        assert_eq!(tile_width(40, 1100.), (1100. - 4. * 12.) / 5.);
        assert_eq!(
            tile_width(40, 900.),
            (900. - 3. * 12.) / 4.,
            "190 px min leaves room for 4"
        );
    }

    #[test]
    fn never_below_the_minimum_even_in_a_narrow_window() {
        assert_eq!(tile_width(40, 150.), 190.);
        assert!(tile_width(40, 500.) >= 190.);
    }

    #[test]
    fn columns_that_do_not_fit_collapse_into_fewer_wider_ones() {
        let w = tile_width(40, 700.);
        assert_eq!(w, (700. - 2. * 12.) / 3.);
    }
}

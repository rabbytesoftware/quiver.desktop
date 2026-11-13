//! Applying a caller's style on top of a component's own. GPUI's `StyleRefinement::refine` copies the `text`
//! field wholesale instead of merging it, so a caller's `.text_size(..)` (or any other text setter) used to
//! erase the colour, weight and family the component had set for itself: a filled button in the dark theme
//! lost its dark label and showed the page's white one. Every component applies its caller's style through
//! [`apply`].

use gpui::{Refineable, StyleRefinement};

/// `target.refine(caller)`, with the two text styles merged field by field.
pub(crate) fn apply(
    target: &mut StyleRefinement,
    caller: &StyleRefinement,
) {
    let own_text = target.text.clone();
    target.refine(caller);
    if let (Some(mut text), Some(over)) = (own_text, caller.text.as_ref()) {
        text.refine(over);
        target.text = Some(text);
    }
}

#[cfg(test)]
mod tests {
    use gpui::prelude::*;
    use gpui::{black, div, px, white};

    use super::apply;

    #[test]
    fn a_callers_text_size_keeps_the_components_text_colour() {
        let mut own = div()
            .text_color(black())
            .font_weight(gpui::FontWeight::MEDIUM);
        let mut caller = div().text_size(px(15.));
        let caller = caller.style().clone();
        apply(own.style(), &caller);
        let text = own.style().text.clone().expect("text style");
        assert_eq!(text.color, Some(black()));
        assert_eq!(text.font_weight, Some(gpui::FontWeight::MEDIUM));
        assert_eq!(text.font_size, Some(px(15.).into()));
    }

    #[test]
    fn a_callers_colour_wins_over_the_components() {
        let mut own = div().text_color(black());
        let mut caller = div().text_color(white());
        let caller = caller.style().clone();
        apply(own.style(), &caller);
        assert_eq!(own.style().text.clone().unwrap().color, Some(white()));
    }
}

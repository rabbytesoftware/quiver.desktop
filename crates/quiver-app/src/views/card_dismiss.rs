//! Dismissing a hand-drawn popover card (the Steps and ⓘ cards of arrow details, the Follow ⓘ card of a
//! collection). Menu and Select close on an outside press and on Escape; these cards were toggled only by their
//! own button, so they stayed up until that button was pressed again.
//!
//! A press on the card's own trigger counts as "outside" the card, so it closes the card before the trigger's
//! click runs; without care the click would open it again. [`CardDismiss`] remembers which card was open when
//! the press began (a capture-phase listener at the page root) and [`CardDismiss::next`] decides the new state.

use gpui::FocusHandle;

pub struct CardDismiss<K> {
    /// Takes the keyboard while a card is open, so Escape reaches the page root.
    pub focus: FocusHandle,
    open_at_press: Option<K>,
}

impl<K: Copy + PartialEq> CardDismiss<K> {
    pub fn new(focus: FocusHandle) -> Self {
        Self {
            focus,
            open_at_press: None,
        }
    }

    /// A mouse press began anywhere in the window while `open` was showing.
    pub fn press(
        &mut self,
        open: Option<K>,
    ) {
        self.open_at_press = open;
    }

    /// The trigger of `card` was clicked while `open` is showing: which card shows next.
    pub fn next(
        &mut self,
        card: K,
        open: Option<K>,
    ) -> Option<K> {
        let at_press = self.open_at_press.take();
        if at_press == Some(card) {
            // The press that began this click already closed it from outside: leave it closed.
            return None;
        }
        if open == Some(card) {
            None
        } else {
            Some(card)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Card {
        Steps,
        Info,
    }

    fn dismiss(cx: &mut gpui::TestAppContext) -> CardDismiss<Card> {
        CardDismiss::new(cx.update(|cx| cx.focus_handle()))
    }

    #[gpui::test]
    fn a_click_on_the_trigger_of_a_closed_card_opens_it(cx: &mut gpui::TestAppContext) {
        let mut d = dismiss(cx);
        d.press(None);
        assert_eq!(d.next(Card::Steps, None), Some(Card::Steps));
    }

    /// The outside press on the trigger closed the card; the click that follows must not reopen it.
    #[gpui::test]
    fn pressing_the_trigger_of_an_open_card_closes_it_and_the_click_does_not_reopen(
        cx: &mut gpui::TestAppContext
    ) {
        let mut d = dismiss(cx);
        d.press(Some(Card::Steps));
        // By the time the click runs, the card's outside-press handler has already closed it.
        assert_eq!(d.next(Card::Steps, None), None);
    }

    #[gpui::test]
    fn the_other_trigger_opens_its_own_card_while_one_is_up(cx: &mut gpui::TestAppContext) {
        let mut d = dismiss(cx);
        d.press(Some(Card::Steps));
        assert_eq!(d.next(Card::Info, None), Some(Card::Info));
    }

    /// A keyboard activation has no press: it toggles on the current state.
    #[gpui::test]
    fn a_keyboard_click_toggles_on_the_current_state(cx: &mut gpui::TestAppContext) {
        let mut d = dismiss(cx);
        assert_eq!(d.next(Card::Info, Some(Card::Info)), None);
        assert_eq!(d.next(Card::Info, None), Some(Card::Info));
    }

    /// One press only decides one click: a stale press must not swallow a later keyboard activation.
    #[gpui::test]
    fn a_press_decides_one_click_only(cx: &mut gpui::TestAppContext) {
        let mut d = dismiss(cx);
        d.press(Some(Card::Steps));
        assert_eq!(d.next(Card::Steps, None), None);
        assert_eq!(d.next(Card::Steps, None), Some(Card::Steps));
    }
}

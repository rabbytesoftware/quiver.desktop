/// A six-digit pairing code, one character (or empty) per slot -- kept as a fixed-length array rather than
/// a plain string so a digit entered out of order (paste into slot 0, then fill slot 3) never collapses
/// the gap between it and its neighbours.
pub const PAIRING_CODE_LENGTH: usize = 6;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PairingCode {
    slots: [Option<char>; PAIRING_CODE_LENGTH],
}

impl PairingCode {
    pub fn slot(
        &self,
        index: usize,
    ) -> Option<char> {
        self.slots.get(index).copied().flatten()
    }

    /// `raw` is a controlled input's whole field value, so it may briefly hold more than one character
    /// (typing over an existing digit) -- only the last digit typed is kept, matching a native
    /// single-digit field. Returns the slot to focus next, if any.
    pub fn set_digit(
        &mut self,
        index: usize,
        raw: &str,
    ) -> Option<usize> {
        let digit = raw.chars().rfind(char::is_ascii_digit);
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = digit;
        }
        (digit.is_some() && index + 1 < PAIRING_CODE_LENGTH).then_some(index + 1)
    }

    pub fn clear_digit(
        &mut self,
        index: usize,
    ) {
        self.set_digit(index, "");
    }

    /// Spreads a paste over the slots from the first; returns the slot to focus afterwards.
    pub fn paste(
        &mut self,
        pasted: &str,
    ) -> usize {
        let digits: Vec<char> = pasted
            .chars()
            .filter(char::is_ascii_digit)
            .take(PAIRING_CODE_LENGTH)
            .collect();
        self.slots = [None; PAIRING_CODE_LENGTH];
        for (slot, digit) in self.slots.iter_mut().zip(&digits) {
            *slot = Some(*digit);
        }
        digits.len().min(PAIRING_CODE_LENGTH - 1)
    }

    pub fn is_complete(&self) -> bool {
        self.slots.iter().all(Option::is_some)
    }

    pub fn value(&self) -> String {
        self.slots.iter().flatten().collect()
    }

    /// Which slot Backspace should move focus to, or `None` to let the normal in-place clear happen. Only
    /// fires when the current slot is already empty -- a filled slot clears itself first.
    pub fn backspace_focus(
        &self,
        index: usize,
    ) -> Option<usize> {
        (self.slot(index).is_none() && index > 0).then(|| index - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_keeps_only_the_last_digit_and_moves_on() {
        let mut code = PairingCode::default();
        assert_eq!(code.set_digit(0, "7"), Some(1));
        assert_eq!(code.set_digit(0, "78"), Some(1));
        assert_eq!(code.slot(0), Some('8'));
        assert_eq!(
            code.set_digit(5, "1"),
            None,
            "the last slot has nowhere to go"
        );
    }

    #[test]
    fn a_non_digit_clears_the_slot_and_stays_put() {
        let mut code = PairingCode::default();
        code.set_digit(2, "4");
        assert_eq!(code.set_digit(2, "x"), None);
        assert_eq!(code.slot(2), None);
    }

    #[test]
    fn an_out_of_order_digit_leaves_the_gap_alone() {
        let mut code = PairingCode::default();
        code.paste("12");
        code.set_digit(4, "9");
        assert_eq!(
            (code.slot(1), code.slot(2), code.slot(4)),
            (Some('2'), None, Some('9'))
        );
        assert!(!code.is_complete());
        assert_eq!(code.value(), "129");
    }

    #[test]
    fn a_paste_strips_non_digits_truncates_and_reports_where_focus_goes() {
        let mut code = PairingCode::default();
        assert_eq!(code.paste("12-34 56 78"), 5);
        assert_eq!(code.value(), "123456");
        assert!(code.is_complete());
        assert_eq!(code.paste("12"), 2);
        assert_eq!(code.paste(""), 0);
        assert_eq!(code.value(), "");
    }

    #[test]
    fn backspace_moves_back_only_from_an_empty_slot() {
        let mut code = PairingCode::default();
        code.set_digit(1, "5");
        assert_eq!(code.backspace_focus(1), None);
        assert_eq!(code.backspace_focus(2), Some(1));
        assert_eq!(code.backspace_focus(0), None);
        code.clear_digit(1);
        assert_eq!(code.slot(1), None);
    }
}

use super::pairing_code::PairingCode;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AddStage {
    #[default]
    Details,
    Pairing,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AddSubmission {
    pub name: String,
    pub url: String,
    pub code: String,
}

/// A daemon worth pairing with is one worth reaching first: stage one collects Name and URL and
/// health-checks the URL before stage two ever asks for a pairing code that only the right machine could
/// accept anyway.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AddConnectionForm {
    pub stage: AddStage,
    pub name: String,
    pub url: String,
    pub code: PairingCode,
    pub health_error: bool,
    pub checking: bool,
}

impl AddConnectionForm {
    /// Reopening starts clean.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    /// Editing the URL withdraws the verdict on the old one.
    pub fn set_url(&mut self, url: &str) {
        self.url = url.to_owned();
        self.health_error = false;
    }

    pub fn can_continue(&self) -> bool {
        !self.name.trim().is_empty() && !self.url.trim().is_empty() && !self.checking
    }

    /// Continue pressed: the URL to health-check, or `None` when the press does nothing.
    pub fn begin_check(&mut self) -> Option<String> {
        if !self.can_continue() {
            return None;
        }
        self.health_error = false;
        self.checking = true;
        Some(self.url.trim().to_owned())
    }

    pub fn finish_check(&mut self, reachable: bool) {
        self.checking = false;
        if reachable {
            self.stage = AddStage::Pairing;
        } else {
            self.health_error = true;
        }
    }

    pub fn back(&mut self) {
        self.stage = AddStage::Details;
    }

    pub fn can_submit(&self, busy: bool) -> bool {
        self.code.is_complete() && !busy
    }

    pub fn submit(&self, busy: bool) -> Option<AddSubmission> {
        self.can_submit(busy).then(|| AddSubmission {
            name: self.name.trim().to_owned(),
            url: self.url.trim().to_owned(),
            code: self.code.value(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> AddConnectionForm {
        let mut form = AddConnectionForm::default();
        form.set_name("  Home ");
        form.set_url(" https://home:9 ");
        form
    }

    #[test]
    fn continue_needs_a_name_and_a_url() {
        let mut form = AddConnectionForm::default();
        assert!(!form.can_continue());
        form.set_name("Home");
        assert!(!form.can_continue());
        form.set_url("   ");
        assert!(!form.can_continue());
        form.set_url("https://home");
        assert!(form.can_continue());
        assert_eq!(AddConnectionForm::default().begin_check(), None);
    }

    #[test]
    fn a_reachable_url_moves_to_pairing_and_an_unreachable_one_stays_with_an_error() {
        let mut form = filled();
        assert_eq!(form.begin_check().as_deref(), Some("https://home:9"));
        assert!(
            form.checking && !form.can_continue(),
            "no second check while one is in flight"
        );
        assert_eq!(form.begin_check(), None);
        form.finish_check(false);
        assert_eq!(
            (form.stage, form.health_error, form.checking),
            (AddStage::Details, true, false)
        );
        form.set_url("https://other");
        assert!(!form.health_error, "editing the URL withdraws the error");
        form.begin_check();
        form.finish_check(true);
        assert_eq!(form.stage, AddStage::Pairing);
    }

    #[test]
    fn submit_needs_a_complete_code_and_is_trimmed() {
        let mut form = filled();
        form.finish_check(true);
        assert_eq!(form.submit(false), None);
        form.code.paste("123456");
        assert_eq!(form.submit(true), None, "busy blocks a second submit");
        assert_eq!(
            form.submit(false),
            Some(AddSubmission {
                name: "Home".into(),
                url: "https://home:9".into(),
                code: "123456".into()
            })
        );
    }

    #[test]
    fn back_returns_to_details_and_reopening_starts_clean() {
        let mut form = filled();
        form.finish_check(true);
        form.back();
        assert_eq!(form.stage, AddStage::Details);
        form.reset();
        assert_eq!(form, AddConnectionForm::default());
    }
}

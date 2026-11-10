//! The Settings values: shared between the page (an action carries them) and the Settings modal
//! (which edits them), so both observe the one entity.

use quiver_state::queries::mutations::Variables;

#[derive(Default)]
pub struct FormValues {
    pub values: Variables,
}

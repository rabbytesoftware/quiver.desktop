//! Arrow page decisions: header model, action row, progress control, methods, switch flow
//! (port of `src/features/arrow-details`, shaped to the new design: no tabs, README + Details,
//! Settings and Methods as modals).
//!
//! Nothing here touches the store or Core: the view layer fills [`ArrowFacts`] and calls these pure
//! functions, then performs the calls itself.

pub mod action_row;
pub mod actions;
pub mod channel;
pub mod dependency_rows;
pub mod details;
pub mod facts;
pub mod from_detail;
pub mod glance;
pub mod header;
pub mod inference;
pub mod invoke;
pub mod methods;
pub mod platform;
pub mod progress;
pub mod release_variables;
pub mod status;
pub mod switch;
pub mod variable_form;

pub use action_row::{build_action_row, ActionButton, ActionInfo, ActionRow, ActionSlot};
pub use actions::{
    compute_actions, launch_label, ArrowAction, ArrowActionKind, ArrowActionVariant,
};
pub use facts::{
    ActiveRunFacts, ArrowFacts, LifecycleFacts, MethodFacts, RunStepFacts, StepSpec, StepStatus,
};
pub use from_detail::{details_facts, facts_from_detail, last_run_facts};
pub use header::{build_header, HeaderContext, HeaderModel, VersionControl};
pub use progress::{ProgressControl, ProgressStep, StepState};
pub use status::{compute_status, ArrowProblem, ArrowStatus, RunState, StatusGlyph};

#[cfg(test)]
#[path = "action_row_tests.rs"]
mod action_row_tests;
#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;

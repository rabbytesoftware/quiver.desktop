//! Settings screen logic: the engine tab (config patches, defaults/running/configured diffs, rejection
//! handling), automatic arrow registration (Fletcher), log levels, PATH setup state, catalog visibility
//! and the tab list (port of `src/features/settings`).
//!
//! The screen is a modal in the new design; the logic is unchanged.

pub mod auto_register;
pub mod catalog_visibility;
pub mod core_options;
pub mod core_update;
pub mod engine;
pub mod engine_io;
pub mod engine_model;
pub mod log_level;
pub mod path;
pub mod preferences;
pub mod tabs;

pub use auto_register::{
    auto_register_patch, auto_register_rejection, auto_register_reset_patch, is_auto_register_on,
};
pub use engine::{parse_patch_response, ConfigView, EngineState, PatchResult, Rejection};
pub use engine_model::{build_engine_model, EngineModel};
pub use log_level::{normalise_level, Level};
pub use path::{path_state, PathState, PathStatus, PathStore};
pub use tabs::{visible_tabs, SettingsTab, TabItem};

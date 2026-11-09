//! The window shell: rail side and width, theme preference, traffic-light geometry
//! (port of `src/features/shell`). Persisted values are exposed as plain `normalise_*` functions;
//! storing them is the persistence layer's job.

mod geometry;
mod store;
mod theme;

pub use geometry::{
    rail_owns_controls, window_controls, ControlsEdge, SidebarSide, WindowControls, ROW_H,
};
pub use store::{
    normalise_side, normalise_width, ShellSettings, SHELL_STORAGE_KEY, SIDEBAR_DEFAULT,
    SIDEBAR_MAX, SIDEBAR_MIN,
};
pub use theme::{normalise_theme_preference, ThemePreference, THEME_STORAGE_KEY};

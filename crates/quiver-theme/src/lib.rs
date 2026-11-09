//! Quiver design tokens, ported from `src/index.css` (the Coss UI / Tailwind variables).
//!
//! Pure data: no GPUI. `quiver-ui` converts these into GPUI colours; views never hard-code a colour,
//! they read it from [`Tokens`].

mod color;
mod layout;
mod palette;
mod tokens;

pub use color::{Color, Oklch};
pub use layout::{FontFamily, Layout, Radius, TypeScale, LAYOUT};
pub use tokens::{Mode, Tokens};

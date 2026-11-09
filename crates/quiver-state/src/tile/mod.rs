//! The one tile component's model (new design, `TileAfter`): used by Home, Search, Library,
//! Collections, Collection detail and Recommended.
//!
//! Badge on the art top-right, caption = description only, hover shows title and tags for banner tiles,
//! hover Start button top-right. A tile renders what [`TileModel`] says and decides nothing.

mod badge;
mod model;
mod sources;

pub use badge::{badge_for, BadgePolicy, TileBadge, TileBadgeKind};
pub use model::{
    build_tile, hue_for, HoverContent, TileAction, TileArt, TileInput, TileKind, TileModel,
};

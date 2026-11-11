//! The chrome the tile-grid screens (Recommended, Search, Library, Collections, Collection) share:
//! back link + header, filter chips, the wavy "Load more", loading / empty / error states and the
//! feed that keeps a Core read fresh. None of it decides anything; the screens pass what
//! `quiver-state` said.

mod chips;
mod feed;
mod header;
mod layout;
mod load_more;
mod states;
#[cfg(test)]
pub mod testing;

pub use chips::{chip, counted_chip};
pub use feed::{start_feed, Feed};
pub use header::{back_link, page_header};
pub use layout::{grid_tile_width, page, tile_grid, GRID_GAP_X, GRID_GAP_Y};
pub use load_more::load_more;
pub use states::{error_state, notice, skeleton};

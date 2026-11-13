//! Sidebar logic: primary navigation, the arrow list, the row context menu and the resize handle
//! (port of `src/features/sidebar`). Rendering lives in `quiver-ui`.

mod list;
mod menu;
mod nav;
mod resize;

pub use list::{build_list, ArrowListState, CatalogPhase, ListNotice};
pub use menu::{arrow_menu_items, ArrowMenuItem};
pub use nav::{active_destination, blocks_reselect, Destination, DESTINATIONS};
pub use resize::{nudge_for_key, width_after, NUDGE};

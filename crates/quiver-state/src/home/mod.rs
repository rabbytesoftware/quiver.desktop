//! Home: a calm overview of what you already have, plus the recommended shelves
//! (port of `src/features/home`; spec `docs/home-page-spec.md`).

mod model;
mod popular;
mod shelves;

pub use model::{
    build_home, HomeInput, HomeModel, HomeSection, ShelfView, COLLECTIONS_PREVIEW_LIMIT,
    LEAD_SHELF_LIMIT, LIBRARY_PREVIEW_LIMIT, RECENTS_LIMIT,
};
pub use popular::{popular_slice, PopularSlice, POPULAR_CAP, POPULAR_FIRST, POPULAR_STEP};
pub use shelves::{is_snapshot_empty, visible_shelves};

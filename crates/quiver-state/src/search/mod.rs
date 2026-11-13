//! Search UI logic: query matching, sort, facets, column cap, the sidebar field's commit rules and the
//! results screen model (port of `src/features/search`). The two-lane pass itself lives in
//! `store::search_pass`.

pub mod bar;
pub mod columns;
pub mod lookup;
pub mod narrow;
pub mod offered;
pub mod relevance;
pub mod resolve;
pub mod results;
pub mod sort;
pub mod tile_width;

pub use bar::{
    commit_target, needs_namespace_lookup, submit_action, CommitTarget, SearchField, SubmitAction,
    LOCAL_DEBOUNCE_MS,
};
pub use columns::{column_cap, column_rule, COLUMN_GAP, MIN_TILE};
pub use lookup::resolve_namespace;
pub use narrow::{
    apply_selection, facets_for, host_of, is_narrowed, toggle, Facet, FacetKind, Selection,
};
pub use offered::offered_facets;
pub use relevance::{matches_query, relevant_local};
pub use resolve::{
    looks_like_namespace, pick_target, should_check_collection, NamespaceTarget, TargetKind,
};
pub use results::{build_results, EmptyState, ResultsInput, ResultsModel, SourceLine};
pub use sort::{sort_entries, SortKey, DEFAULT_SORT, SORT_KEYS};
pub use tile_width::{equal_columns_width, tile_width};

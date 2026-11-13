//! Plain domain types, ported from `src/domain/*.ts`. Wire-shaped (serde) because
//! several of them travel through the daemon's JSON unchanged.

pub mod arrow;
pub mod collection;
pub mod connection;
pub mod detail;
pub mod home;
pub mod release;
pub mod search;
pub mod step;

pub use arrow::*;
pub use collection::*;
pub use connection::*;
pub use detail::*;
pub use home::*;
pub use release::*;
pub use search::*;
pub use step::*;

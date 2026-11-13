//! Wire shapes of the daemon's v0 API and their conversions into domain types
//! (`src/lib/core-store/dtos/v0`). Every nullable list the wire can send is defaulted here, at the boundary.

pub mod arrow;
pub mod collection;
pub mod config;
pub mod home;
pub mod manifest;
pub mod path;
pub mod runtime;
pub mod search;

#[cfg(test)]
mod arrow_tests;

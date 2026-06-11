#![forbid(unsafe_code)]
//! Schema validation public compatibility module.
//!
//! The canonical implementation lives in [`legacy`]. This module preserves the
//! existing `crate::schema::*` public API by re-exporting the canonical
//! implementation instead of carrying a second copy.

pub mod legacy;

pub use legacy::*;

#![forbid(unsafe_code)]
//! Type and taint validation public compatibility module.
//!
//! The canonical implementation lives in [`finishing`]. This module preserves
//! the existing `crate::type_taint::*` public API by re-exporting the canonical
//! implementation instead of carrying a second copy.

pub mod finishing;

pub use finishing::*;

#[cfg(test)]
#[path = "type_taint_tests.rs"]
mod tests;

#![forbid(unsafe_code)]
//! Reference validation public compatibility module.
//!
//! The canonical implementation lives in [`v2`]. This module preserves the
//! existing `crate::references::*` public API by re-exporting the canonical
//! implementation instead of carrying a second copy.

pub mod v2;

pub use v2::*;

#![forbid(unsafe_code)]

//! Diagnostic render mapping.
//!
//! Wave-16 renamed the monolithic `mapping.rs` to `mapping/orig.rs`. The
//! existing code under `mapping/orig.rs` is the canonical implementation;
//! this `mod.rs` re-exports its public surface so existing callers continue
//! to import via `super::mapping::error_diagnostic_parts`.
#![allow(unreachable_pub)]

mod orig;

pub use orig::error_diagnostic_parts;

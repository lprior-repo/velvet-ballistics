#![forbid(unsafe_code)]

//! Source location tracking for YAML nodes.
//!
//! Provides [`SourceMap`] which maps YAML node indices to (line, column)
//! spans extracted from the parser event stream.

#[path = "source_map_types.rs"]
mod source_map_types;
#[path = "source_map_build.rs"]
mod source_map_build;

pub use source_map_types::*;
pub use source_map_build::*;

#[cfg(test)]
#[path = "source_map_tests.rs"]
mod source_map_tests;

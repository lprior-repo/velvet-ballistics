#![forbid(unsafe_code)]

//! Source location tracking for YAML nodes.
//!
//! Provides [`SourceMap`] which maps YAML node indices to (line, column)
//! spans extracted from the parser event stream.

#[path = "yaml_source_map_build.rs"]
mod source_map_build;
#[path = "yaml_source_map_types.rs"]
mod source_map_types;

pub use source_map_build::*;
pub use source_map_types::*;

#[cfg(test)]
#[path = "yaml_source_map_tests.rs"]
mod source_map_tests;

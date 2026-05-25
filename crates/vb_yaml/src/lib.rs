#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
// Pedantic allows: these lints are documentation-only or would require pervasive
// changes with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]

//! Cold-path YAML parsing and profile enforcement for velvet-ballistics.
//!
//! This crate wraps `saphyr-parser` to provide strict YAML event parsing,
//! profile rejection (anchors, aliases, merge keys, duplicate keys, etc.),
//! source maps, span tracking, and a typed AST. The runtime never depends on
//! this crate.

pub mod ast;
pub mod events;
pub mod profile;
pub mod source_map;

mod error;
mod limits;

pub use error::*;
pub use limits::*;

/// Parse YAML text into a stream of typed events.
///
/// This is the lowest-level public API. It runs profile validation first,
/// then returns the event stream for further processing.
pub fn parse_yaml_events(text: &str) -> crate::YamlResult<Vec<events::YamlEvent>> {
    profile::validate_yaml_profile(text)?;
    events::collect_events(text)
}

/// Parse YAML text into a typed [`ast::WorkflowSource`] AST.
///
/// Combines profile validation, event collection, and AST construction
/// into a single call for convenience.
pub fn parse_workflow_source(text: &str) -> crate::YamlResult<ast::WorkflowSource> {
    profile::validate_yaml_profile(text)?;
    ast::parse_workflow_ast(text)
}

/// Validate that the YAML text conforms to the strict profile.
///
/// Rejects anchors, aliases, merge keys, custom tags, binary scalars,
/// multiple documents, and YAML 1.1 ambiguous booleans.
pub fn validate_yaml_profile(text: &str) -> crate::YamlResult<()> {
    profile::validate_yaml_profile(text)
}

/// Reject duplicate keys in a YAML mapping.
///
/// Intended for use by downstream consumers that need to check a list of
/// key-value pairs for duplicates after parsing.
pub fn reject_duplicate_keys(keys: &[&str]) -> crate::YamlResult<()> {
    profile::reject_duplicate_keys(keys)
}

/// Reject forbidden YAML features from a pre-collected event list.
///
/// This is a secondary check that can be applied to already-parsed events.
pub fn reject_forbidden_yaml_features(events: &[events::YamlEvent]) -> crate::YamlResult<()> {
    profile::reject_forbidden_features(events)
}

/// Reject anchors, aliases, and merge keys from a pre-collected event list.
pub fn reject_anchors_aliases_merges(events: &[events::YamlEvent]) -> crate::YamlResult<()> {
    profile::reject_anchors_aliases_merges(events)
}

/// Reject multiple YAML documents from a pre-collected event list.
pub fn reject_multiple_documents(events: &[events::YamlEvent]) -> crate::YamlResult<()> {
    profile::reject_multiple_documents(events)
}

/// Reject YAML 1.1 ambiguous boolean scalars (yes/no/on/off).
pub fn reject_yaml_1_1_ambiguous_scalars(scalars: &[&str]) -> crate::YamlResult<()> {
    profile::reject_yaml_1_1_ambiguous_scalars(scalars)
}

/// Build a source map from YAML text, mapping node indices to line/column spans.
pub fn build_source_map(text: &str) -> crate::YamlResult<source_map::SourceMap> {
    source_map::build_source_map(text)
}

/// Look up the span for a node by index in a pre-built source map.
pub fn span_for_node(
    map: &source_map::SourceMap,
    node_index: u32,
) -> Option<source_map::SourceSpan> {
    map.span_for_node(node_index)
}

/// Load a YAML fixture source for testing.
///
/// Validates the profile and returns the parsed [`ast::WorkflowSource`].
pub fn load_fixture_source(text: &str) -> crate::YamlResult<ast::WorkflowSource> {
    parse_workflow_source(text)
}

#[cfg(test)]
mod lib_tests;

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Cold-path YAML parsing and profile enforcement for velvet-ballastics.
//!
//! This crate wraps `saphyr-parser` to provide strict YAML event parsing,
//! profile rejection (anchors, aliases, merge keys, duplicate keys, etc.),
//! source maps, span tracking, and a typed AST. The runtime never depends on
//! this crate.

pub mod ast;
pub mod events;
pub mod profile;
pub mod source_map;

use thiserror::Error;

/// YAML parsing error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum YamlError {
    #[error("unsupported YAML feature: {feature}")]
    UnsupportedFeature { feature: &'static str },

    #[error("duplicate key found: {key}")]
    DuplicateKey { key: Box<str> },

    #[error("anchor/alias/merge key rejected")]
    AnchorAliasMerge,

    #[error("custom tag rejected: {tag}")]
    CustomTag { tag: Box<str> },

    #[error("binary scalar rejected")]
    BinaryScalar,

    #[error("multiple documents rejected")]
    MultipleDocuments { count: usize },

    #[error("YAML 1.1 ambiguous scalar rejected: {scalar}")]
    AmbiguousScalar { scalar: Box<str> },

    #[error("source too large: {size} bytes, max {max}")]
    SourceTooLarge { size: usize, max: usize },

    #[error("nesting too deep: {depth}, max {max}")]
    NestingTooDeep { depth: u16, max: u16 },

    #[error("node limit exceeded: {count}, max {max}")]
    NodeLimitExceeded { count: u32, max: u32 },

    #[error("scalar too long: {len} bytes, max {max}")]
    ScalarTooLong { len: usize, max: usize },

    #[error("sequence too long: {len}, max {max}")]
    SequenceTooLong { len: usize, max: usize },

    #[error("mapping too large: {count} entries, max {max}")]
    MappingTooLarge { count: usize, max: usize },

    #[error("unknown field: {field}")]
    UnknownField { field: Box<str> },

    #[error("empty source")]
    EmptySource,

    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("field shape error: {field} expected {expected}")]
    FieldShape { field: &'static str, expected: &'static str },

    #[error("parse error at line {line}: {reason}")]
    ParseError { line: usize, reason: Box<str> },
}

/// Alias for results using [`YamlError`].
pub type YamlResult<T> = Result<T, YamlError>;

/// Strict YAML profile limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum source text size in bytes.
    pub max_source_bytes: usize,
    /// Maximum nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum scalar value length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        }
    }
}

/// Parse YAML text into a stream of typed events.
///
/// This is the lowest-level public API. It runs profile validation first,
/// then returns the event stream for further processing.
pub fn parse_yaml_events(text: &str) -> YamlResult<Vec<events::YamlEvent>> {
    profile::validate_yaml_profile(text)?;
    events::collect_events(text)
}

/// Parse YAML text into a typed [`ast::WorkflowSource`] AST.
///
/// Combines profile validation, event collection, and AST construction
/// into a single call for convenience.
pub fn parse_workflow_source(text: &str) -> YamlResult<ast::WorkflowSource> {
    profile::validate_yaml_profile(text)?;
    ast::parse_workflow_ast(text)
}

/// Validate that the YAML text conforms to the strict profile.
///
/// Rejects anchors, aliases, merge keys, custom tags, binary scalars,
/// multiple documents, and YAML 1.1 ambiguous booleans.
pub fn validate_yaml_profile(text: &str) -> YamlResult<()> {
    profile::validate_yaml_profile(text)
}

/// Reject duplicate keys in a YAML mapping.
///
/// Intended for use by downstream consumers that need to check a list of
/// key-value pairs for duplicates after parsing.
pub fn reject_duplicate_keys(keys: &[&str]) -> YamlResult<()> {
    profile::reject_duplicate_keys(keys)
}

/// Reject forbidden YAML features from a pre-collected event list.
///
/// This is a secondary check that can be applied to already-parsed events.
pub fn reject_forbidden_yaml_features(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_forbidden_features(events)
}

/// Reject anchors, aliases, and merge keys from a pre-collected event list.
pub fn reject_anchors_aliases_merges(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_anchors_aliases_merges(events)
}

/// Reject multiple YAML documents from a pre-collected event list.
pub fn reject_multiple_documents(events: &[events::YamlEvent]) -> YamlResult<()> {
    profile::reject_multiple_documents(events)
}

/// Reject YAML 1.1 ambiguous boolean scalars (yes/no/on/off).
pub fn reject_yaml_1_1_ambiguous_scalars(scalars: &[&str]) -> YamlResult<()> {
    profile::reject_yaml_1_1_ambiguous_scalars(scalars)
}

/// Build a source map from YAML text, mapping node indices to line/column spans.
pub fn build_source_map(text: &str) -> YamlResult<source_map::SourceMap> {
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
pub fn load_fixture_source(text: &str) -> YamlResult<ast::WorkflowSource> {
    parse_workflow_source(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_source() {
        let result = validate_yaml_profile("");
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_simple_mapping() {
        let yaml = "key: value\n";
        let result = validate_yaml_profile(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_events_returns_typed_events() {
        let yaml = "a: 1\n";
        let events = parse_yaml_events(yaml).unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn reject_duplicate_keys_detects_dups() {
        let keys = vec!["foo", "bar", "foo"];
        let result = reject_duplicate_keys(&keys);
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_keys_allows_unique() {
        let keys = vec!["foo", "bar", "baz"];
        let result = reject_duplicate_keys(&keys);
        assert!(result.is_ok());
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_rejects_yes() {
        let scalars = vec!["yes"];
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        assert!(result.is_err());
    }

    #[test]
    fn reject_yaml_1_1_ambiguous_allows_true() {
        let scalars = vec!["true"];
        let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
        assert!(result.is_ok());
    }
}

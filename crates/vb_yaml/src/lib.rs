#![forbid(unsafe_code)]
//! Cold-path YAML parsing boundary for Velvet Ballistics.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! native Rust `saphyr` parsing.

pub mod yaml_ast;
pub mod yaml_error;
pub mod yaml_events;
pub mod yaml_limits;
pub mod yaml_profile;
pub mod yaml_source_map;

#[cfg(kani)]
pub mod yaml_kani;

// Re-exports for compatibility
pub use yaml_error::{YamlError, YamlResult};
pub use yaml_events::{EventSpan, ScalarStyle, YamlEvent, collect_events, convert_event};
pub use yaml_profile::{
    reject_anchors_aliases_merges, reject_duplicate_keys, reject_forbidden_features,
    reject_multiple_documents, reject_yaml_1_1_ambiguous_scalars, validate_yaml_profile,
};
pub use yaml_source_map::{
    SemanticSourceMap, SourceMap, SourceSpan, build_semantic_source_map, build_source_map,
    span_for_node,
};

#[cfg(any(test, feature = "test-util"))]
pub use yaml_ast::types::WorkflowSourceParts;
pub use yaml_ast::types::{
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, ExampleAst, InputField, ResultMapping,
    RetryPolicy, ScalarValue, SecretField, StepAst, StepPrimitive, TogetherBranch, TriggerAst,
    VarField, WorkflowSource,
};

/// Parse YAML text into a vector of YAML events.
pub fn parse_yaml_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
    yaml_profile::validate_yaml_profile(text)?;
    yaml_events::collect_events(text)
}

/// Parse YAML text into a WorkflowSource AST.
pub fn parse_workflow_source(text: &str) -> YamlResult<yaml_ast::WorkflowSource> {
    yaml_profile::validate_yaml_profile(text)?;
    yaml_ast::parse_workflow_ast(text)
}

/// Load a workflow source from a fixture (same as parse_workflow_source).
pub fn load_fixture_source(text: &str) -> YamlResult<yaml_ast::WorkflowSource> {
    parse_workflow_source(text)
}

/// Reject forbidden YAML features from a list of events.
pub fn reject_forbidden_yaml_features(events: &[YamlEvent]) -> YamlResult<()> {
    yaml_profile::reject_forbidden_features(events)
}

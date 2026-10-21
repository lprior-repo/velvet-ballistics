#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

pub mod ast;
mod control_flow;
pub mod expression;
mod expression_bytecode;
mod limits;
mod mod_compile_core;
mod mod_compile_errors;
pub mod mod_compile_lowering;
mod mod_compile_validation;
mod references;
mod schema;
pub mod strict_yaml;
mod type_taint;

// Import expression and YAML modules from dedicated crates
pub use vb_expr::{
    ExprError, ExprResult,
    bytecode, eval, lexer, parser, stack_ops, typecheck,
    compile_expr, compile_expr_to_bytecode, compile_expr_with_pool,
    compile_expr_with_resolver, check_expr_stack_bound, ReferenceResolver,
    eval_binary_op, eval_expr_program, eval_expr_program_with_store,
    eval_helper, eval_helper_with_store, eval_unary_op,
};

pub use vb_yaml::{
    YamlError, YamlResult,
    YamlEvent, EventSpan, ScalarStyle,
    collect_events, convert_event,
    reject_anchors_aliases_merges, reject_duplicate_keys,
    reject_forbidden_features, reject_multiple_documents,
    reject_yaml_1_1_ambiguous_scalars, validate_yaml_profile,
    SourceMap, SourceSpan, SemanticSourceMap,
    build_source_map, build_semantic_source_map, span_for_node,
    WorkflowSource,
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, ExampleAst,
    InputField, ResultMapping, RetryPolicy, ScalarValue, SecretField,
    StepAst, StepPrimitive, TogetherBranch, TriggerAst, VarField,
};

#[cfg(any(test, feature = "test-util"))]
pub use vb_yaml::WorkflowSourceParts;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Proptest properties for ChooseSlot lowering (vb-282my).
#[cfg(test)]
mod proptest_choose_lowering;

// Kani harnesses disabled (see original lib.rs comments)
// ... (same #[cfg(any())] blocks as original)

// Internal test modules (error variant completeness, together digest unit tests).
#[cfg(test)]
mod tests;

use mod_compile_core as core;
use mod_compile_errors as errors;
use mod_compile_lowering as lwr;
use mod_compile_validation as validation;

pub use core::{
    YamlCompiler, YamlLimits, build_accessor_table, build_constant_pool, build_slot_layout,
    check_idempotency_gates, compile_workflow, compile_workflow_with_contracts,
    compute_compiled_digest, emit_compiled_artifact, is_compile_idempotency_gate_accepted,
};
pub use errors::{CompileError, CompileErrors, SourceMark};
pub(crate) use errors::{collect, non_string_key_error};
#[allow(unused_imports)]
pub(crate) use lwr::digest_step_primitive as digest_step_primitive_part05;
pub use lwr::{
    SlotCompiler, WaitKind, canonical_digest, canonical_digest as canonical_digest_part05,
    compile_source, lower_ask, lower_choose, lower_collect, lower_do, lower_finish, lower_for_each,
    lower_reduce, lower_repeat, lower_set, lower_steps_to_ir, lower_together, lower_wait,
    validate_ir,
};
pub(crate) use validation::validate_public_name;

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

// Provide convenience wrapper functions that delegate to vb_yaml
pub fn parse_yaml_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
    vb_yaml::parse_yaml_events(text)
}

pub fn parse_workflow_source(text: &str) -> YamlResult<WorkflowSource> {
    vb_yaml::parse_workflow_source(text)
}

pub fn load_fixture_source(text: &str) -> YamlResult<WorkflowSource> {
    vb_yaml::load_fixture_source(text)
}

pub fn reject_forbidden_yaml_features(events: &[YamlEvent]) -> YamlResult<()> {
    vb_yaml::reject_forbidden_yaml_features(events)
}

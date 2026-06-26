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

// Expression modules (folded from vb_expr)
pub mod expr_lexer;
pub mod expr_parser;
pub mod expr_bytecode;
pub mod expr_eval;
pub mod expr_typecheck;
pub mod expr_proofs;

#[cfg(test)]
mod expr_property_tests;

pub mod expr_stack_ops;
mod expr_slot_eval;
mod expr_builtin_eval;
#[cfg(test)]
mod expr_eval_tests;

// Re-exports for backward compatibility (vb_expr public API)
pub use expr_lexer as lexer;
pub use expr_parser as parser;
pub use expr_bytecode as bytecode;
pub use expr_eval as eval;
pub use expr_typecheck as typecheck;
pub use expr_stack_ops as stack_ops;

// Expression error type (moved from vb_expr)
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprError {
    #[error("unexpected token: {token}")]
    UnexpectedToken { token: String },
    #[error("unexpected end of expression")]
    UnexpectedEof,
    #[error("unknown operator: {op}")]
    UnknownOperator { op: String },
    #[error("unknown helper: {helper}")]
    UnknownHelper { helper: String },
    #[error("stack overflow: max {max}")]
    StackOverflow { max: u8 },
    #[error("stack underflow")]
    StackUnderflow,
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    #[error("division by zero")]
    DivisionByZero,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("invalid reference: {reference}")]
    InvalidReference { reference: String },
    #[error("expression too long: {len} tokens, max {max}")]
    ExpressionTooLong { len: usize, max: usize },
    #[error("unterminated string")]
    UnterminatedString,
    #[error("integer out of range")]
    IntegerOutOfRange,
    #[error("non-finite float")]
    NonFiniteFloat,
    #[error("unexpected character: {ch}")]
    UnexpectedChar { ch: char },
    #[error("parse depth exceeded: max {max}")]
    ParseDepthExceeded { max: usize },
    #[error("too many helper arguments: {len}, max {max}")]
    TooManyHelperArgs { len: usize, max: usize },
    #[error("helper arity mismatch: {helper} expects {expected}, got {actual}")]
    HelperArityMismatch { helper: String, expected: usize, actual: usize },
    #[error("bytecode too long: {len} ops, max {max}")]
    BytecodeTooLong { len: usize, max: usize },
    #[error("constant pool overflow")]
    ConstantPoolOverflow,
    #[error("unsupported literal: {literal}")]
    UnsupportedLiteral { literal: String },
}

impl From<vb_core::CoreError> for ExprError {
    fn from(e: vb_core::CoreError) -> Self {
        match e {
            vb_core::CoreError::NonFiniteNumber => ExprError::NonFiniteFloat,
            vb_core::CoreError::DivisionByZero => ExprError::DivisionByZero,
            vb_core::CoreError::ExpressionStackUnderflow => ExprError::StackUnderflow,
            vb_core::CoreError::ExpressionStackOverflow { max } => ExprError::StackOverflow { max },
            vb_core::CoreError::TypeMismatch { expected, found } => ExprError::TypeMismatch {
                expected: expected.to_string(),
                found: found.to_string(),
            },
            _ => ExprError::UnexpectedEof,
        }
    }
}
pub type ExprResult<T> = Result<T, ExprError>;

// Re-exports from expr modules (vb_expr public API)
pub use bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver,
};
pub use eval::{
    eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper,
    eval_helper_with_store, eval_unary_op,
};

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

// YAML parsing layer (moved from vb_yaml)
pub mod yaml_ast;
pub mod yaml_events;
pub mod yaml_profile;
pub mod yaml_source_map;
pub mod yaml_error;
pub mod yaml_limits;

#[cfg(kani)]
pub mod yaml_kani;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Proptest properties for ChooseSlot lowering (vb-282my).
#[cfg(test)]
mod proptest_choose_lowering;

// Kani harnesses for Finish digest verification (vb-xi2f.34).
#[cfg(kani)]
pub mod kani_finish_digest;

// Internal test modules (error variant completeness, together digest unit tests).
#[cfg(test)]
mod tests;

// Kani harnesses for canonical_primitive_name coverage (vb-xi2f.16, vb-xi2f.29).
#[cfg(kani)]
pub mod kani_canonical_name;

// Kani harnesses for together digest step verification (vb-xi2f.29).
#[cfg(kani)]
pub mod together_digest_kani;

// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(kani)]
pub mod kani_idempotency_parity;

// Kani harnesses for vb-a001 for_each lowering fix verification.
// Proves PRE-002 (body SetConst.next = ForEachNext), PRE-005 (no backward edges),
// PRE-006 (all nodes reachable), POST-003 (malformed IR rejection).
#[cfg(kani)]
pub mod kani_foreach_parity;

// Kani harnesses for repeat/ask id+1 lowering overflow rejection.
#[cfg(kani)]
pub mod kani_lower_control;

// Kani harnesses for vb-xi2f.33: digest covering Ask primitives.
// Feature-gated behind test-util because these harnesses depend on
// WorkflowSourceParts which is pub(crate) in production and only
// re-exported as pub when test-util feature is active.
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_ask_empty_prompt;
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_ask_field_ordering;
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_ask_prompt_sensitivity;
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_ask_timeout_sensitivity;
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_ask_timeout_sentinel;
#[cfg(all(kani, any(test, feature = "test-util")))]
pub mod kani_digest_step_primitive_no_panic;

// Kani harnesses for wait digest coverage verification (vb-xi2f.32).
#[cfg(kani)]
pub mod kani_wait_digest;

// Kani harnesses for Repeat digest coverage (bead vb-xi2f.31).
// PO-001 through PO-005: digest_step_primitive Repeat { max_attempts, body }.
#[cfg(kani)]
pub mod kani_digest_repeat;

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

// Re-export YAML parsing layer (formerly vb_yaml)
pub use yaml_error::YamlError;
pub use yaml_error::YamlResult;

pub use yaml_events::{collect_events, convert_event, EventSpan, ScalarStyle, YamlEvent};
pub use yaml_profile::{
    reject_anchors_aliases_merges, reject_duplicate_keys, reject_forbidden_features,
    reject_multiple_documents, reject_yaml_1_1_ambiguous_scalars, validate_yaml_profile,
};
pub use yaml_source_map::{build_semantic_source_map, build_source_map, span_for_node, SourceMap, SourceSpan, SemanticSourceMap};
pub use yaml_ast::types::{
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, ExampleAst, InputField, ResultMapping,
    RetryPolicy, ScalarValue, SecretField, StepAst, StepPrimitive, TogetherBranch, TriggerAst,
    VarField, WorkflowSource,
};
#[cfg(any(test, feature = "test-util"))]
pub use yaml_ast::types::WorkflowSourceParts;

pub fn parse_yaml_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
    yaml_profile::validate_yaml_profile(text)?;
    yaml_events::collect_events(text)
}

pub fn parse_workflow_source(text: &str) -> YamlResult<yaml_ast::WorkflowSource> {
    yaml_profile::validate_yaml_profile(text)?;
    yaml_ast::parse_workflow_ast(text)
}

pub fn load_fixture_source(text: &str) -> YamlResult<yaml_ast::WorkflowSource> {
    parse_workflow_source(text)
}

pub fn reject_forbidden_yaml_features(events: &[YamlEvent]) -> YamlResult<()> {
    yaml_profile::reject_forbidden_features(events)
}

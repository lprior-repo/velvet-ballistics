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
//! ```
//! // Verify old public expr_proofs migration path still resolves
//! use vb_compile::expr_proofs;
//! ```
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

pub mod ast;

// Preserve the original public paths while the dedicated crates own the code.
pub use vb_expr::{
    expr_bytecode, expr_eval, expr_lexer, expr_parser, expr_proofs, expr_stack_ops, expr_typecheck,
};
#[cfg(kani)]
pub use vb_yaml::yaml_kani;
pub use vb_yaml::{yaml_ast, yaml_error, yaml_events, yaml_limits, yaml_profile, yaml_source_map};
mod control_flow;
pub mod expression;
mod expression_bytecode;
mod limits;
#[cfg(test)]
mod migration_compat;
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
    ExprError, ExprResult, ReferenceResolver, bytecode, check_expr_stack_bound, compile_expr,
    compile_expr_to_bytecode, compile_expr_with_pool, compile_expr_with_resolver, eval,
    eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper,
    eval_helper_with_store, eval_unary_op, lexer, parser, stack_ops, typecheck,
};

pub use vb_yaml::{
    AuthorEntry, AuthorValue, ChooseBranch, ErrorHandlerAst, EventSpan, ExampleAst, InputField,
    ResultMapping, RetryPolicy, ScalarStyle, ScalarValue, SecretField, SemanticSourceMap,
    SourceMap, SourceSpan, StepAst, StepPrimitive, TogetherBranch, TriggerAst, VarField,
    WorkflowSource, YamlError, YamlEvent, YamlResult, build_semantic_source_map, build_source_map,
    collect_events, convert_event, reject_anchors_aliases_merges, reject_duplicate_keys,
    reject_forbidden_features, reject_multiple_documents, reject_yaml_1_1_ambiguous_scalars,
    span_for_node, validate_yaml_profile,
};

#[cfg(any(test, feature = "test-util"))]
pub use vb_yaml::WorkflowSourceParts;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Proptest properties for ChooseSlot lowering (vb-282my).
#[cfg(test)]
mod proptest_choose_lowering;

// === Kani harnesses disabled pending upstream StepAst/StepPrimitiveAst refactor port ===
// Tracked in BLOCK_GLOBAL follow-up: vb_xi2f_29_kani_proofs_post_stepast_refactor
// Original modules preserved below in cfg(any()) gate for future porting.

// Kani harnesses for Finish digest verification (vb-xi2f.34).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_finish_digest;

// Kani harnesses for canonical_primitive_name coverage (vb-xi2f.16, vb-xi2f.29).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_canonical_name;

// Kani harnesses for together digest step verification (vb-xi2f.29).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod together_digest_kani;

// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_idempotency_parity;

// Kani harnesses for vb-a001 for_each lowering fix verification.
// Proves PRE-002 (body SetConst.next = ForEachNext), PRE-005 (no backward edges),
// PRE-006 (all nodes reachable), POST-003 (malformed IR rejection).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_foreach_parity;

// Kani harnesses for repeat/ask id+1 lowering overflow rejection.
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_lower_control;
#[cfg(kani)]
pub mod kani_lower_repeat;

// Kani harnesses for vb-xi2f.33: digest covering Ask primitives.
// Feature-gated behind test-util because these harnesses depend on
// WorkflowSourceParts which is pub(crate) in production and only
// re-exported as pub when test-util feature is active.
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_ask_empty_prompt;
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_ask_field_ordering;
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_ask_prompt_sensitivity;
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_ask_timeout_sensitivity;
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_ask_timeout_sentinel;
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_step_primitive_no_panic;

// Kani harnesses for wait digest coverage verification (vb-xi2f.32).
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_wait_digest;

// Kani harnesses for Repeat digest coverage (bead vb-xi2f.31).
// PO-001 through PO-005: digest_step_primitive Repeat { max_attempts, body }.
#[cfg(any())]
// disabled: stale StepAst refs — see lib.rs:153 BLOCK_GLOBAL follow-up
pub mod kani_digest_repeat;

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
pub mod artifact;
pub use artifact::{
    ArtifactVerificationError, CompilationBinding, CompilerCheck, SourceProvenance,
    VerifiedCompilation, compile_verified_artifact, verify_compiled_artifact,
};
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
mod tests_target_identity;

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
mod compiled_ir;
mod control_flow;
pub use compiled_ir::{compute_compiled_ir_digest, encode_compiled_ir};
pub mod expression;
mod expression_bytecode;
mod limits;
mod mod_compile_core;
mod mod_compile_errors;
pub mod mod_compile_lowering;
mod mod_compile_validation;
mod references;
mod schema;
mod source_symbol_binding;
mod source_symbols;
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

pub use vb_yaml::WorkflowSourceParts;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Proptest properties for ChooseSlot lowering (vb-282my).
#[cfg(test)]
mod proptest_choose_lowering;

// vb-tkcc7: the dead always-false-gated kani fleet (15 wired modules + orphan files)
// was deleted outright. Re-enabling was rejected: the harnesses referenced the
// pre-refactor StepAst enum, the canonical kani CI story covers vb_core and
// vb_validate only, and digest/lowering behavior keeps proptest/unit coverage.

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
    compile_source, lower_ask, lower_choose, lower_do, lower_finish, lower_for_each, lower_reduce,
    lower_repeat, lower_set, lower_steps_to_ir, lower_together, lower_wait,
};
pub(crate) use validation::validate_public_name;

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

// Re-export the YAML entry points directly from the dedicated `vb_yaml` crate,
// which owns these functions. Names and signatures are unchanged.
pub use vb_yaml::{
    load_fixture_source, parse_workflow_source, parse_yaml_events, reject_forbidden_yaml_features,
};

// Test-only module: it exercises test-gated `vb_yaml` internals and holds
// no production items, so normal builds exclude it entirely.
#[cfg(test)]
mod tests_target_identity;

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
mod mod_compile_lowering;
mod mod_compile_validation;
mod references;
mod schema;
pub mod strict_yaml;
mod type_taint;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Kani harnesses for Finish digest verification (vb-xi2f.34).
#[cfg(kani)]
pub mod kani_finish_digest;

// TEMPORARILY DISABLED: pre-existing proptest macro compatibility issue in bytecode_ast_parity.rs
// #[cfg(test)]
// mod property_tests;

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
#[cfg(kani)]
pub mod kani_digest_ask_empty_prompt;
#[cfg(kani)]
pub mod kani_digest_ask_field_ordering;
#[cfg(kani)]
pub mod kani_digest_ask_prompt_sensitivity;
#[cfg(kani)]
pub mod kani_digest_ask_timeout_sensitivity;
#[cfg(kani)]
pub mod kani_digest_ask_timeout_sentinel;
#[cfg(kani)]
pub mod kani_digest_step_primitive_no_panic;

// Kani harnesses for wait digest coverage verification (vb-xi2f.32).
#[cfg(kani)]
pub mod kani_wait_digest;

pub use expression_bytecode::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};

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
pub use lwr::{
    SlotCompiler, WaitKind,
    canonical_digest, canonical_digest as canonical_digest_part05,
    lower_ask, lower_choose, lower_collect, lower_do, lower_finish, lower_for_each,
    lower_reduce, lower_repeat, lower_set, lower_steps_to_ir, lower_together, lower_wait,
    validate_ir,
};
#[allow(unused_imports)]
pub(crate) use lwr::{
    compile_source, digest_step_primitive as digest_step_primitive_part05,
};
pub(crate) use validation::validate_public_name;

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

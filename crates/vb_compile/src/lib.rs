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
// Compile-time variable-scope restriction tests for `$attempt.number`.
// Master spec §15 line 305 reserves the `attempt` keyword and §45 line 2473
// defines the `RepeatAttempt` IR node. The 19 tests under
// `restrictions::tests::attempt_number_tests` (4 B1 happy-path scope tests
// + 13 B2 error-path tests + 2 B0 metadata tests) verify that
// `$attempt.number` references are accepted inside repeat bodies and
// rejected with `InvalidVariableScope` outside them. The walker body and
// the error variant are added by vb-0xyvo and vb-ykv39 respectively;
// this declaration only wires the test surface so the harness runs.
#[cfg(test)]
mod restrictions;
mod schema;
pub mod strict_yaml;
mod type_taint;

// Proptest properties for Finish digest verification (vb-xi2f.34).
#[cfg(test)]
mod proptest_finish_digest;

// Enum parity tests verifying SideEffect / RetrySafety match master plan §65.
#[cfg(test)]
mod enums;

// Proptest properties for ChooseSlot lowering (vb-282my).
#[cfg(test)]
mod proptest_choose_lowering;

// ── vb-xi2f.22: Nested Together Body Lowering proptest properties ──
// RESOLVED (State 12 formal-verifier RETRY):
// - `crate::compile` → `crate::SlotCompiler` (pub re-export at crate root)
// - `part_01`/`part_04` private modules → items made pub(crate) + correct paths used
// - Syntax error in proptest_together_errors.rs:101 → resolved (was cascading import error)
#[cfg(test)]
mod proptest_body_dispatcher_together;
#[cfg(test)]
mod proptest_body_step_width;
#[cfg(test)]
mod proptest_budget_together;
#[cfg(test)]
mod proptest_gate11_together;
#[cfg(test)]
mod proptest_together_errors;
// ── end vb-xi2f.22 proptest properties ──

// ── vb-xi2f.22: Flux refinement annotations ──
// PO-001-F: body step width refinement (canonical_body_step_width for Together).
mod body_step_width_flux;
// PO-002-F: body dispatcher together refinement (emit_single_body_set for Together).
mod body_dispatcher_together_flux;
// ── end vb-xi2f.22 flux refinements ──

// Kani harnesses for Finish digest verification (vb-xi2f.34).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod kani_finish_digest;

// Master plan §38 ("Bytecode/AST parity" row, lines 1167-1170) requires
// the bytecode_ast_parity proptest to be wired in. The proptest currently
// surfaces a real parity violation: the production bytecode evaluator's
// `Sub` opcode (`vb_core::engine::expr_eval::ops::eval_i64_pair`) is
// I64-only, so `Neg(F64(x))` (which lowers to `0 - x` with both as F64)
// rejects F64 operands with `EvalErrorKind::TypeMismatch`, while the
// recursive AST oracle returns the correct negated F64 value. The
// `lower_numeric_negation` helper now emits `F64(0.0)` when the inner
// expression is statically F64, so the lowering itself is correct; the
// remaining gap is in the production evaluator's `Sub` arm, which needs
// to be extended to support F64 operands. The test is wired in and
// marked `#[ignore = "blocked by vb-3g1qq: ...; remove ignore after fix
// lands"]`. Follow-up bead: vb-3g1qq. The original lowering-bug
// triage is closed as vb-cwb90.
#[cfg(test)]
mod property_tests;

// Internal test modules (error variant completeness, together digest unit tests).
#[cfg(test)]
mod tests;

// Scope-guard regression tests for bead vb-sitry.
// Lives next to the production `references` module that owns the guard,
// and does not require the aspirational `Repeat { steps: ... }` shape
// that the cold AST does not yet preserve.
#[cfg(test)]
mod references_scope_guard_tests;

// Kani harnesses for canonical_primitive_name coverage (vb-xi2f.16, vb-xi2f.29).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod kani_canonical_name;

// Kani harnesses for together digest step verification (vb-xi2f.29).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod together_digest_kani;

// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod kani_idempotency_parity;

// Kani harnesses for vb-a001 for_each lowering fix verification.
// Proves PRE-002 (body SetConst.next = ForEachNext), PRE-005 (no backward edges),
// PRE-006 (all nodes reachable), POST-003 (malformed IR rejection).
#[cfg(all(kani, feature = "kani-compile-tier-a"))]
pub mod kani_foreach_parity;

// Kani harnesses for repeat/ask id+1 lowering overflow rejection.
#[cfg(all(kani, feature = "kani-compile-tier-a"))]
pub mod kani_lower_control;

// Kani harnesses for Save canonical name and digest prefix (vb-pkif2).
// Proves Save{value} canonical name is "set" and digest uses b"set" prefix.
// The harness lives in mod_compile_lowering/kani_proofs and is exported
// as a pub mod via that path; no src-level declaration needed.

// Kani harnesses for vb-xi2f.33: digest covering Ask primitives.
// Feature-gated behind test-util because these harnesses depend on
// WorkflowSourceParts which is pub(crate) in production and only
// re-exported as pub when test-util feature is active.
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_ask_empty_prompt;
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_ask_field_ordering;
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_ask_prompt_sensitivity;
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_ask_timeout_sensitivity;
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_ask_timeout_sentinel;
#[cfg(all(
    kani,
    feature = "kani-compile-legacy",
    any(test, feature = "test-util")
))]
pub mod kani_digest_step_primitive_no_panic;

// Kani harnesses for wait digest coverage verification (vb-xi2f.32).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod kani_wait_digest;

// Kani harnesses for Repeat digest coverage (bead vb-xi2f.31).
// PO-001 through PO-005: digest_step_primitive Repeat { max_attempts, body }.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod kani_digest_repeat;

// ── vb-xi2f.22: Nested Together Body Lowering Kani harnesses ──
// Kani harness for PO-001-K: body step width acceptance (GOD RULE 1: varied primitives).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod body_step_width_kani;
// Kani harness for PO-002-K: body dispatcher together acceptance.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod body_dispatcher_together_kani;
// Kani harness for PO-003-K: width/node parity (TH-1 defense).
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod width_parity_kani;
// Kani harness for PO-004-K: emission order monotonicity.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod emit_order_together_kani;
// Kani harness for PO-005-K: nested together 2-level lowering.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod nested_together_kani;
// Kani harness for PO-006-K: together error paths panic-free.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod together_error_paths_kani;
// Kani harness for PO-007-K: together digest nested.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod together_digest_nested_kani;
// Kani harness for PO-008-K: gate 11 together body acceptance.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod gate11_together_kani;
// Kani harness for PO-009-K: budget together body compliance.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod budget_together_kani;
// Kani harness for PO-010-K: comprehensive panic-freedom.
#[cfg(all(kani, feature = "kani-compile-legacy"))]
pub mod panic_free_together_lowering_kani;
// ── end vb-xi2f.22 Kani harnesses ──

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

pub mod budget_analyzer;
pub use budget_analyzer::compute_whole_workflow_budget;

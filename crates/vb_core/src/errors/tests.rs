//! Tests for the errors module.

use crate::errors::{
    CollectExtraHydrationFailureKind, CollectPageOrderViolationKind, CoreError, DiagnosticCode,
    EngineError,
};
use crate::ids::{
    ActionId, BlobId, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId,
};
use chrono::Utc;

// -- diagnostic_code is correct for every variant --

#[test]
fn core_error_diagnostic_code_invalid_program_counter() {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(5),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1001));
    assert_eq!(error.to_string(), "invalid program counter: StepIdx(5)");
}

#[test]
fn core_error_diagnostic_code_missing_next_step() {
    let error = CoreError::MissingNextStep {
        step: StepIdx::new(3),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1002));
    assert_eq!(error.to_string(), "missing next step for StepIdx(3)");
}

#[test]
fn core_error_diagnostic_code_slot_out_of_bounds() {
    let error = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(99),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1011));
    assert_eq!(error.to_string(), "slot index out of bounds: SlotIdx(99)");
}

#[test]
fn core_error_diagnostic_code_expr_out_of_bounds() {
    let error = CoreError::ExprOutOfBounds {
        expr: ExprIdx::new(7),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1015));
    assert_eq!(
        error.to_string(),
        "expression index out of bounds: ExprIdx(7)"
    );
}

#[test]
fn core_error_diagnostic_code_const_out_of_bounds() {
    let error = CoreError::ConstOutOfBounds {
        index: ConstIdx::new(12),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1013));
    assert_eq!(
        error.to_string(),
        "constant index out of bounds: ConstIdx(12)"
    );
}

#[test]
fn core_error_diagnostic_code_missing_output_slot() {
    let error = CoreError::MissingOutputSlot {
        step: StepIdx::new(2),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1305));
    assert_eq!(error.to_string(), "missing output slot for StepIdx(2)");
}

#[test]
fn core_error_diagnostic_code_step_state_out_of_bounds() {
    let error = CoreError::StepStateOutOfBounds {
        step: StepIdx::new(200),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1306));
    assert_eq!(
        error.to_string(),
        "step state index out of bounds: StepIdx(200)"
    );
}

#[test]
fn core_error_diagnostic_code_type_mismatch() {
    let error = CoreError::TypeMismatch {
        expected: "number",
        found: "boolean",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1101));
    assert_eq!(
        error.to_string(),
        "type mismatch: expected number, found boolean"
    );
}

#[test]
fn core_error_diagnostic_code_non_bool_condition() {
    let error = CoreError::NonBoolCondition {
        slot: SlotIdx::new(4),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1104));
    assert_eq!(
        error.to_string(),
        "type mismatch: expected boolean, found slot SlotIdx(4)"
    );
}

#[test]
fn core_error_diagnostic_code_division_by_zero() {
    let error = CoreError::DivisionByZero;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1103));
    assert_eq!(error.to_string(), "division by zero");
}

#[test]
fn core_error_diagnostic_code_non_finite_number() {
    let error = CoreError::NonFiniteNumber;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1102));
    assert_eq!(error.to_string(), "non-finite number is not allowed");
}

#[test]
fn core_error_diagnostic_code_step_budget_exhausted() {
    let error = CoreError::StepBudgetExhausted;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1201));
    assert_eq!(error.to_string(), "step budget exhausted");
}

#[test]
fn core_error_diagnostic_code_step_counter_overflow() {
    let error = CoreError::StepCounterOverflow;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1202));
    assert_eq!(error.to_string(), "step counter overflow");
}

#[test]
fn core_error_diagnostic_code_queue_full() {
    let error = CoreError::QueueFull;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1301));
    assert_eq!(error.to_string(), "queue full");
}

#[test]
fn core_error_diagnostic_code_resource_limit_exceeded() {
    let error = CoreError::ResourceLimitExceeded { resource: "memory" };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1302));
    assert_eq!(error.to_string(), "resource limit exceeded: memory");
}

#[test]
fn core_error_diagnostic_code_allocation_failed() {
    let error = CoreError::AllocationFailed;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1303));
    assert_eq!(error.to_string(), "allocation failed");
}

#[test]
fn core_error_diagnostic_code_expression_stack_overflow() {
    let error = CoreError::ExpressionStackOverflow { max: 64 };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1304));
    assert_eq!(error.to_string(), "expression stack overflow: max 64");
}

#[test]
fn core_error_diagnostic_code_expression_stack_underflow() {
    let error = CoreError::ExpressionStackUnderflow;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130B));
    assert_eq!(error.to_string(), "expression stack underflow");
}

#[test]
fn core_error_diagnostic_code_invalid_compiled_workflow() {
    let error = CoreError::InvalidCompiledWorkflow { reason: "bad node" };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1307));
    assert_eq!(error.to_string(), "invalid compiled workflow: bad node");
}

#[test]
fn core_error_diagnostic_code_unsupported_primitive() {
    let error = CoreError::UnsupportedPrimitive {
        primitive: "fancy_op",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1308));
    assert_eq!(error.to_string(), "unsupported primitive: fancy_op");
}

#[test]
fn core_error_diagnostic_code_unsupported_accessor_traversal() {
    let error = CoreError::UnsupportedAccessorTraversal {
        segment: "field",
        found: "list",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130A));
    assert_eq!(
        error.to_string(),
        "unsupported accessor traversal: field on list"
    );
}

#[test]
fn core_error_diagnostic_code_object_field_not_found() {
    let error = CoreError::ObjectFieldNotFound {
        field: SymbolId::new(42),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130C));
    assert_eq!(error.to_string(), "object field not found: SymbolId(42)");
}

#[test]
fn core_error_diagnostic_code_list_index_out_of_bounds() {
    let error = CoreError::ListIndexOutOfBounds { index: 10 };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130D));
    assert_eq!(error.to_string(), "list index out of bounds: 10");
}

#[test]
fn core_error_diagnostic_code_internal_invariant_violation() {
    let error = CoreError::InternalInvariantViolation {
        reason: "impossible",
    };
    // CV-105: relocated from 0x1309 to 0x1601 (Internal owns 0x16xx).
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1601));
    assert_eq!(
        error.to_string(),
        "internal invariant violation: impossible"
    );
}

#[test]
fn core_error_diagnostic_code_symbol_out_of_bounds() {
    let error = CoreError::SymbolOutOfBounds {
        symbol: SymbolId::new(100),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1311));
    assert_eq!(error.to_string(), "symbol id out of bounds: SymbolId(100)");
}

#[test]
fn core_error_diagnostic_code_list_out_of_bounds() {
    let error = CoreError::ListOutOfBounds {
        list: ListId::new(7),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1312));
    assert_eq!(error.to_string(), "list id out of bounds: ListId(7)");
}

#[test]
fn core_error_diagnostic_code_object_out_of_bounds() {
    let error = CoreError::ObjectOutOfBounds {
        object: ObjectId::new(3),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1313));
    assert_eq!(error.to_string(), "object id out of bounds: ObjectId(3)");
}

#[test]
fn core_error_diagnostic_code_blob_out_of_bounds() {
    let error = CoreError::BlobOutOfBounds {
        blob: BlobId::new(9),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1314));
    assert_eq!(error.to_string(), "blob id out of bounds: BlobId(9)");
}

#[test]
fn core_error_diagnostic_code_iteration_limit_exceeded() {
    let error = CoreError::IterationLimitExceeded {
        resource: "for_each",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1401));
    assert_eq!(error.to_string(), "iteration limit exceeded: for_each");
}

#[test]
fn core_error_diagnostic_code_repeat_exhausted() {
    let error = CoreError::RepeatExhausted { max: 5 };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1402));
    assert_eq!(error.to_string(), "repeat exhausted max attempts: 5");
}

#[test]
fn core_error_diagnostic_code_collect_page_limit_exceeded() {
    let error = CoreError::CollectPageLimitExceeded;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1403));
    assert_eq!(error.to_string(), "collect page limit exceeded");
}

#[test]
fn core_error_diagnostic_code_collect_item_limit_exceeded() {
    let error = CoreError::CollectItemLimitExceeded;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1404));
    assert_eq!(error.to_string(), "collect item limit exceeded");
}

#[test]
fn core_error_diagnostic_code_collect_time_limit_exceeded() {
    let error = CoreError::CollectTimeLimitExceeded;
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1407));
    assert_eq!(error.to_string(), "collect time limit exceeded");
}

#[test]
fn core_error_diagnostic_code_together_branch_limit_exceeded() {
    let error = CoreError::TogetherBranchLimitExceeded { max: 32 };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1405));
    assert_eq!(error.to_string(), "together branch limit exceeded: 32");
}

#[test]
fn core_error_diagnostic_code_budget_exceeded() {
    let error = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 1_024,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1406));
    assert_eq!(
        error.to_string(),
        "budget exceeded: max_slots limit was 1024"
    );
}

#[test]
fn core_error_runtime_codes_cover_section_17_core_mappings() {
    assert_eq!(
        CoreError::ConstOutOfBounds {
            index: ConstIdx::new(1)
        }
        .runtime_code(),
        Some("CONST_OUT_OF_BOUNDS")
    );
    assert_eq!(
        CoreError::TypeMismatch {
            expected: "list",
            found: "number",
        }
        .runtime_code(),
        Some("INPUT_TYPE_MISMATCH")
    );
    assert_eq!(
        CoreError::NonBoolCondition {
            slot: SlotIdx::new(1)
        }
        .runtime_code(),
        Some("INPUT_TYPE_MISMATCH")
    );
    assert_eq!(
        CoreError::MissingOutputSlot {
            step: StepIdx::new(2)
        }
        .runtime_code(),
        Some("MISSING_OUTPUT_SLOT")
    );
    assert_eq!(
        CoreError::StepStateOutOfBounds {
            step: StepIdx::new(3)
        }
        .runtime_code(),
        Some("STEP_STATE_OUT_OF_BOUNDS")
    );
    assert_eq!(
        CoreError::ExpressionStackOverflow { max: 4 }.runtime_code(),
        Some("EXPRESSION_STACK_OVERFLOW")
    );
    assert_eq!(
        CoreError::ExpressionStackUnderflow.runtime_code(),
        Some("EXPRESSION_STACK_UNDERFLOW")
    );
    assert_eq!(
        CoreError::InvalidCompiledWorkflow { reason: "bad" }.runtime_code(),
        Some("INVALID_COMPILED_WORKFLOW")
    );
    assert_eq!(
        CoreError::InternalInvariantViolation { reason: "bad" }.runtime_code(),
        Some("INTERNAL_INVARIANT_VIOLATION")
    );
    assert_eq!(
        CoreError::UnsupportedPrimitive { primitive: "op" }.runtime_code(),
        Some("UNSUPPORTED_PRIMITIVE")
    );
    assert_eq!(CoreError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
    assert_eq!(
        CoreError::RepeatExhausted { max: 3 }.runtime_code(),
        Some("REPEAT_LIMIT_REACHED")
    );
    assert_eq!(
        CoreError::CollectPageLimitExceeded.runtime_code(),
        Some("COLLECT_LIMIT_REACHED")
    );
    assert_eq!(
        CoreError::CollectItemLimitExceeded.runtime_code(),
        Some("COLLECT_LIMIT_REACHED")
    );
    assert_eq!(
        CoreError::CollectTimeLimitExceeded.runtime_code(),
        Some("COLLECT_LIMIT_REACHED")
    );
    assert_eq!(
        CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1_024,
        }
        .runtime_code(),
        Some("BUDGET_EXCEEDED")
    );
}

#[test]
fn core_error_runtime_codes_are_unique() {
    let codes = [
        CoreError::CONST_OUT_OF_BOUNDS_RUNTIME_CODE,
        CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE,
        CoreError::MISSING_OUTPUT_SLOT_RUNTIME_CODE,
        CoreError::STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE,
        CoreError::EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE,
        CoreError::EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE,
        CoreError::INVALID_COMPILED_WORKFLOW_RUNTIME_CODE,
        CoreError::INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE,
        CoreError::UNSUPPORTED_PRIMITIVE_RUNTIME_CODE,
        CoreError::QUEUE_FULL_RUNTIME_CODE,
        CoreError::REPEAT_LIMIT_REACHED_RUNTIME_CODE,
        CoreError::COLLECT_LIMIT_REACHED_RUNTIME_CODE,
        CoreError::BUDGET_EXCEEDED_RUNTIME_CODE,
    ];
    assert_eq!(codes.len(), 13);
    assert_eq!(
        codes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        13
    );
}

#[test]
fn core_error_runtime_code_is_absent_without_section_17_equivalent() {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(1),
    };
    assert_eq!(error.runtime_code(), None);
}

// -- exact variant field assertions for variants with fields --

#[test]
fn core_error_invalid_program_counter_exact_variant() -> Result<(), String> {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(42),
    };
    let CoreError::InvalidProgramCounter { step } = error else {
        return Err(String::from("expected InvalidProgramCounter variant"));
    };
    if step != StepIdx::new(42) {
        return Err(String::from("unexpected step"));
    }
    Ok(())
}

#[test]
fn core_error_missing_next_step_exact_variant() -> Result<(), String> {
    let error = CoreError::MissingNextStep {
        step: StepIdx::new(10),
    };
    let CoreError::MissingNextStep { step } = error else {
        return Err(String::from("expected MissingNextStep variant"));
    };
    if step != StepIdx::new(10) {
        return Err(String::from("unexpected step"));
    }
    Ok(())
}

#[test]
fn core_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(255),
    };
    let CoreError::SlotOutOfBounds { slot } = error else {
        return Err(String::from("expected SlotOutOfBounds variant"));
    };
    if slot != SlotIdx::new(255) {
        return Err(String::from("unexpected slot"));
    }
    Ok(())
}

#[test]
fn core_error_expr_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::ExprOutOfBounds {
        expr: ExprIdx::new(8),
    };
    let CoreError::ExprOutOfBounds { expr } = error else {
        return Err(String::from("expected ExprOutOfBounds variant"));
    };
    if expr != ExprIdx::new(8) {
        return Err(String::from("unexpected expr"));
    }
    Ok(())
}

#[test]
fn core_error_const_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::ConstOutOfBounds {
        index: ConstIdx::new(99),
    };
    let CoreError::ConstOutOfBounds { index } = error else {
        return Err(String::from("expected ConstOutOfBounds variant"));
    };
    if index != ConstIdx::new(99) {
        return Err(String::from("unexpected const index"));
    }
    Ok(())
}

#[test]
fn core_error_missing_output_slot_exact_variant() -> Result<(), String> {
    let error = CoreError::MissingOutputSlot {
        step: StepIdx::new(1),
    };
    let CoreError::MissingOutputSlot { step } = error else {
        return Err(String::from("expected MissingOutputSlot variant"));
    };
    if step != StepIdx::new(1) {
        return Err(String::from("unexpected step"));
    }
    Ok(())
}

#[test]
fn core_error_step_state_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::StepStateOutOfBounds {
        step: StepIdx::new(500),
    };
    let CoreError::StepStateOutOfBounds { step } = error else {
        return Err(String::from("expected StepStateOutOfBounds variant"));
    };
    if step != StepIdx::new(500) {
        return Err(String::from("unexpected step"));
    }
    Ok(())
}

#[test]
fn core_error_type_mismatch_exact_variant() -> Result<(), String> {
    let error = CoreError::TypeMismatch {
        expected: "i64",
        found: "bool",
    };
    let CoreError::TypeMismatch { expected, found } = error else {
        return Err(String::from("expected TypeMismatch variant"));
    };
    if expected != "i64" || found != "bool" {
        return Err(String::from("unexpected type mismatch fields"));
    }
    Ok(())
}

#[test]
fn core_error_non_bool_condition_exact_variant() -> Result<(), String> {
    let error = CoreError::NonBoolCondition {
        slot: SlotIdx::new(3),
    };
    let CoreError::NonBoolCondition { slot } = error else {
        return Err(String::from("expected NonBoolCondition variant"));
    };
    if slot != SlotIdx::new(3) {
        return Err(String::from("unexpected slot"));
    }
    Ok(())
}

#[test]
fn core_error_resource_limit_exceeded_exact_variant() -> Result<(), String> {
    let error = CoreError::ResourceLimitExceeded { resource: "slots" };
    let CoreError::ResourceLimitExceeded { resource } = error else {
        return Err(String::from("expected ResourceLimitExceeded variant"));
    };
    if resource != "slots" {
        return Err(String::from("unexpected resource"));
    }
    Ok(())
}

#[test]
fn core_error_expression_stack_overflow_exact_variant() -> Result<(), String> {
    let error = CoreError::ExpressionStackOverflow { max: 128 };
    let CoreError::ExpressionStackOverflow { max } = error else {
        return Err(String::from("expected ExpressionStackOverflow variant"));
    };
    if max != 128 {
        return Err(String::from("unexpected max"));
    }
    Ok(())
}

#[test]
fn core_error_invalid_compiled_workflow_exact_variant() -> Result<(), String> {
    let error = CoreError::InvalidCompiledWorkflow {
        reason: "missing entry",
    };
    let CoreError::InvalidCompiledWorkflow { reason } = error else {
        return Err(String::from("expected InvalidCompiledWorkflow variant"));
    };
    if reason != "missing entry" {
        return Err(String::from("unexpected reason"));
    }
    Ok(())
}

#[test]
fn core_error_unsupported_primitive_exact_variant() -> Result<(), String> {
    let error = CoreError::UnsupportedPrimitive {
        primitive: "async_await",
    };
    let CoreError::UnsupportedPrimitive { primitive } = error else {
        return Err(String::from("expected UnsupportedPrimitive variant"));
    };
    if primitive != "async_await" {
        return Err(String::from("unexpected primitive"));
    }
    Ok(())
}

#[test]
fn core_error_unsupported_accessor_traversal_exact_variant() -> Result<(), String> {
    let error = CoreError::UnsupportedAccessorTraversal {
        segment: "index",
        found: "object",
    };
    let CoreError::UnsupportedAccessorTraversal { segment, found } = error else {
        return Err(String::from(
            "expected UnsupportedAccessorTraversal variant",
        ));
    };
    if segment != "index" || found != "object" {
        return Err(String::from("unexpected accessor traversal fields"));
    }
    Ok(())
}

#[test]
fn core_error_object_field_not_found_exact_variant() -> Result<(), String> {
    let error = CoreError::ObjectFieldNotFound {
        field: SymbolId::new(7),
    };
    let CoreError::ObjectFieldNotFound { field } = error else {
        return Err(String::from("expected ObjectFieldNotFound variant"));
    };
    if field != SymbolId::new(7) {
        return Err(String::from("unexpected field"));
    }
    Ok(())
}

#[test]
fn core_error_list_index_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::ListIndexOutOfBounds { index: 999 };
    let CoreError::ListIndexOutOfBounds { index } = error else {
        return Err(String::from("expected ListIndexOutOfBounds variant"));
    };
    if index != 999 {
        return Err(String::from("unexpected index"));
    }
    Ok(())
}

#[test]
fn core_error_internal_invariant_violation_exact_variant() -> Result<(), String> {
    let error = CoreError::InternalInvariantViolation {
        reason: "corrupted",
    };
    let CoreError::InternalInvariantViolation { reason } = error else {
        return Err(String::from("expected InternalInvariantViolation variant"));
    };
    if reason != "corrupted" {
        return Err(String::from("unexpected reason"));
    }
    Ok(())
}

#[test]
fn core_error_symbol_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::SymbolOutOfBounds {
        symbol: SymbolId::new(55),
    };
    let CoreError::SymbolOutOfBounds { symbol } = error else {
        return Err(String::from("expected SymbolOutOfBounds variant"));
    };
    if symbol != SymbolId::new(55) {
        return Err(String::from("unexpected symbol"));
    }
    Ok(())
}

#[test]
fn core_error_list_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::ListOutOfBounds {
        list: ListId::new(33),
    };
    let CoreError::ListOutOfBounds { list } = error else {
        return Err(String::from("expected ListOutOfBounds variant"));
    };
    if list != ListId::new(33) {
        return Err(String::from("unexpected list"));
    }
    Ok(())
}

#[test]
fn core_error_object_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::ObjectOutOfBounds {
        object: ObjectId::new(21),
    };
    let CoreError::ObjectOutOfBounds { object } = error else {
        return Err(String::from("expected ObjectOutOfBounds variant"));
    };
    if object != ObjectId::new(21) {
        return Err(String::from("unexpected object"));
    }
    Ok(())
}

#[test]
fn core_error_blob_out_of_bounds_exact_variant() -> Result<(), String> {
    let error = CoreError::BlobOutOfBounds {
        blob: BlobId::new(11),
    };
    let CoreError::BlobOutOfBounds { blob } = error else {
        return Err(String::from("expected BlobOutOfBounds variant"));
    };
    if blob != BlobId::new(11) {
        return Err(String::from("unexpected blob"));
    }
    Ok(())
}

#[test]
fn core_error_iteration_limit_exceeded_exact_variant() -> Result<(), String> {
    let error = CoreError::IterationLimitExceeded {
        resource: "collect",
    };
    let CoreError::IterationLimitExceeded { resource } = error else {
        return Err(String::from("expected IterationLimitExceeded variant"));
    };
    if resource != "collect" {
        return Err(String::from("unexpected resource"));
    }
    Ok(())
}

#[test]
fn core_error_repeat_exhausted_exact_variant() -> Result<(), String> {
    let error = CoreError::RepeatExhausted { max: 10 };
    let CoreError::RepeatExhausted { max } = error else {
        return Err(String::from("expected RepeatExhausted variant"));
    };
    if max != 10 {
        return Err(String::from("unexpected max"));
    }
    Ok(())
}

#[test]
fn core_error_together_branch_limit_exceeded_exact_variant() -> Result<(), String> {
    let error = CoreError::TogetherBranchLimitExceeded { max: 64 };
    let CoreError::TogetherBranchLimitExceeded { max } = error else {
        return Err(String::from("expected TogetherBranchLimitExceeded variant"));
    };
    if max != 64 {
        return Err(String::from("unexpected max"));
    }
    Ok(())
}

#[test]
fn core_error_budget_exceeded_exact_variant() -> Result<(), String> {
    let error = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 1_024,
    };
    let CoreError::BudgetExceeded { budget, limit } = error else {
        return Err(String::from("expected BudgetExceeded variant"));
    };
    if budget != "max_slots" {
        return Err(String::from("unexpected budget name"));
    }
    if limit != 1_024 {
        return Err(String::from("unexpected limit"));
    }
    Ok(())
}

// =========================================================================
// Edge-case tests -- EngineError display, runtime_code mappings,
// equality, and boundary variants
// =========================================================================

#[test]
fn engine_error_is_core_error_alias() {
    // EngineError is documented as a backward-compatible alias for CoreError.
    let error: EngineError = CoreError::DivisionByZero;
    assert_eq!(error, CoreError::DivisionByZero);
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1103));
}

#[test]
fn engine_error_slot_uninitialized_display() {
    let error = CoreError::SlotUninitialized {
        slot: SlotIdx::new(7),
    };
    let msg = error.to_string();
    assert!(
        msg.contains("slot not initialized"),
        "display must contain 'slot not initialized', got: {msg}"
    );
    assert!(
        msg.contains("SlotIdx(7)"),
        "display must contain slot index, got: {msg}"
    );
}

#[test]
fn engine_error_step_budget_exhausted_display() {
    let error = CoreError::StepBudgetExhausted;
    assert_eq!(error.to_string(), "step budget exhausted");
}

#[test]
fn engine_error_queue_full_display() {
    let error = CoreError::QueueFull;
    assert_eq!(error.to_string(), "queue full");
}

#[test]
fn engine_error_non_finite_number_display() {
    let error = CoreError::NonFiniteNumber;
    assert_eq!(error.to_string(), "non-finite number is not allowed");
}

#[test]
fn engine_error_allocation_failed_display() {
    let error = CoreError::AllocationFailed;
    assert_eq!(error.to_string(), "allocation failed");
}

#[test]
fn engine_error_runtime_code_capability_denied() {
    use crate::capability::{Capability, CapabilitySet};
    let cap = Capability::new(String::from("file_read").into_boxed_str(), ActionId::new(1));
    let error = CoreError::CapabilityDenied {
        action: ActionId::new(1),
        required: cap,
        granted: CapabilitySet::empty(),
    };
    assert_eq!(error.runtime_code(), Some("CAPABILITY_DENIED"));
}

#[test]
fn engine_error_runtime_code_parallel_limit_exceeded() {
    let error = CoreError::ParallelLimitExceeded { limit: 10 };
    // ParallelLimitExceeded does not have a direct runtime_code mapping.
    assert_eq!(error.runtime_code(), None);
}

#[test]
fn engine_error_runtime_code_together_branch_limit_exceeded() {
    let error = CoreError::TogetherBranchLimitExceeded { max: 8 };
    // TogetherBranchLimitExceeded does not have a direct runtime_code mapping.
    assert_eq!(error.runtime_code(), None);
}

#[test]
fn engine_error_equality_same_variant() {
    let a = CoreError::DivisionByZero;
    let b = CoreError::DivisionByZero;
    assert_eq!(a, b);
}

#[test]
fn engine_error_inequality_different_variants() {
    let a = CoreError::DivisionByZero;
    let b = CoreError::NonFiniteNumber;
    assert_ne!(a, b);
}

#[test]
fn engine_error_budget_exceeded_display_contains_both_fields() {
    let error = CoreError::BudgetExceeded {
        budget: "memory",
        limit: 512,
    };
    let msg = error.to_string();
    assert!(
        msg.contains("memory"),
        "display must contain budget name, got: {msg}"
    );
    assert!(
        msg.contains("512"),
        "display must contain limit, got: {msg}"
    );
}

#[test]
fn engine_error_resource_limit_exceeded_display() {
    let error = CoreError::ResourceLimitExceeded {
        resource: "connections",
    };
    let msg = error.to_string();
    assert!(
        msg.contains("connections"),
        "display must contain resource name, got: {msg}"
    );
}

#[test]
fn engine_error_expression_stack_overflow_display_contains_max() {
    let error = CoreError::ExpressionStackOverflow { max: 32 };
    let msg = error.to_string();
    assert!(
        msg.contains("32"),
        "display must contain max value, got: {msg}"
    );
}

#[test]
fn engine_error_type_mismatch_equality() {
    let a = CoreError::TypeMismatch {
        expected: "list",
        found: "number",
    };
    let b = CoreError::TypeMismatch {
        expected: "list",
        found: "number",
    };
    assert_eq!(a, b);
}

#[test]
fn engine_error_type_mismatch_inequality() {
    let a = CoreError::TypeMismatch {
        expected: "list",
        found: "number",
    };
    let b = CoreError::TypeMismatch {
        expected: "bool",
        found: "number",
    };
    assert_ne!(a, b);
}

#[test]
fn engine_error_runtime_code_repeat_exhausted() {
    let error = CoreError::RepeatExhausted { max: 3 };
    assert_eq!(error.runtime_code(), Some("REPEAT_LIMIT_REACHED"));
}

#[test]
fn engine_error_diagnostic_code_slot_uninitialized() {
    let error = CoreError::SlotUninitialized {
        slot: SlotIdx::new(0),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1012));
}

// -- Missing exact variant tests from review --

#[test]
fn core_error_budget_parse_exact_variant() {
    let error = CoreError::BudgetParse {
        reason: "invalid u64 value",
    };
    let CoreError::BudgetParse { reason } = error else {
        panic!("expected BudgetParse variant");
    };
    assert_eq!(reason, "invalid u64 value");
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x140A));
}

#[test]
fn core_error_collect_page_order_violation_exact_variant() {
    let error = CoreError::CollectPageOrderViolation {
        kind: CollectPageOrderViolationKind::OutOfOrder,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        expected_page: ListId::new(3),
        observed_page: ListId::new(4),
    };
    let CoreError::CollectPageOrderViolation {
        kind,
        run_id,
        collector_slot,
        expected_page,
        observed_page,
    } = error
    else {
        panic!("expected CollectPageOrderViolation variant");
    };
    assert_eq!(kind, CollectPageOrderViolationKind::OutOfOrder);
    assert_eq!(run_id, RunId::new(1));
    assert_eq!(collector_slot, SlotIdx::new(2));
    assert_eq!(expected_page, ListId::new(3));
    assert_eq!(observed_page, ListId::new(4));
}

#[test]
fn core_error_collect_extra_hydration_failed_exact_variant() {
    let error = CoreError::CollectExtraHydrationFailed {
        kind: CollectExtraHydrationFailureKind::EmptyExtra,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        event_seq: None,
    };
    let CoreError::CollectExtraHydrationFailed {
        kind,
        run_id,
        collector_slot,
        event_seq,
    } = error
    else {
        panic!("expected CollectExtraHydrationFailed variant");
    };
    assert_eq!(kind, CollectExtraHydrationFailureKind::EmptyExtra);
    assert_eq!(run_id, RunId::new(1));
    assert_eq!(collector_slot, SlotIdx::new(2));
    assert_eq!(event_seq, None);
}

#[test]
fn core_error_collect_evidence_capacity_exceeded_exact_variant() {
    let error = CoreError::CollectEvidenceCapacityExceeded {
        run_id: RunId::new(1),
        slot: SlotIdx::new(2),
        capacity: 100,
        len: 200,
        required: "extra slots",
    };
    let CoreError::CollectEvidenceCapacityExceeded {
        run_id,
        slot,
        capacity,
        len,
        required,
    } = error
    else {
        panic!("expected CollectEvidenceCapacityExceeded variant");
    };
    assert_eq!(run_id, RunId::new(1));
    assert_eq!(slot, SlotIdx::new(2));
    assert_eq!(capacity, 100);
    assert_eq!(len, 200);
    assert_eq!(required, "extra slots");
}

#[test]
fn core_error_lifecycle_storage_unavailable_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::LifecycleStorageUnavailable {
        code: DiagnosticCode::new(0x1501),
        context: String::from("disk full"),
        timestamp: ts,
        bead_id: Some(RunId::new(1)),
    };
    let CoreError::LifecycleStorageUnavailable {
        code,
        context,
        timestamp,
        bead_id,
    } = error
    else {
        panic!("expected LifecycleStorageUnavailable variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1501));
    assert_eq!(context, "disk full");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, Some(RunId::new(1)));
}

#[test]
fn core_error_lifecycle_duplicate_request_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::LifecycleDuplicateRequest {
        code: DiagnosticCode::new(0x1502),
        context: String::from("dup"),
        timestamp: ts,
        bead_id: None,
        command: Some("run"),
    };
    let CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp,
        bead_id,
        command,
    } = error
    else {
        panic!("expected LifecycleDuplicateRequest variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1502));
    assert_eq!(context, "dup");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, None);
    assert_eq!(command, Some("run"));
}

#[test]
fn core_error_lifecycle_stale_request_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::LifecycleStaleRequest {
        code: DiagnosticCode::new(0x1503),
        context: String::from("stale"),
        timestamp: ts,
        bead_id: Some(RunId::new(2)),
        command: None,
    };
    let CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp,
        bead_id,
        command,
    } = error
    else {
        panic!("expected LifecycleStaleRequest variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1503));
    assert_eq!(context, "stale");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, Some(RunId::new(2)));
    assert_eq!(command, None);
}

#[test]
fn core_error_lifecycle_invalid_transition_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::LifecycleInvalidTransition {
        code: DiagnosticCode::new(0x1504),
        context: String::from("bad transition"),
        timestamp: ts,
        bead_id: None,
        command: Some("step"),
    };
    let CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp,
        bead_id,
        command,
    } = error
    else {
        panic!("expected LifecycleInvalidTransition variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1504));
    assert_eq!(context, "bad transition");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, None);
    assert_eq!(command, Some("step"));
}

#[test]
fn core_error_journal_write_failure_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::JournalWriteFailure {
        code: DiagnosticCode::new(0x1505),
        context: String::from("io error"),
        timestamp: ts,
        bead_id: Some(RunId::new(3)),
    };
    let CoreError::JournalWriteFailure {
        code,
        context,
        timestamp,
        bead_id,
    } = error
    else {
        panic!("expected JournalWriteFailure variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1505));
    assert_eq!(context, "io error");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, Some(RunId::new(3)));
}

#[test]
fn core_error_replay_corruption_exact_variant() {
    let ts = Utc::now();
    let error = CoreError::ReplayCorruption {
        code: DiagnosticCode::new(0x1506),
        context: String::from("checksum mismatch"),
        timestamp: ts,
        bead_id: None,
    };
    let CoreError::ReplayCorruption {
        code,
        context,
        timestamp,
        bead_id,
    } = error
    else {
        panic!("expected ReplayCorruption variant");
    };
    assert_eq!(code, DiagnosticCode::new(0x1506));
    assert_eq!(context, "checksum mismatch");
    assert_eq!(timestamp, ts);
    assert_eq!(bead_id, None);
}

#[test]
fn core_error_diagnostic_code_budget_parse() {
    let error = CoreError::BudgetParse {
        reason: "bad value",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x140A));
}

#[test]
fn core_error_diagnostic_code_collect_page_order_violation() {
    let error = CoreError::CollectPageOrderViolation {
        kind: CollectPageOrderViolationKind::Duplicate,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        expected_page: ListId::new(0),
        observed_page: ListId::new(0),
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x140B));
}

#[test]
fn core_error_diagnostic_code_collect_extra_hydration_failed() {
    let error = CoreError::CollectExtraHydrationFailed {
        kind: CollectExtraHydrationFailureKind::DecodeFailed,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        event_seq: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x140C));
}

#[test]
fn core_error_diagnostic_code_collect_evidence_capacity_exceeded() {
    let error = CoreError::CollectEvidenceCapacityExceeded {
        run_id: RunId::new(1),
        slot: SlotIdx::new(0),
        capacity: 1,
        len: 2,
        required: "test",
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x140D));
}

#[test]
fn core_error_diagnostic_code_lifecycle_storage_unavailable() {
    let error = CoreError::LifecycleStorageUnavailable {
        code: DiagnosticCode::new(0x1501),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1501));
}

#[test]
fn core_error_diagnostic_code_lifecycle_duplicate_request() {
    let error = CoreError::LifecycleDuplicateRequest {
        code: DiagnosticCode::new(0x1502),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
        command: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1502));
}

#[test]
fn core_error_diagnostic_code_lifecycle_stale_request() {
    let error = CoreError::LifecycleStaleRequest {
        code: DiagnosticCode::new(0x1503),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
        command: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1503));
}

#[test]
fn core_error_diagnostic_code_lifecycle_invalid_transition() {
    let error = CoreError::LifecycleInvalidTransition {
        code: DiagnosticCode::new(0x1504),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
        command: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1504));
}

#[test]
fn core_error_diagnostic_code_journal_write_failure() {
    let error = CoreError::JournalWriteFailure {
        code: DiagnosticCode::new(0x1505),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1505));
}

#[test]
fn core_error_diagnostic_code_replay_corruption() {
    let error = CoreError::ReplayCorruption {
        code: DiagnosticCode::new(0x1506),
        context: String::new(),
        timestamp: Utc::now(),
        bead_id: None,
    };
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1506));
}

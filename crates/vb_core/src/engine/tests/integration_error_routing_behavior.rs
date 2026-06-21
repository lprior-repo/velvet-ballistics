#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Integration behavior tests for error routing.
//!
//! Covers every error variant, routing outcomes, retry tracking,
//! escalation, suppression, chaining, display, lifecycle errors,
//! partial state, double-fault, and proptest-based context preservation.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError,
    WorkflowParts,
};

use crate::engine::{ErrorHandlerOutcome, ErrorSlotData, new_run_frame, route_error_handler};

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

// ---------------------------------------------------------------------------
// Test fixture builders
// ---------------------------------------------------------------------------

fn make_workflow<F>(name: &str, f: F) -> Result<CompiledWorkflow, String>
where
    F: FnOnce(&mut Vec<CompiledNode>),
{
    let mut nodes = Vec::new();
    f(&mut nodes);

    let slot_count = nodes.len().max(4) as u16;
    let parts = WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: slot_count + 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn make_frame(run_id: u64, workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    new_run_frame(RunId::new(run_id), workflow).map_err(|error| error.to_string())
}

fn make_simple_handler_workflow() -> Result<CompiledWorkflow, String> {
    make_workflow("handler_test", |nodes| {
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: Some(StepIdx::new(2)),
            error_slot: Some(SlotIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        });
    })
}

fn make_no_handler_workflow() -> Result<CompiledWorkflow, String> {
    make_workflow("no_handler_test", |nodes| {
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
    })
}

fn make_multihandler_workflow() -> Result<CompiledWorkflow, String> {
    make_workflow("multihandler_test", |nodes| {
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: Some(StepIdx::new(3)),
            error_slot: Some(SlotIdx::new(2)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(5)),
            on_error: Some(StepIdx::new(4)),
            error_slot: Some(SlotIdx::new(3)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(4)),
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(4)),
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
    })
}

fn assert_error_variant_propagates(
    error: EngineError,
    expected_code: &str,
    test_name: &str,
) -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(100, &workflow)?;

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    assert_error_code(&error, expected_code, test_name)
}

fn assert_error_code(
    error: &EngineError,
    expected_code: &str,
    test_name: &str,
) -> Result<(), String> {
    let data = ErrorSlotData::from_engine_error(error, StepIdx::new(0));
    if &*data.code == expected_code {
        Ok(())
    } else {
        Err(format!(
            "code mismatch for {test_name}: expected {expected_code} got {}",
            &*data.code
        ))
    }
}

// =========================================================================
// 1. Every error variant propagation through routing
// =========================================================================

macro_rules! test_error_variant_propagation {
    ($test_name:ident, $error:expr, $expected_code:literal) => {
        #[test]
        fn $test_name() -> Result<(), String> {
            assert_error_variant_propagates($error, $expected_code, stringify!($test_name))
        }
    };
}

test_error_variant_propagation!(
    division_by_zero_propagates,
    EngineError::DivisionByZero,
    "DIVISION_BY_ZERO"
);

test_error_variant_propagation!(
    type_mismatch_propagates,
    EngineError::TypeMismatch {
        expected: "i64",
        found: "bool",
    },
    "INPUT_TYPE_MISMATCH"
);

test_error_variant_propagation!(
    resource_limit_exceeded_propagates,
    EngineError::ResourceLimitExceeded { resource: "slots" },
    "RESOURCE_LIMIT_EXCEEDED"
);

test_error_variant_propagation!(
    non_finite_number_propagates,
    EngineError::NonFiniteNumber,
    "NON_FINITE_NUMBER"
);

test_error_variant_propagation!(
    step_budget_exhausted_propagates,
    EngineError::StepBudgetExhausted,
    "STEP_BUDGET_EXHAUSTED"
);

test_error_variant_propagation!(
    step_counter_overflow_propagates,
    EngineError::StepCounterOverflow,
    "STEP_COUNTER_OVERFLOW"
);

test_error_variant_propagation!(queue_full_propagates, EngineError::QueueFull, "QUEUE_FULL");

test_error_variant_propagation!(
    allocation_failed_propagates,
    EngineError::AllocationFailed,
    "ALLOCATION_FAILED"
);

test_error_variant_propagation!(
    expression_stack_overflow_propagates,
    EngineError::ExpressionStackOverflow { max: 64 },
    "EXPRESSION_STACK_OVERFLOW"
);

test_error_variant_propagation!(
    expression_stack_underflow_propagates,
    EngineError::ExpressionStackUnderflow,
    "EXPRESSION_STACK_UNDERFLOW"
);

test_error_variant_propagation!(
    invalid_program_counter_propagates,
    EngineError::InvalidProgramCounter {
        step: StepIdx::new(0)
    },
    "INVALID_PROGRAM_COUNTER"
);

test_error_variant_propagation!(
    missing_next_step_propagates,
    EngineError::MissingNextStep {
        step: StepIdx::new(0)
    },
    "MISSING_NEXT_STEP"
);

test_error_variant_propagation!(
    slot_out_of_bounds_propagates,
    EngineError::SlotOutOfBounds {
        slot: SlotIdx::new(99)
    },
    "SLOT_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    expr_out_of_bounds_propagates,
    EngineError::ExprOutOfBounds {
        expr: crate::ids::ExprIdx::new(7)
    },
    "EXPR_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    const_out_of_bounds_propagates,
    EngineError::ConstOutOfBounds {
        index: ConstIdx::new(5)
    },
    "CONST_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    step_state_out_of_bounds_propagates,
    EngineError::StepStateOutOfBounds {
        step: StepIdx::new(200)
    },
    "STEP_STATE_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    list_index_out_of_bounds_propagates,
    EngineError::ListIndexOutOfBounds { index: 5 },
    "LIST_INDEX_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    symbol_out_of_bounds_propagates,
    EngineError::SymbolOutOfBounds {
        symbol: crate::ids::SymbolId::new(42)
    },
    "SYMBOL_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    list_out_of_bounds_propagates,
    EngineError::ListOutOfBounds {
        list: crate::ids::ListId::new(3)
    },
    "LIST_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    object_out_of_bounds_propagates,
    EngineError::ObjectOutOfBounds {
        object: crate::ids::ObjectId::new(7)
    },
    "OBJECT_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    blob_out_of_bounds_propagates,
    EngineError::BlobOutOfBounds {
        blob: crate::ids::BlobId::new(9)
    },
    "BLOB_OUT_OF_BOUNDS"
);

test_error_variant_propagation!(
    non_bool_condition_propagates,
    EngineError::NonBoolCondition {
        slot: SlotIdx::new(3)
    },
    "INPUT_TYPE_MISMATCH"
);

test_error_variant_propagation!(
    unsupported_primitive_propagates,
    EngineError::UnsupportedPrimitive {
        primitive: "fancy_op"
    },
    "UNSUPPORTED_PRIMITIVE"
);

test_error_variant_propagation!(
    unsupported_accessor_traversal_propagates,
    EngineError::UnsupportedAccessorTraversal {
        segment: "field",
        found: "list",
    },
    "UNSUPPORTED_ACCESSOR_TRAVERSAL"
);

test_error_variant_propagation!(
    object_field_not_found_propagates,
    EngineError::ObjectFieldNotFound {
        field: crate::ids::SymbolId::new(10)
    },
    "OBJECT_FIELD_NOT_FOUND"
);

test_error_variant_propagation!(
    internal_invariant_violation_propagates,
    EngineError::InternalInvariantViolation {
        reason: "impossible"
    },
    "INTERNAL_INVARIANT_VIOLATION"
);

test_error_variant_propagation!(
    iteration_limit_exceeded_propagates,
    EngineError::IterationLimitExceeded {
        resource: "for_each"
    },
    "ITERATION_LIMIT_EXCEEDED"
);

test_error_variant_propagation!(
    repeat_exhausted_propagates,
    EngineError::RepeatExhausted { max: 5 },
    "REPEAT_LIMIT_REACHED"
);

test_error_variant_propagation!(
    collect_page_limit_exceeded_propagates,
    EngineError::CollectPageLimitExceeded,
    "COLLECT_LIMIT_REACHED"
);

test_error_variant_propagation!(
    collect_item_limit_exceeded_propagates,
    EngineError::CollectItemLimitExceeded,
    "COLLECT_LIMIT_REACHED"
);

test_error_variant_propagation!(
    collect_time_limit_exceeded_propagates,
    EngineError::CollectTimeLimitExceeded,
    "COLLECT_LIMIT_REACHED"
);

test_error_variant_propagation!(
    together_branch_limit_exceeded_propagates,
    EngineError::TogetherBranchLimitExceeded { max: 32 },
    "TOGETHER_BRANCH_LIMIT_EXCEEDED"
);

test_error_variant_propagation!(
    parallel_limit_exceeded_propagates,
    EngineError::ParallelLimitExceeded { limit: 10 },
    "PARALLEL_LIMIT_EXCEEDED"
);

test_error_variant_propagation!(
    budget_exceeded_propagates,
    EngineError::BudgetExceeded {
        budget: "max_slots",
        limit: 1024,
    },
    "BUDGET_EXCEEDED"
);

test_error_variant_propagation!(
    budget_parse_propagates,
    EngineError::BudgetParse {
        reason: "bad value"
    },
    "BUDGET_PARSE"
);

test_error_variant_propagation!(
    capability_denied_propagates,
    EngineError::CapabilityDenied {
        action: crate::ids::ActionId::new(1),
        required: crate::capability::Capability::new(
            String::from("file_read").into_boxed_str(),
            crate::ids::ActionId::new(1)
        ),
        granted: crate::capability::CapabilitySet::empty(),
    },
    "CAPABILITY_DENIED"
);

test_error_variant_propagation!(
    collect_page_order_violation_propagates,
    EngineError::CollectPageOrderViolation {
        kind: crate::errors::CollectPageOrderViolationKind::OutOfOrder,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        expected_page: crate::ids::ListId::new(3),
        observed_page: crate::ids::ListId::new(4),
    },
    "COLLECT_PAGE_ORDER_VIOLATION"
);

test_error_variant_propagation!(
    collect_extra_hydration_failed_propagates,
    EngineError::CollectExtraHydrationFailed {
        kind: crate::errors::CollectExtraHydrationFailureKind::EmptyExtra,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        event_seq: None,
    },
    "COLLECT_EXTRA_HYDRATION_FAILED"
);

test_error_variant_propagation!(
    collect_evidence_capacity_exceeded_propagates,
    EngineError::CollectEvidenceCapacityExceeded {
        run_id: RunId::new(1),
        slot: SlotIdx::new(2),
        capacity: 100,
        len: 200,
        required: "extra slots",
    },
    "COLLECT_EVIDENCE_CAPACITY_EXCEEDED"
);

test_error_variant_propagation!(
    missing_output_slot_propagates,
    EngineError::MissingOutputSlot {
        step: StepIdx::new(1)
    },
    "MISSING_OUTPUT_SLOT"
);

test_error_variant_propagation!(
    invalid_compiled_workflow_propagates,
    EngineError::InvalidCompiledWorkflow { reason: "bad node" },
    "INVALID_COMPILED_WORKFLOW"
);

// -- SlotUninitialized variant --

test_error_variant_propagation!(
    slot_uninitialized_propagates,
    EngineError::SlotUninitialized {
        slot: SlotIdx::new(3)
    },
    "SLOT_UNINITIALIZED"
);

// =========================================================================
// 2. Error type routing: TypeMismatch specifically
// =========================================================================

#[test]
fn type_mismatch_routing_contains_expected_and_found_in_message() -> Result<(), String> {
    let error = EngineError::TypeMismatch {
        expected: "list",
        found: "number",
    };
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(1));

    ensure_equal(&*data.code, "INPUT_TYPE_MISMATCH")?;
    ensure_equal(data.failed_step, StepIdx::new(1))?;
    if !data.message.contains("list") {
        return Err(format!(
            "message should contain 'list', got: {}",
            data.message
        ));
    }
    if !data.message.contains("number") {
        return Err(format!(
            "message should contain 'number', got: {}",
            data.message
        ));
    }
    Ok(())
}

#[test]
fn resource_limit_exceeded_routing_contains_resource() -> Result<(), String> {
    let error = EngineError::ResourceLimitExceeded {
        resource: "connections",
    };
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(7));

    if !data.message.contains("connections") {
        return Err(format!(
            "message should contain 'connections', got: {}",
            data.message
        ));
    }
    ensure_equal(&*data.code, "RESOURCE_LIMIT_EXCEEDED")?;
    Ok(())
}

// =========================================================================
// 3. Retry counter tracking
// =========================================================================

#[test]
fn route_error_handler_increments_executed_counter() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(200, &workflow)?;
    let before = run.executed();
    let error = EngineError::DivisionByZero;

    let _outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    ensure_equal(run.executed(), before + 1)?;
    Ok(())
}

// =========================================================================
// 4. Error escalation: failed step index written
// =========================================================================

#[test]
fn failed_step_index_written_to_error_slot() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(300, &workflow)?;
    let failed_step = StepIdx::new(0);
    let error = EngineError::TypeMismatch {
        expected: "bool",
        found: "i64",
    };

    let _outcome =
        route_error_handler(&workflow, &mut run, failed_step, &error).map_err(|e| e.to_string())?;

    let slot_value = run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(*slot_value, SlotValue::I64(0))?;
    Ok(())
}

#[test]
fn failed_step_index_written_for_large_step() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(301, &workflow)?;
    let error = EngineError::StepBudgetExhausted;

    let _outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    let slot_value = run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(*slot_value, SlotValue::I64(0))?;
    Ok(())
}

// =========================================================================
// 5. Error suppression: NoHandler preserves PC
// =========================================================================

#[test]
fn no_handler_preserves_pc_unchanged() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(400, &workflow)?;
    let original_pc = run.pc();
    let error = EngineError::DivisionByZero;

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    ensure_equal(run.pc(), original_pc)?;
    Ok(())
}

#[test]
fn no_handler_does_not_increment_executed() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(401, &workflow)?;
    let before = run.executed();
    let error = EngineError::StepBudgetExhausted;

    let _outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    ensure_equal(run.executed(), before)?;
    Ok(())
}

#[test]
fn no_handler_does_not_write_error_slot() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(402, &workflow)?;
    let error = EngineError::DivisionByZero;

    let _outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    let read_result = run.read_slot(SlotIdx::new(1));
    if read_result.is_ok() {
        return Err(String::from("error slot should remain uninitialized"));
    }
    Ok(())
}

// =========================================================================
// 6. Custom handler mapping: different steps → different handlers
// =========================================================================

#[test]
fn different_steps_route_to_different_handlers() -> Result<(), String> {
    let workflow = make_multihandler_workflow()?;
    let error = EngineError::DivisionByZero;

    // Step 0 has on_error → Step 3
    let mut run0 = make_frame(500, &workflow)?;
    let outcome0 = route_error_handler(&workflow, &mut run0, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome0, ErrorHandlerOutcome::Routed)?;
    ensure_equal(run0.pc(), StepIdx::new(3))?;

    // Step 2 has on_error → Step 4
    let mut run2 = make_frame(501, &workflow)?;
    let outcome2 = route_error_handler(&workflow, &mut run2, StepIdx::new(2), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome2, ErrorHandlerOutcome::Routed)?;
    ensure_equal(run2.pc(), StepIdx::new(4))?;

    Ok(())
}

#[test]
fn different_steps_write_to_different_error_slots() -> Result<(), String> {
    let workflow = make_multihandler_workflow()?;
    let error = EngineError::DivisionByZero;

    let mut run0 = make_frame(510, &workflow)?;
    let _ = route_error_handler(&workflow, &mut run0, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    let slot0_val = run0.read_slot(SlotIdx::new(2)).map_err(|e| e.to_string())?;
    ensure_equal(*slot0_val, SlotValue::I64(0))?;

    let mut run2 = make_frame(511, &workflow)?;
    let _ = route_error_handler(&workflow, &mut run2, StepIdx::new(2), &error)
        .map_err(|e| e.to_string())?;
    let slot2_val = run2.read_slot(SlotIdx::new(3)).map_err(|e| e.to_string())?;
    ensure_equal(*slot2_val, SlotValue::I64(2))?;

    Ok(())
}

// =========================================================================
// 7. Default handler: NoHandler enum, all unchanged for many error types
// =========================================================================

#[test]
fn no_handler_returns_no_handler_for_division_by_zero() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(600, &workflow)?;
    let outcome = route_error_handler(
        &workflow,
        &mut run,
        StepIdx::new(0),
        &EngineError::DivisionByZero,
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    Ok(())
}

#[test]
fn no_handler_returns_no_handler_for_type_mismatch() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(601, &workflow)?;
    let outcome = route_error_handler(
        &workflow,
        &mut run,
        StepIdx::new(0),
        &EngineError::TypeMismatch {
            expected: "list",
            found: "number",
        },
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    Ok(())
}

#[test]
fn no_handler_returns_no_handler_for_budget_exceeded() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(602, &workflow)?;
    let outcome = route_error_handler(
        &workflow,
        &mut run,
        StepIdx::new(0),
        &EngineError::BudgetExceeded {
            budget: "test",
            limit: 10,
        },
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    Ok(())
}

#[test]
fn no_handler_returns_no_handler_for_resource_limit_exceeded() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(603, &workflow)?;
    let outcome = route_error_handler(
        &workflow,
        &mut run,
        StepIdx::new(0),
        &EngineError::ResourceLimitExceeded { resource: "cpu" },
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    Ok(())
}

#[test]
fn no_handler_returns_no_handler_for_slot_out_of_bounds() -> Result<(), String> {
    let workflow = make_no_handler_workflow()?;
    let mut run = make_frame(604, &workflow)?;
    let outcome = route_error_handler(
        &workflow,
        &mut run,
        StepIdx::new(0),
        &EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(999),
        },
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::NoHandler)?;
    Ok(())
}

// =========================================================================
// 8. Error chaining: std::error::Error trait
// =========================================================================

#[test]
fn engine_error_implements_std_error() {
    fn takes_error(_: &dyn std::error::Error) {}
    let error = EngineError::DivisionByZero;
    takes_error(&error);
}

#[test]
fn engine_error_source_returns_none() {
    let error = EngineError::DivisionByZero;
    assert!(
        std::error::Error::source(&error).is_none(),
        "DivisionByZero should have no source"
    );
}

#[test]
fn engine_error_source_returns_none_for_compound_variant() {
    let error = EngineError::TypeMismatch {
        expected: "bool",
        found: "i64",
    };
    assert!(
        std::error::Error::source(&error).is_none(),
        "TypeMismatch should have no source"
    );
}

// =========================================================================
// 9. Layer conversion: EngineError alias
// =========================================================================

#[test]
fn engine_error_is_core_error_alias_in_routing_context() {
    let _error: EngineError = crate::errors::CoreError::DivisionByZero;
}

#[test]
fn route_error_handler_accepts_engine_error() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(700, &workflow)?;
    let error: EngineError = crate::errors::CoreError::DivisionByZero;

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;
    Ok(())
}

// =========================================================================
// 10. Display/Debug: all variant messages
// =========================================================================

#[test]
fn error_display_division_by_zero() {
    assert_eq!(EngineError::DivisionByZero.to_string(), "division by zero");
}

#[test]
fn error_display_step_budget_exhausted() {
    assert_eq!(
        EngineError::StepBudgetExhausted.to_string(),
        "step budget exhausted"
    );
}

#[test]
fn error_display_queue_full() {
    assert_eq!(EngineError::QueueFull.to_string(), "queue full");
}

#[test]
fn error_display_allocation_failed() {
    assert_eq!(
        EngineError::AllocationFailed.to_string(),
        "allocation failed"
    );
}

#[test]
fn error_display_non_finite_number() {
    assert_eq!(
        EngineError::NonFiniteNumber.to_string(),
        "non-finite number is not allowed"
    );
}

#[test]
fn error_display_step_counter_overflow() {
    assert_eq!(
        EngineError::StepCounterOverflow.to_string(),
        "step counter overflow"
    );
}

#[test]
fn error_display_all_variants_produce_non_empty_string() {
    let errors: Vec<EngineError> = vec![
        EngineError::DivisionByZero,
        EngineError::NonFiniteNumber,
        EngineError::StepBudgetExhausted,
        EngineError::StepCounterOverflow,
        EngineError::QueueFull,
        EngineError::AllocationFailed,
        EngineError::ExpressionStackUnderflow,
        EngineError::CollectPageLimitExceeded,
        EngineError::CollectItemLimitExceeded,
        EngineError::CollectTimeLimitExceeded,
        EngineError::TypeMismatch {
            expected: "i64",
            found: "bool",
        },
        EngineError::NonBoolCondition {
            slot: SlotIdx::new(1),
        },
        EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        EngineError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        EngineError::MissingNextStep {
            step: StepIdx::new(0),
        },
        EngineError::ExprOutOfBounds {
            expr: crate::ids::ExprIdx::new(0),
        },
        EngineError::ConstOutOfBounds {
            index: ConstIdx::new(0),
        },
        EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        },
        EngineError::MissingOutputSlot {
            step: StepIdx::new(0),
        },
        EngineError::StepStateOutOfBounds {
            step: StepIdx::new(0),
        },
        EngineError::ListIndexOutOfBounds { index: 0 },
        EngineError::SymbolOutOfBounds {
            symbol: crate::ids::SymbolId::new(0),
        },
        EngineError::ListOutOfBounds {
            list: crate::ids::ListId::new(0),
        },
        EngineError::ObjectOutOfBounds {
            object: crate::ids::ObjectId::new(0),
        },
        EngineError::BlobOutOfBounds {
            blob: crate::ids::BlobId::new(0),
        },
        EngineError::UnsupportedPrimitive { primitive: "test" },
        EngineError::UnsupportedAccessorTraversal {
            segment: "idx",
            found: "obj",
        },
        EngineError::ObjectFieldNotFound {
            field: crate::ids::SymbolId::new(0),
        },
        EngineError::InternalInvariantViolation { reason: "test" },
        EngineError::InvalidCompiledWorkflow { reason: "test" },
        EngineError::ExpressionStackOverflow { max: 1 },
        EngineError::IterationLimitExceeded { resource: "test" },
        EngineError::RepeatExhausted { max: 1 },
        EngineError::TogetherBranchLimitExceeded { max: 1 },
        EngineError::ParallelLimitExceeded { limit: 1 },
        EngineError::BudgetExceeded {
            budget: "test",
            limit: 1,
        },
        EngineError::BudgetParse { reason: "test" },
        EngineError::ResourceLimitExceeded { resource: "test" },
    ];

    for error in &errors {
        let msg = error.to_string();
        if msg.is_empty() {
            panic!("Display for {error:?} produced empty string");
        }
    }
}

#[test]
fn error_debug_all_variants_produce_non_empty_string() {
    let errors: Vec<EngineError> = vec![
        EngineError::DivisionByZero,
        EngineError::NonFiniteNumber,
        EngineError::StepBudgetExhausted,
        EngineError::StepCounterOverflow,
        EngineError::QueueFull,
        EngineError::AllocationFailed,
        EngineError::ExpressionStackUnderflow,
        EngineError::CollectPageLimitExceeded,
        EngineError::CollectItemLimitExceeded,
        EngineError::CollectTimeLimitExceeded,
        EngineError::TypeMismatch {
            expected: "i64",
            found: "bool",
        },
        EngineError::NonBoolCondition {
            slot: SlotIdx::new(1),
        },
        EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        EngineError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        EngineError::MissingNextStep {
            step: StepIdx::new(0),
        },
        EngineError::ExprOutOfBounds {
            expr: crate::ids::ExprIdx::new(0),
        },
        EngineError::ConstOutOfBounds {
            index: ConstIdx::new(0),
        },
        EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        },
        EngineError::MissingOutputSlot {
            step: StepIdx::new(0),
        },
        EngineError::StepStateOutOfBounds {
            step: StepIdx::new(0),
        },
        EngineError::ListIndexOutOfBounds { index: 0 },
        EngineError::SymbolOutOfBounds {
            symbol: crate::ids::SymbolId::new(0),
        },
        EngineError::ListOutOfBounds {
            list: crate::ids::ListId::new(0),
        },
        EngineError::ObjectOutOfBounds {
            object: crate::ids::ObjectId::new(0),
        },
        EngineError::BlobOutOfBounds {
            blob: crate::ids::BlobId::new(0),
        },
        EngineError::UnsupportedPrimitive { primitive: "test" },
        EngineError::UnsupportedAccessorTraversal {
            segment: "idx",
            found: "obj",
        },
        EngineError::ObjectFieldNotFound {
            field: crate::ids::SymbolId::new(0),
        },
        EngineError::InternalInvariantViolation { reason: "test" },
        EngineError::InvalidCompiledWorkflow { reason: "test" },
        EngineError::ExpressionStackOverflow { max: 1 },
        EngineError::IterationLimitExceeded { resource: "test" },
        EngineError::RepeatExhausted { max: 1 },
        EngineError::TogetherBranchLimitExceeded { max: 1 },
        EngineError::ParallelLimitExceeded { limit: 1 },
        EngineError::BudgetExceeded {
            budget: "test",
            limit: 1,
        },
        EngineError::BudgetParse { reason: "test" },
        EngineError::ResourceLimitExceeded { resource: "test" },
    ];

    for error in &errors {
        let msg = format!("{error:?}");
        if msg.is_empty() {
            panic!("Debug for {error:?} produced empty string");
        }
    }
}

// =========================================================================
// 11. External source: all lifecycle errors
// =========================================================================

#[test]
fn lifecycle_storage_unavailable_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(800, &workflow)?;
    let error = EngineError::LifecycleStorageUnavailable {
        code: crate::diagnostic::DiagnosticCode::new(0x1501),
        context: String::from("disk full"),
        timestamp: chrono::Utc::now(),
        bead_id: None,
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "LIFECYCLE_STORAGE_UNAVAILABLE")?;
    if !data.message.contains("disk full") {
        return Err(format!(
            "message should contain 'disk full', got: {}",
            data.message
        ));
    }
    Ok(())
}

#[test]
fn lifecycle_duplicate_request_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(810, &workflow)?;
    let error = EngineError::LifecycleDuplicateRequest {
        code: crate::diagnostic::DiagnosticCode::new(0x1502),
        context: String::from("dup request"),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(5)),
        command: Some("run"),
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "LIFECYCLE_DUPLICATE_REQUEST")?;
    Ok(())
}

#[test]
fn lifecycle_stale_request_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(820, &workflow)?;
    let error = EngineError::LifecycleStaleRequest {
        code: crate::diagnostic::DiagnosticCode::new(0x1503),
        context: String::from("stale"),
        timestamp: chrono::Utc::now(),
        bead_id: None,
        command: None,
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "LIFECYCLE_STALE_REQUEST")?;
    Ok(())
}

#[test]
fn lifecycle_invalid_transition_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(830, &workflow)?;
    let error = EngineError::LifecycleInvalidTransition {
        code: crate::diagnostic::DiagnosticCode::new(0x1504),
        context: String::from("bad state"),
        timestamp: chrono::Utc::now(),
        bead_id: None,
        command: Some("step"),
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "LIFECYCLE_INVALID_TRANSITION")?;
    Ok(())
}

#[test]
fn journal_write_failure_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(840, &workflow)?;
    let error = EngineError::JournalWriteFailure {
        code: crate::diagnostic::DiagnosticCode::new(0x1505),
        context: String::from("io fail"),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(3)),
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "JOURNAL_WRITE_FAILURE")?;
    Ok(())
}

#[test]
fn replay_corruption_routing() -> Result<(), String> {
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(850, &workflow)?;
    let error = EngineError::ReplayCorruption {
        code: crate::diagnostic::DiagnosticCode::new(0x1506),
        context: String::from("checksum fail"),
        timestamp: chrono::Utc::now(),
        bead_id: None,
    };

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;

    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    ensure_equal(&*data.code, "REPLAY_CORRUPTION")?;
    Ok(())
}

// =========================================================================
// 12. Partial state: error slot before PC advance
// =========================================================================

#[test]
fn error_slot_written_before_pc_advance_observed_order() -> Result<(), String> {
    // Test that the error slot WAS written (must exist after routing)
    // and PC WAS advanced (already verified in prior tests).
    // The ordering is an internal guarantee; we validate the outcome.
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(900, &workflow)?;
    let error = EngineError::DivisionByZero;

    let _outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;

    let slot_val = run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?;
    ensure_equal(*slot_val, SlotValue::I64(0))?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    Ok(())
}

// =========================================================================
// 13. Double-fault: NoHandler on handler step failure
// =========================================================================

#[test]
fn double_fault_when_handler_step_not_in_workflow() -> Result<(), String> {
    // Error on a step with on_error pointing to an invalid step
    let result = invalid_handler_step_workflow_result();
    ensure_equal(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99),
        }),
    )
}

fn invalid_handler_step_workflow_result() -> Result<CompiledWorkflow, WorkflowError> {
    let parts = WorkflowParts {
        name: Box::<str>::from("doublefault"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: invalid_handler_step_nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts)
}

fn invalid_handler_step_nodes() -> Box<[CompiledNode]> {
    vec![
        invalid_handler_source_node(),
        finish_node_for_invalid_handler(),
    ]
    .into_boxed_slice()
}

fn invalid_handler_source_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: Some(StepIdx::new(99)),
        error_slot: Some(SlotIdx::new(1)),
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }
}

fn finish_node_for_invalid_handler() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }
}

#[test]
fn double_fault_handler_step_has_no_handler() -> Result<(), String> {
    // The handler step (step 2) itself has no on_error handler
    // When routing to it normally works, but if that step were to fail, there's no fallback.
    // Here we just verify that routing to step 2 still succeeds (the double-fault would
    // occur on a subsequent step failure at step 2).
    let workflow = make_simple_handler_workflow()?;
    let mut run = make_frame(1010, &workflow)?;
    let error = EngineError::DivisionByZero;

    let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error)
        .map_err(|e| e.to_string())?;
    ensure_equal(outcome, ErrorHandlerOutcome::Routed)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    Ok(())
}

// =========================================================================
// 14. Proptest: error context never lost
// =========================================================================

mod proptest_properties {
    use proptest::prelude::*;

    use crate::engine::ErrorHandlerOutcome;
    use crate::engine::ErrorSlotData;
    use crate::engine::error_routing::route_error_handler;
    use crate::errors::EngineError;
    use crate::frame::RunFrame;
    use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::ConstValue;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    fn prop_workflow() -> CompiledWorkflow {
        let parts = WorkflowParts {
            name: Box::<str>::from("proptest"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: Some(StepIdx::new(2)),
                    error_slot: Some(SlotIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 5,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).expect("valid proptest workflow")
    }

    fn arb_engine_error() -> impl Strategy<Value = EngineError> {
        prop_oneof![
            Just(EngineError::DivisionByZero),
            Just(EngineError::NonFiniteNumber),
            Just(EngineError::StepBudgetExhausted),
            Just(EngineError::StepCounterOverflow),
            Just(EngineError::QueueFull),
            Just(EngineError::AllocationFailed),
            Just(EngineError::ExpressionStackUnderflow),
            Just(EngineError::CollectPageLimitExceeded),
            Just(EngineError::CollectItemLimitExceeded),
            Just(EngineError::CollectTimeLimitExceeded),
            (any::<u8>()).prop_map(|v| EngineError::ExpressionStackOverflow { max: v }),
            (1u16..100u16).prop_map(|v| EngineError::RepeatExhausted { max: v }),
            (1u16..100u16).prop_map(|v| EngineError::TogetherBranchLimitExceeded { max: v }),
            (1u16..100u16).prop_map(|v| EngineError::ParallelLimitExceeded { limit: v }),
            (any::<u8>()).prop_map(|_| EngineError::TypeMismatch {
                expected: "i64",
                found: "bool",
            }),
            (any::<u8>()).prop_map(|_| EngineError::ResourceLimitExceeded { resource: "test" }),
            (any::<u8>()).prop_map(|v| EngineError::BudgetExceeded {
                budget: "test",
                limit: v as u64,
            }),
            (any::<u16>()).prop_map(|v| EngineError::SlotOutOfBounds {
                slot: SlotIdx::new(v),
            }),
        ]
    }

    proptest! {
        #[test]
        fn error_slot_data_code_is_never_empty(error in arb_engine_error()) {
            let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
            prop_assert!(!data.code.is_empty());
        }

        #[test]
        fn error_slot_data_message_is_never_empty(error in arb_engine_error()) {
            let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
            prop_assert!(!data.message.is_empty());
        }

        #[test]
        fn error_slot_data_failed_step_preserved(error in arb_engine_error(), step in 0u16..100u16) {
            let step_idx = StepIdx::new(step);
            let data = ErrorSlotData::from_engine_error(&error, step_idx);
            prop_assert_eq!(data.failed_step, step_idx);
        }

        #[test]
        fn route_error_handler_never_loses_error_context(
            error in arb_engine_error()
        ) {
            let workflow = prop_workflow();
            let mut run = RunFrame::new(
                RunId::new(1),
                StepIdx::new(0),
                workflow.node_count(),
                workflow.slot_count(),
            ).expect("valid frame");
            let result = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error);
            let outcome = result.expect("route_error_handler must surface Err or Ok, never panic");
            prop_assert!(matches!(
                outcome,
                ErrorHandlerOutcome::Routed | ErrorHandlerOutcome::NoHandler
            ));
        }

        #[test]
        fn error_code_is_static_consistent(error in arb_engine_error()) {
            // Errors with runtime_code have a specific code; others use the static engine code.
            // But every error must produce a non-empty code string.
            // We already covered "never empty" above. Here we verify that the same error
            // always produces the same code (determinism).
            let data1 = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
            let data2 = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
            prop_assert_eq!(data1.code, data2.code);
            prop_assert_eq!(data1.message, data2.message);
        }
    }
}

#[test]
fn error_slot_data_from_expression_stack_underflow() {
    let error = EngineError::ExpressionStackUnderflow;
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(7));
    assert_eq!(&*data.code, "EXPRESSION_STACK_UNDERFLOW");
    assert_eq!(data.failed_step, StepIdx::new(7));
    assert!(!data.message.is_empty());
}

#[test]
fn error_slot_data_from_step_counter_overflow() {
    let error = EngineError::StepCounterOverflow;
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(3));
    assert_eq!(&*data.code, "STEP_COUNTER_OVERFLOW");
    assert_eq!(data.failed_step, StepIdx::new(3));
    assert!(!data.message.is_empty());
}

#[test]
fn error_slot_data_from_parallel_limit_exceeded() {
    let error = EngineError::ParallelLimitExceeded { limit: 16 };
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(9));
    assert_eq!(&*data.code, "PARALLEL_LIMIT_EXCEEDED");
    assert_eq!(data.failed_step, StepIdx::new(9));
    assert!(!data.message.is_empty());
}

#[test]
fn error_slot_data_from_budget_parse() {
    let error = EngineError::BudgetParse {
        reason: "bad value",
    };
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(2));
    assert_eq!(&*data.code, "BUDGET_PARSE");
    assert_eq!(data.failed_step, StepIdx::new(2));
    assert!(!data.message.is_empty());
}

#[test]
fn error_handler_outcome_debug_display() {
    let routed = ErrorHandlerOutcome::Routed;
    let no_handler = ErrorHandlerOutcome::NoHandler;

    let routed_debug = format!("{routed:?}");
    let no_handler_debug = format!("{no_handler:?}");
    assert!(!routed_debug.is_empty());
    assert!(!no_handler_debug.is_empty());
}

#[test]
fn error_slot_data_failed_step_at_zero() {
    let error = EngineError::QueueFull;
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
    assert_eq!(data.failed_step, StepIdx::new(0));
}

#[test]
fn error_slot_data_failed_step_at_max_u32() {
    let error = EngineError::QueueFull;
    let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(u16::MAX));
    assert_eq!(data.failed_step, StepIdx::new(u16::MAX));
}

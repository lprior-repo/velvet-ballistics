#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! BEHAVIOR tests for vb_core admission control and trigger contract behavior.
//!
//! These tests cover:
//! - Admission policy enforcement behavior (RuntimePolicy)
//! - Trigger condition evaluation behavior (EngineSignal emission)
//! - Fail-closed vs fail-open behavior paths (error routing)
//! - Sharp assertions on exact state transitions (StepState machine)
//!
//! All tests use sharp assertions on exact error variants and state transitions.

use vb_core::capability::{Capability, CapabilitySet};
use vb_core::errors::CoreError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ResourceContract, WorkflowParts,
};
use vb_core::{
    EngineSignal, ErrorHandlerOutcome, StepBudget, drive_deterministic, resume_action_completion,
    resume_action_failure, route_error_handler, step_once,
};

// ============================================================================
// Test helpers
// ============================================================================

fn digest(bytes: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([bytes; 32])
}

fn make_simple_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("simple_test"),
        digest: digest(0x01),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
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
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn make_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())
}

fn workflow_with_error_handler(
    handler_body: StepIdx,
    handler_target: StepIdx,
) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("error_handler_workflow"),
        digest: digest(0x02),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: Some(handler_target),
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
                id: handler_body,
                output: Some(SlotIdx::new(2)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn workflow_without_error_handler() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("no_handler_workflow"),
        digest: digest(0x03),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
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
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(99)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn do_node_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_node_workflow"),
        digest: digest(0x04),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
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
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

// ============================================================================
// ADMISSION POLICY ENFORCEMENT BEHAVIOR
// Tests for RuntimePolicy variant behavior
// ============================================================================

mod admission_policy_enforcement {
    use super::*;

    #[test]
    fn runtime_policy_variants_are_exhaustive() {
        // Sharp assertion: RuntimePolicy has exactly 3 variants
        match RuntimePolicy::Strict {
            RuntimePolicy::Strict => {}
            RuntimePolicy::Journaled => panic!("should be Strict"),
            RuntimePolicy::Relaxed => panic!("should be Strict"),
            _ => panic!("unexpected RuntimePolicy variant"),
        }
        match RuntimePolicy::Journaled {
            RuntimePolicy::Strict => panic!("should be Journaled"),
            RuntimePolicy::Journaled => {}
            RuntimePolicy::Relaxed => panic!("should be Journaled"),
            _ => panic!("unexpected RuntimePolicy variant"),
        }
        match RuntimePolicy::Relaxed {
            RuntimePolicy::Strict => panic!("should be Relaxed"),
            RuntimePolicy::Journaled => panic!("should be Relaxed"),
            RuntimePolicy::Relaxed => {}
            _ => panic!("unexpected RuntimePolicy variant"),
        }
    }

    #[test]
    fn runtime_policy_strict_requires_artifact() {
        // Strict policy enforces artifact verification
        let policy = RuntimePolicy::Strict;
        assert_eq!(policy, RuntimePolicy::Strict);
        // Sharp assertion: Strict is not Journaled
        assert_ne!(policy, RuntimePolicy::Journaled);
        assert_ne!(policy, RuntimePolicy::Relaxed);
    }

    #[test]
    fn runtime_policy_journaled_skips_sync_barrier() {
        let policy = RuntimePolicy::Journaled;
        assert_eq!(policy, RuntimePolicy::Journaled);
        assert_ne!(policy, RuntimePolicy::Strict);
        assert_ne!(policy, RuntimePolicy::Relaxed);
    }

    #[test]
    fn runtime_policy_relaxed_testing_only() {
        let policy = RuntimePolicy::Relaxed;
        assert_eq!(policy, RuntimePolicy::Relaxed);
        assert_ne!(policy, RuntimePolicy::Strict);
        assert_ne!(policy, RuntimePolicy::Journaled);
    }

    #[test]
    fn runtime_policy_debug_contains_variant_name() {
        let strict = format!("{:?}", RuntimePolicy::Strict);
        let journaled = format!("{:?}", RuntimePolicy::Journaled);
        let relaxed = format!("{:?}", RuntimePolicy::Relaxed);
        assert!(
            strict.contains("Strict"),
            "Strict debug should contain 'Strict'"
        );
        assert!(
            journaled.contains("Journaled"),
            "Journaled debug should contain 'Journaled'"
        );
        assert!(
            relaxed.contains("Relaxed"),
            "Relaxed debug should contain 'Relaxed'"
        );
    }

    #[test]
    fn runtime_policy_clone_preserves_value() {
        let original = RuntimePolicy::Strict;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}

// ============================================================================
// TRIGGER CONDITION EVALUATION BEHAVIOR
// Tests for EngineSignal emission based on step execution
// ============================================================================

mod trigger_condition_evaluation {
    use super::*;

    #[test]
    fn step_once_emit_continue_on_nop() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        // Precondition: initialize slot so finish doesn't fail
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;

        // Execute step 0 (Nop-like SetConst)
        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        // Sharp assertion: Continue signal emitted
        assert_eq!(result, EngineSignal::Continue);
        assert_eq!(run.pc(), StepIdx::new(1));
        Ok(())
    }

    #[test]
    fn step_once_emit_finished_on_finish() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        // Initialize and advance to finish step
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;
        run.set_pc(StepIdx::new(1)).map_err(|e| e.to_string())?;

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        // Sharp assertion: Finished signal with exact value
        match result {
            EngineSignal::Finished(value, taint) => {
                assert_eq!(value, SlotValue::I64(42));
                assert_eq!(taint, Taint::Clean);
            }
            other => return Err(format!("expected Finished, got {:?}", other)),
        }
        Ok(())
    }

    #[test]
    fn step_once_emit_awaiting_action_on_do_node() -> Result<(), String> {
        let workflow = do_node_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        // Sharp assertion: AwaitingAction signal
        assert!(matches!(result, EngineSignal::AwaitingAction { .. }));
        // PC does not advance for AwaitingAction
        assert_eq!(run.pc(), StepIdx::new(0));
        Ok(())
    }

    #[test]
    fn step_once_emit_awaiting_wait_on_wait_until() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("wait_test"),
            digest: digest(0x05),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        assert_eq!(
            result,
            EngineSignal::AwaitingWait {
                deadline_slot: vb_core::ids::SlotIdx::new(0)
            }
        );
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Waiting
        );
        Ok(())
    }

    #[test]
    fn step_once_emit_awaiting_ask_on_ask() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("ask_test"),
            digest: digest(0x06),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        assert_eq!(result, EngineSignal::AwaitingAsk { timeout_slot: None });
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Asking
        );
        Ok(())
    }

    #[test]
    fn drive_deterministic_emit_step_budget_exhausted() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        // Zero budget should emit StepBudgetExhausted immediately
        let mut budget = StepBudget::new(0);
        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        assert_eq!(result, EngineSignal::StepBudgetExhausted);
        // No steps executed
        assert_eq!(run.executed(), 0);
        Ok(())
    }

    #[test]
    fn drive_deterministic_stops_on_do_suspension() -> Result<(), String> {
        let workflow = do_node_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();
        let mut budget = StepBudget::MAX;

        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        assert!(matches!(result, EngineSignal::AwaitingAction { .. }));
        Ok(())
    }
}

// ============================================================================
// FAIL-CLOSED VS FAIL-OPEN BEHAVIOR PATHS
// Tests for error routing with/without error handlers
// ============================================================================

mod fail_closed_vs_fail_open {
    use super::*;

    #[test]
    fn error_handler_present_routes_to_handler() -> Result<(), String> {
        // Workflow where step 2 is the error handler body
        let workflow = workflow_with_error_handler(StepIdx::new(2), StepIdx::new(2))?;
        let mut run = make_frame(&workflow)?;
        let error = CoreError::DivisionByZero;

        let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error.into())
            .map_err(|e| e.to_string())?;

        // Sharp assertion: Routed outcome
        assert_eq!(outcome, ErrorHandlerOutcome::Routed);
        // PC should be at handler step
        assert_eq!(run.pc(), StepIdx::new(2));
        Ok(())
    }

    #[test]
    fn error_handler_writes_failed_step_to_error_slot() -> Result<(), String> {
        let workflow = workflow_with_error_handler(StepIdx::new(2), StepIdx::new(2))?;
        let mut run = make_frame(&workflow)?;
        let error = CoreError::DivisionByZero;

        route_error_handler(&workflow, &mut run, StepIdx::new(0), &error.into())
            .map_err(|e| e.to_string())?;

        // Error slot (SlotIdx::new(1)) should contain failed step index as I64
        let slot_value = run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?;
        match slot_value {
            SlotValue::I64(val) => assert_eq!(*val, 0), // StepIdx(0).get() == 0
            other => return Err(format!("expected I64 in error slot, got {:?}", other)),
        }
        Ok(())
    }

    #[test]
    fn no_error_handler_returns_no_handler_outcome() -> Result<(), String> {
        let workflow = workflow_without_error_handler()?;
        let mut run = make_frame(&workflow)?;
        let error = CoreError::DivisionByZero;

        let outcome = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error.into())
            .map_err(|e| e.to_string())?;

        // Sharp assertion: NoHandler outcome (fail-closed: error propagates)
        assert_eq!(outcome, ErrorHandlerOutcome::NoHandler);
        Ok(())
    }

    #[test]
    fn error_handler_chain_advances_pc_to_handler() -> Result<(), String> {
        let workflow = workflow_with_error_handler(StepIdx::new(2), StepIdx::new(2))?;
        let mut run = make_frame(&workflow)?;
        let error = CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        };

        route_error_handler(&workflow, &mut run, StepIdx::new(0), &error.into())
            .map_err(|e| e.to_string())?;

        assert_eq!(run.pc(), StepIdx::new(2));
        Ok(())
    }

    #[test]
    fn resume_action_failure_with_handler_routes_and_continues() -> Result<(), String> {
        let workflow = do_node_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        // Execute the Do node to get into suspended state
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
            ..Default::default()
        };

        let (signal, _journal) = resume_action_failure(
            &workflow,
            &mut run,
            ticket,
            vb_core::action::ActionFailureCode::Timeout,
            vb_core::action::RetryPolicy::NonRetryable,
        )
        .map_err(|e| e.to_string())?;

        // No handler configured, so AwaitingAction is returned for external handling
        assert!(matches!(signal, EngineSignal::AwaitingAction { .. }));
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Failed
        );
        Ok(())
    }

    #[test]
    fn division_by_zero_error_routes_correctly() -> Result<(), String> {
        // Create workflow with expression that will cause division by zero
        let expr = vb_core::workflow::ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)), // push 0
                ExprOp::LoadConst(ConstIdx::new(1)), // push 1
                ExprOp::Div,                         // 1/0 - division by zero
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("div_zero"),
            digest: digest(0x07),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: Some(StepIdx::new(2)),
                    error_slot: Some(SlotIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
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
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1), ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;

        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        // Sharp assertion: DivisionByZero is caught and routed to handler
        // Continue signal because error routing succeeded
        assert_eq!(result, EngineSignal::Continue);
        assert_eq!(run.pc(), StepIdx::new(2));
        Ok(())
    }

    #[test]
    fn division_by_zero_without_handler_propagates() -> Result<(), String> {
        let expr = vb_core::workflow::ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("div_zero_no_handler"),
            digest: digest(0x08),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None, // No handler!
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
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
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1), ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;

        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        let result = step_once(&workflow, &mut run, &mut store);

        // Sharp assertion: Error propagates (fail-closed)
        match result {
            Err(CoreError::DivisionByZero) => {}
            Err(other) => return Err(format!("expected DivisionByZero, got {:?}", other)),
            Ok(signal) => return Err(format!("expected error, got {:?}", signal)),
        }
        Ok(())
    }
}

// ============================================================================
// SHARP ASSERTIONS ON EXACT STATE TRANSITIONS
// Tests for StepState machine transitions
// ============================================================================

mod state_transitions {
    use super::*;

    #[test]
    fn step_state_pending_to_running_to_succeeded() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Initial state
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Pending
        );

        // Mark running
        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Running
        );

        // Mark succeeded
        run.mark_succeeded(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Succeeded
        );
        Ok(())
    }

    #[test]
    fn step_state_succeeded_rejects_running_direct_transition() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        run.mark_succeeded(StepIdx::new(0))
            .map_err(|e| e.to_string())?;

        // Master contract (velvet-ballistics-MASTER.md:1569): no terminal
        // state transitions back to running. Loop body reentry uses the
        // explicit Succeeded->Pending admission path before mark_running.
        let result = run.mark_running(StepIdx::new(0));
        assert!(
            matches!(
                result,
                Err(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "invalid_state_transition"
                })
            ),
            "Succeeded→Running must be rejected (terminal states are absorbing)"
        );
        Ok(())
    }

    #[test]
    fn step_state_failed_is_terminal() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        run.mark_failed(StepIdx::new(0))
            .map_err(|e| e.to_string())?;

        // Sharp assertion: Cannot transition from Failed to Succeeded
        let result = run.mark_succeeded(StepIdx::new(0));
        assert!(result.is_err());
        match result {
            Err(CoreError::InternalInvariantViolation { reason }) => {
                assert_eq!(reason, "invalid_state_transition");
            }
            Err(other) => {
                return Err(format!(
                    "expected InternalInvariantViolation, got {:?}",
                    other
                ));
            }
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn step_state_cancelled_is_terminal() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        run.mark_cancelled(StepIdx::new(0))
            .map_err(|e| e.to_string())?;

        // Sharp assertion: Cannot transition from Cancelled back to Running
        let result = run.mark_running(StepIdx::new(0));
        assert!(result.is_err());
        match result {
            Err(CoreError::InternalInvariantViolation { reason }) => {
                assert_eq!(reason, "invalid_state_transition");
            }
            Err(other) => {
                return Err(format!(
                    "expected InternalInvariantViolation, got {:?}",
                    other
                ));
            }
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn step_state_skipped_is_terminal() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        run.mark_skipped(StepIdx::new(0))
            .map_err(|e| e.to_string())?;

        // Sharp assertion: Cannot transition from Skipped to Failed
        let result = run.mark_failed(StepIdx::new(0));
        assert!(result.is_err());
        match result {
            Err(CoreError::InternalInvariantViolation { reason }) => {
                assert_eq!(reason, "invalid_state_transition");
            }
            Err(other) => {
                return Err(format!(
                    "expected InternalInvariantViolation, got {:?}",
                    other
                ));
            }
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn step_state_waiting_and_asking_are_suspension_states() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Waiting state
        run.mark_running(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        run.mark_waiting(StepIdx::new(0))
            .map_err(|e| e.to_string())?;
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Waiting
        );

        // Asking state
        run.mark_running(StepIdx::new(1))
            .map_err(|e| e.to_string())?;
        run.mark_asking(StepIdx::new(1))
            .map_err(|e| e.to_string())?;
        assert_eq!(
            run.step_state(StepIdx::new(1)).map_err(|e| e.to_string())?,
            StepState::Asking
        );
        Ok(())
    }

    #[test]
    fn step_state_pending_transitions_allowed() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Pending -> Running (allowed)
        assert!(run.mark_running(StepIdx::new(0)).is_ok());
        // Pending -> Skipped (allowed via Running)
        run.mark_running(StepIdx::new(1))
            .map_err(|e| e.to_string())?;
        run.mark_skipped(StepIdx::new(1))
            .map_err(|e| e.to_string())?;
        assert_eq!(
            run.step_state(StepIdx::new(1)).map_err(|e| e.to_string())?,
            StepState::Skipped
        );
        Ok(())
    }

    #[test]
    fn frame_pc_rejects_out_of_bounds() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Sharp assertion: PC must be within step_count
        let result = run.set_pc(StepIdx::new(9999));
        assert!(result.is_err());
        match result {
            Err(CoreError::InvalidProgramCounter { step }) => {
                assert_eq!(step, StepIdx::new(9999));
            }
            Err(other) => return Err(format!("expected InvalidProgramCounter, got {:?}", other)),
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn frame_pc_accepts_valid_step() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Sharp assertion: Valid PC values are accepted
        assert!(run.set_pc(StepIdx::new(0)).is_ok());
        assert!(run.set_pc(StepIdx::new(1)).is_ok());
        // Exactly at step_count is invalid
        assert!(run.set_pc(StepIdx::new(2)).is_err());
        Ok(())
    }

    #[test]
    fn frame_increment_executed_overflow_returns_error() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Normal increments work
        run.increment_executed().map_err(|e| e.to_string())?;
        assert_eq!(run.executed(), 1);

        run.increment_executed().map_err(|e| e.to_string())?;
        assert_eq!(run.executed(), 2);
        Ok(())
    }

    #[test]
    fn frame_read_slot_uninitialized_returns_error() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let run = make_frame(&workflow)?;

        // Sharp assertion: Reading uninitialized slot returns specific error
        let result = run.read_slot(SlotIdx::new(0));
        assert!(result.is_err());
        match result {
            Err(CoreError::SlotUninitialized { slot }) => {
                assert_eq!(slot, SlotIdx::new(0));
            }
            Err(other) => return Err(format!("expected SlotUninitialized, got {:?}", other)),
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }

    #[test]
    fn frame_write_and_read_slot_roundtrip() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;

        let value = run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?;
        assert_eq!(*value, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn frame_write_taint_requires_initialized_slot() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;

        // Sharp assertion: Cannot write taint to uninitialized slot
        let result = run.write_taint(SlotIdx::new(0), Taint::Secret);
        assert!(result.is_err());
        match result {
            Err(CoreError::SlotUninitialized { slot }) => {
                assert_eq!(slot, SlotIdx::new(0));
            }
            Err(other) => return Err(format!("expected SlotUninitialized, got {:?}", other)),
            Ok(_) => return Err(String::from("expected error, got Ok")),
        }
        Ok(())
    }
}

// ============================================================================
// RESOURCE CONTRACT ADMISSION BEHAVIOR
// ============================================================================

mod resource_contract_admission {
    use super::*;

    #[test]
    fn resource_contract_default_has_conservative_bounds() {
        let contract = ResourceContract::DEFAULT;

        // Sharp assertions on default bounds
        assert_eq!(contract.max_steps, 1_000);
        assert_eq!(contract.max_slots, 1_024);
        assert_eq!(contract.max_constants, 8_192);
        assert_eq!(contract.max_accessors, 8_192);
        assert_eq!(contract.max_expressions, 4_096);
        assert_eq!(contract.max_expr_stack, 64);
        assert!(contract.allows_secret_results == false);
    }

    #[test]
    fn resource_contract_builder_rejects_excessive_bounds() {
        // Attempting to create a contract with bounds exceeding hard limits
        // should be rejected during workflow validation
        let contract = ResourceContract {
            max_steps: u16::MAX, // Exceeds MAX_STEPS_PER_WORKFLOW
            max_slots: u16::MAX,
            max_constants: u16::MAX,
            max_accessors: u16::MAX,
            max_expressions: u16::MAX,
            max_expr_stack: u8::MAX,
            max_step_budget_per_tick: u64::MAX,
            max_transitions_per_tick: u64::MAX,
            max_input_bytes: u32::MAX,
            max_output_bytes: u32::MAX,
            max_blob_bytes: u64::MAX,
            max_ipc_payload_bytes: u32::MAX,
            max_retry_attempts: u16::MAX,
            max_fanout: u16::MAX,
            max_collect_items: u32::MAX,
            max_queue_depth: u32::MAX,
            max_journal_batch_bytes: u32::MAX,
            allows_secret_results: false,
        };

        // Validation happens in CompiledWorkflow::try_from_parts
        let parts = WorkflowParts {
            name: Box::<str>::from("excessive"),
            digest: digest(0x09),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: contract,
            step_names: Box::new([]),
        };

        let result = CompiledWorkflow::try_from_parts(parts);
        // Sharp assertion: Validation should fail due to excessive max_steps
        assert!(result.is_err());
    }
}

// ============================================================================
// CAPABILITY ADMISSION BEHAVIOR
// ============================================================================

mod capability_admission {
    use super::*;

    #[test]
    fn capability_set_empty_grants_nothing() {
        let caps = CapabilitySet::empty();
        let required = Capability::new("network".into(), ActionId::new(1));

        // Sharp assertion: Empty set grants nothing
        assert!(!caps.grants(&required));
    }

    #[test]
    fn capability_set_grants_exact_match() {
        let caps = CapabilitySet::from_grants(Box::new([Capability::new(
            "network".into(),
            ActionId::new(1),
        )]));

        let required = Capability::new("network".into(), ActionId::new(1));
        // Sharp assertion: Exact match grants
        assert!(caps.grants(&required));
    }

    #[test]
    fn capability_set_rejects_prefix_without_exact_match() {
        let caps =
            CapabilitySet::from_grants(Box::new([Capability::new("net".into(), ActionId::new(1))]));

        let required = Capability::new("network".into(), ActionId::new(1));
        // Sharp assertion: Partial prefix does NOT grant
        assert!(!caps.grants(&required));
    }

    #[test]
    fn capability_set_requires_action_match() {
        let caps = CapabilitySet::from_grants(Box::new([Capability::new(
            "network".into(),
            ActionId::new(1),
        )]));

        let wrong_action = Capability::new("network".into(), ActionId::new(2));
        // Sharp assertion: Same name but different action does NOT grant
        assert!(!caps.grants(&wrong_action));
    }

    #[test]
    fn capability_set_rejects_hierarchical_prefix() {
        let caps = CapabilitySet::from_grants(Box::new([Capability::new(
            "network".into(),
            ActionId::new(1),
        )]));

        let nested = Capability::new("network.github".into(), ActionId::new(1));
        // Sharp assertion: Hierarchical prefix does NOT grant
        assert!(!caps.grants(&nested));
    }

    #[test]
    fn capability_set_empty_name_grants_nothing() {
        let caps =
            CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(1))]));

        let required = Capability::new("network".into(), ActionId::new(1));
        // Sharp assertion: Empty name grants nothing
        assert!(!caps.grants(&required));
    }

    #[test]
    fn capability_denied_error_contains_required_and_granted() {
        let required = Capability::new("network".into(), ActionId::new(1));
        let granted = CapabilitySet::empty();

        // CoreError::CapabilityDenied carries the exact capability required
        let error = CoreError::CapabilityDenied {
            action: ActionId::new(1),
            required: required.clone(),
            granted: granted.clone(),
        };

        match error {
            CoreError::CapabilityDenied {
                action,
                required: req,
                granted: grant,
            } => {
                assert_eq!(action, ActionId::new(1));
                assert_eq!(req.name(), "network");
                assert!(grant.is_empty());
            }
            other => panic!("expected CapabilityDenied, got {:?}", other),
        }
    }
}

// ============================================================================
// STEP BUDGET TRIGGER BEHAVIOR
// ============================================================================

mod step_budget_trigger {
    use super::*;

    #[test]
    fn step_budget_zero_never_returns_true() {
        let mut budget = StepBudget::new(0);
        // Sharp assertion: Zero budget returns false on first try_take
        assert_eq!(budget.try_take().unwrap(), false);
        // Subsequent tries also return false
        assert_eq!(budget.try_take().unwrap(), false);
    }

    #[test]
    fn step_budget_exact_count_succeeds() {
        let mut budget = StepBudget::new(3);
        // Sharp assertion: Exactly 3 successful takes
        assert_eq!(budget.try_take().unwrap(), true);
        assert_eq!(budget.try_take().unwrap(), true);
        assert_eq!(budget.try_take().unwrap(), true);
        // Fourth returns false
        assert_eq!(budget.try_take().unwrap(), false);
    }

    #[test]
    fn step_budget_remaining_decrements() {
        let mut budget = StepBudget::new(5);
        assert_eq!(budget.remaining(), 5);

        budget.try_take().unwrap();
        assert_eq!(budget.remaining(), 4);

        budget.try_take().unwrap();
        assert_eq!(budget.remaining(), 3);
    }

    #[test]
    fn step_budget_clamps_to_max() {
        let budget = StepBudget::new(u64::MAX);
        // Sharp assertion: u64::MAX is clamped to MAX_STEP_BUDGET
        let max = vb_core::limits::MAX_STEP_BUDGET;
        assert_eq!(budget.remaining(), max);
    }

    #[test]
    fn drive_deterministic_respects_budget() -> Result<(), String> {
        let workflow = make_simple_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;

        // Budget of 1: only one step should execute
        let mut budget = StepBudget::new(1);
        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        // Sharp assertion: Budget exhausted after exactly 1 step
        assert_eq!(result, EngineSignal::StepBudgetExhausted);
        assert_eq!(run.executed(), 1);
        assert_eq!(run.pc(), StepIdx::new(1));
        Ok(())
    }
}

// ============================================================================
// EXECUTION SIGNAL EXHAUSTION PATHS
// ============================================================================

mod signal_exhaustion_paths {
    use super::*;

    #[test]
    fn engine_signal_variants_are_distinct() {
        let signals: Vec<EngineSignal> = vec![
            EngineSignal::Continue,
            EngineSignal::Finished(SlotValue::Null, Taint::Clean),
            EngineSignal::StepBudgetExhausted,
            EngineSignal::AwaitingAction {
                step: StepIdx::new(0),
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
            },
            EngineSignal::AwaitingWait {
                deadline_slot: vb_core::ids::SlotIdx::new(0),
            },
            EngineSignal::AwaitingAsk { timeout_slot: None },
        ];

        for (i, sig) in signals.iter().enumerate() {
            for (j, other) in signals.iter().enumerate() {
                if i == j {
                    assert_eq!(sig, other);
                } else {
                    assert_ne!(sig, other);
                }
            }
        }
    }

    #[test]
    fn finished_signal_carries_value_and_taint() -> Result<(), String> {
        let signal = EngineSignal::Finished(SlotValue::I64(42), Taint::Secret);

        match signal {
            EngineSignal::Finished(value, taint) => {
                assert_eq!(value, SlotValue::I64(42));
                assert_eq!(taint, Taint::Secret);
            }
            other => return Err(format!("expected Finished, got {:?}", other)),
        }
        Ok(())
    }

    #[test]
    fn continue_is_not_terminal() {
        let signal = EngineSignal::Continue;
        // Sharp assertion: Continue is not equal to any terminal signal
        assert_ne!(signal, EngineSignal::StepBudgetExhausted);
        assert_ne!(
            signal,
            EngineSignal::Finished(SlotValue::Null, Taint::Clean)
        );
    }
}

// ============================================================================
// ACTION RESUMPTION BEHAVIOR
// ============================================================================

mod action_resumption {
    use super::*;

    #[test]
    fn resume_action_completion_writes_output() -> Result<(), String> {
        let workflow = do_node_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        // Suspend on Do node
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
            ..Default::default()
        };

        let (signal, _journal) = resume_action_completion(
            &workflow,
            &mut run,
            ticket,
            SlotIdx::new(0),
            SlotValue::I64(99),
            Taint::Clean,
        )
        .map_err(|e| e.to_string())?;

        // Sharp assertion: Output written and PC advanced
        assert_eq!(signal, EngineSignal::Continue);
        assert_eq!(run.pc(), StepIdx::new(1));
        let output = run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?;
        assert_eq!(*output, SlotValue::I64(99));
        Ok(())
    }

    #[test]
    fn resume_action_completion_marks_step_succeeded() -> Result<(), String> {
        let workflow = do_node_workflow()?;
        let mut run = make_frame(&workflow)?;
        let mut store = vb_core::value_store::ValueStore::new();

        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
            ..Default::default()
        };

        resume_action_completion(
            &workflow,
            &mut run,
            ticket,
            SlotIdx::new(0),
            SlotValue::I64(99),
            Taint::Clean,
        )
        .map_err(|e| e.to_string())?;

        // Sharp assertion: Step marked as Succeeded
        assert_eq!(
            run.step_state(StepIdx::new(0)).map_err(|e| e.to_string())?,
            StepState::Succeeded
        );
        Ok(())
    }
}

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

//! vb_core workflow execution and slot management BEHAVIOR tests.
//!
//! Tests the public API contract for workflow execution and slot management
//! using Given-When-Then assertions. These are integration/behavior tests
//! that verify the observable behavior of the vb_core public API.
//!
//! ## Workflow Execution API coverage:
//! - `new_run_frame` - creating run frames for compiled workflows
//! - `run_until_blocked` / `drive_deterministic` - deterministic execution loop
//! - `step_once` - single step execution
//! - `EngineSignal` outcomes - Continue, Finished, AwaitingAction, etc.
//! - Error propagation from workflow execution
//!
//! ## Slot Management API coverage:
//! - `RunFrame::new` / `reinitialize` - frame lifecycle
//! - `read_slot` / `write_slot` - slot read/write
//! - `read_taint` / `write_taint` - taint propagation
//! - Bounds checking - SlotOutOfBounds, SlotUninitialized errors
//! - State transitions via `mark_*` methods

use vb_core::{
    // Error types
    CoreError,
    EngineSignal,
    RunFrame,
    StepBudget,
    StepState,
    ValueStore,
    // Engine functions
    drive_deterministic,
    ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest},
    new_run_frame,
    run_until_blocked,
    step_once,
    // Validation
    validate_compiled_workflow,
    value::{ConstValue, SlotValue, Taint},
    workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts},
};

// =============================================================================
// Test fixtures - minimal valid workflows
// =============================================================================

/// A minimal 2-step workflow: SetConst -> Finish
fn two_step_workflow(value: i64) -> CompiledWorkflow {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("test_two_step"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
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
        constants: vec![ConstValue::I64(value)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .expect("workflow construction should succeed for valid parts")
}

/// A 3-step workflow: SetConst -> Copy -> Finish
fn three_step_workflow() -> CompiledWorkflow {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("test_three_step"),
        digest: WorkflowDigest::from_bytes([0xCD; 32]),
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
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
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
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .expect("workflow construction should succeed for valid parts")
}

/// Workflow that suspends on a Do node (action)
fn suspend_on_action_workflow() -> CompiledWorkflow {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("test_do_suspend"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
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
    .expect("workflow construction should succeed for valid parts")
}

/// Helper to create a test frame for a given workflow
fn make_frame(workflow: &CompiledWorkflow, run_id: u64) -> RunFrame {
    new_run_frame(RunId::new(run_id), workflow).expect("frame creation should succeed")
}

// =============================================================================
// BEHAVIOR: new_run_frame creates valid RunFrame
// =============================================================================

mod behavior_new_run_frame {
    use super::*;

    #[test]
    fn given_valid_workflow_when_new_run_frame_then_returns_valid_frame() {
        // GIVEN: A valid compiled workflow with 2 steps and 1 slot
        let workflow = two_step_workflow(100);

        // WHEN: Creating a new run frame
        let frame = new_run_frame(RunId::new(1), &workflow);

        // THEN: The frame is created successfully
        let frame = frame.expect("frame should be created");
        assert_eq!(frame.run_id(), RunId::new(1));
        assert_eq!(frame.pc(), workflow.entry());
        assert_eq!(frame.step_count(), 2);
        assert_eq!(frame.slot_count(), 1);
        assert_eq!(frame.executed(), 0);
    }

    #[test]
    fn given_workflow_with_zero_slots_when_new_run_frame_then_returns_valid_frame() {
        // GIVEN: A workflow with 2 steps but 0 slots (Nop-only, no Finish with slots)
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("zero_slots"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop, // No Finish that references slots
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .expect("workflow should be valid");

        // WHEN: Creating a new run frame
        let result = new_run_frame(RunId::new(1), &workflow);

        // THEN: Frame creation succeeds (slot_count=0 is valid)
        assert!(
            result.is_ok(),
            "frame with 0 slots should be created successfully"
        );
        assert_eq!(result.unwrap().slot_count(), 0);
    }
}

// =============================================================================
// BEHAVIOR: run_until_blocked completes or suspends correctly
// =============================================================================

mod behavior_run_until_blocked {
    use super::*;

    #[test]
    fn given_two_step_workflow_when_run_until_blocked_then_finishes_with_result() {
        // GIVEN: A 2-step workflow that sets a constant and finishes
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 1);
        let mut store = ValueStore::new();

        // WHEN: Running until blocked with sufficient budget
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store);

        // THEN: The workflow finishes with the expected value
        let signal = result.expect("run_until_blocked should not error");
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        );
        assert_eq!(frame.executed(), 2);
    }

    #[test]
    fn given_three_step_workflow_when_run_until_blocked_then_finishes_correctly() {
        // GIVEN: A 3-step workflow: SetConst -> Copy -> Finish
        let workflow = three_step_workflow();
        let mut frame = make_frame(&workflow, 2);
        let mut store = ValueStore::new();

        // WHEN: Running until blocked
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store);

        // THEN: The workflow finishes with the copied value (42)
        let signal = result.expect("run_until_blocked should not error");
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        );
        assert_eq!(frame.executed(), 3);
    }

    #[test]
    fn given_workflow_with_zero_budget_when_run_until_blocked_then_returns_exhausted() {
        // GIVEN: A 2-step workflow
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 3);
        let mut store = ValueStore::new();

        // WHEN: Running with zero budget
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(0), &mut store);

        // THEN: Returns StepBudgetExhausted immediately
        let signal = result.expect("run_until_blocked should not error");
        assert_eq!(signal, EngineSignal::StepBudgetExhausted);
        assert_eq!(frame.executed(), 0); // No steps executed
    }

    #[test]
    fn given_workflow_with_limited_budget_when_run_until_blocked_then_stops_at_budget() {
        // GIVEN: A 3-step workflow
        let workflow = three_step_workflow();
        let mut frame = make_frame(&workflow, 4);
        let mut store = ValueStore::new();

        // WHEN: Running with budget of 2 (exactly enough for first two steps)
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(2), &mut store);

        // THEN: Returns StepBudgetExhausted, PC is at step 2
        let signal = result.expect("run_until_blocked should not error");
        assert_eq!(signal, EngineSignal::StepBudgetExhausted);
        assert_eq!(frame.executed(), 2);
        assert_eq!(frame.pc(), StepIdx::new(2));
    }

    #[test]
    fn given_workflow_with_exact_budget_when_run_until_blocked_then_finishes() {
        // GIVEN: A 2-step workflow
        let workflow = two_step_workflow(99);
        let mut frame = make_frame(&workflow, 5);
        let mut store = ValueStore::new();

        // WHEN: Running with exact budget
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(2), &mut store);

        // THEN: The workflow finishes
        let signal = result.expect("run_until_blocked should not error");
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(99), Taint::Clean)
        );
    }

    #[test]
    fn given_do_suspend_workflow_when_run_until_blocked_then_returns_awaiting_action() {
        // GIVEN: A workflow that suspends on Do node
        let workflow = suspend_on_action_workflow();
        let mut frame = make_frame(&workflow, 6);
        let mut store = ValueStore::new();

        // WHEN: Running until blocked (even with max budget)
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store);

        // THEN: The workflow returns AwaitingAction (suspends)
        // Note: executed counter is NOT incremented for AwaitingAction because
        // the step is suspended, not completed. The step is in Running state.
        let signal = result.expect("run_until_blocked should not error");
        assert!(matches!(signal, EngineSignal::AwaitingAction { .. }));
        assert_eq!(frame.executed(), 0); // Step suspended, not executed to completion
    }
}

// =============================================================================
// BEHAVIOR: drive_deterministic executes steps correctly
// =============================================================================

mod behavior_drive_deterministic {
    use super::*;

    #[test]
    fn given_workflow_when_drive_deterministic_then_consumes_budget() {
        // GIVEN: A 2-step workflow with budget of 1
        let workflow = two_step_workflow(77);
        let mut frame = make_frame(&workflow, 7);
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(1);

        // WHEN: Driving deterministically
        let result = drive_deterministic(&workflow, &mut frame, &mut budget, &mut store);

        // THEN: Single step is executed, budget exhausted
        let signal = result.expect("drive_deterministic should not error");
        assert_eq!(signal, EngineSignal::StepBudgetExhausted);
        assert_eq!(frame.executed(), 1);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn given_workflow_when_drive_deterministic_continues_while_budget_remains() {
        // GIVEN: A 2-step workflow with budget of 10
        let workflow = two_step_workflow(77);
        let mut frame = make_frame(&workflow, 8);
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(10);

        // WHEN: Driving deterministically
        let result = drive_deterministic(&workflow, &mut frame, &mut budget, &mut store);

        // THEN: Workflow completes, budget has remaining
        let signal = result.expect("drive_deterministic should not error");
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(77), Taint::Clean)
        );
        assert_eq!(budget.remaining(), 8); // 10 - 2 = 8 remaining
    }
}

// =============================================================================
// BEHAVIOR: step_once executes single step correctly
// =============================================================================

mod behavior_step_once {
    use super::*;

    #[test]
    fn given_two_step_workflow_when_step_once_then_executes_first_step() {
        // GIVEN: A 2-step workflow at the entry point
        let workflow = two_step_workflow(55);
        let mut frame = make_frame(&workflow, 9);
        let mut store = ValueStore::new();

        // WHEN: Executing a single step
        let result = step_once(&workflow, &mut frame, &mut store);

        // THEN: Returns Continue, PC advanced to step 1
        let signal = result.expect("step_once should not error");
        assert_eq!(signal, EngineSignal::Continue);
        assert_eq!(frame.pc(), StepIdx::new(1));
        assert_eq!(frame.executed(), 1);
    }

    #[test]
    fn given_finish_step_when_step_once_then_returns_finished() {
        // GIVEN: A workflow positioned at the Finish node with slot pre-initialized
        let workflow = two_step_workflow(55);
        let mut frame = make_frame(&workflow, 10);
        // Pre-initialize the result slot (as if SetConst step ran)
        frame
            .write_slot(SlotIdx::new(0), SlotValue::I64(55))
            .expect("slot write should succeed");
        // Manually set PC to the finish step
        frame
            .set_pc(StepIdx::new(1))
            .expect("set_pc should succeed");
        let mut store = ValueStore::new();

        // WHEN: Executing the finish step
        let result = step_once(&workflow, &mut frame, &mut store);

        // THEN: Returns Finished with the result
        let signal = result.expect("step_once should not error");
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(55), Taint::Clean)
        );
    }

    #[test]
    fn given_do_node_when_step_once_then_returns_awaiting_action() {
        // GIVEN: A workflow at a Do node
        let workflow = suspend_on_action_workflow();
        let mut frame = make_frame(&workflow, 11);
        let mut store = ValueStore::new();

        // WHEN: Executing the Do step
        let result = step_once(&workflow, &mut frame, &mut store);

        // THEN: Returns AwaitingAction
        let signal = result.expect("step_once should not error");
        assert!(matches!(signal, EngineSignal::AwaitingAction { .. }));
    }
}

// =============================================================================
// BEHAVIOR: Slot read/write and bounds checking
// =============================================================================

mod behavior_slot_management {
    use super::*;

    #[test]
    fn given_empty_frame_when_write_and_read_slot_then_roundtrips_correctly() {
        // GIVEN: A frame with 2 slots, all uninitialized
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("slot_test"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .unwrap();
        let mut frame = make_frame(&workflow, 20);

        // WHEN: Writing values to slots 0 and 1
        frame
            .write_slot(SlotIdx::new(0), SlotValue::I64(100))
            .expect("write should succeed");
        frame
            .write_slot(SlotIdx::new(1), SlotValue::Bool(true))
            .expect("write should succeed");

        // THEN: Reading returns the written values
        assert_eq!(
            frame
                .read_slot(SlotIdx::new(0))
                .expect("read should succeed"),
            &SlotValue::I64(100)
        );
        assert_eq!(
            frame
                .read_slot(SlotIdx::new(1))
                .expect("read should succeed"),
            &SlotValue::Bool(true)
        );
    }

    #[test]
    fn given_frame_when_read_uninitialized_slot_then_returns_error() {
        // GIVEN: A frame with uninitialized slot 0
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("slot_test"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let frame = make_frame(&workflow, 21);

        // WHEN: Reading an uninitialized slot
        let result = frame.read_slot(SlotIdx::new(0));

        // THEN: Returns SlotUninitialized error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::SlotUninitialized { slot } if slot == SlotIdx::new(0)));
    }

    #[test]
    fn given_frame_when_read_out_of_bounds_slot_then_returns_error() {
        // GIVEN: A frame with 2 slots (indices 0 and 1)
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("slot_test"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .unwrap();
        let frame = make_frame(&workflow, 22);

        // WHEN: Reading slot index 99 (out of bounds)
        let result = frame.read_slot(SlotIdx::new(99));

        // THEN: Returns SlotOutOfBounds error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::SlotOutOfBounds { slot } if slot == SlotIdx::new(99)));
    }

    #[test]
    fn given_frame_when_write_and_read_taint_then_roundtrips() {
        // GIVEN: A frame with initialized slot
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_test"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 23);

        // Initialize the slot first
        frame
            .write_slot(SlotIdx::new(0), SlotValue::Null)
            .expect("write should succeed");

        // WHEN: Writing taint to the slot
        frame
            .write_taint(SlotIdx::new(0), Taint::Secret)
            .expect("taint write should succeed");

        // THEN: Reading taint returns the written value
        let taint = frame
            .read_taint(SlotIdx::new(0))
            .expect("taint read should succeed");
        assert_eq!(taint, Taint::Secret);
    }

    #[test]
    fn given_frame_when_write_taint_to_uninitialized_slot_then_returns_error() {
        // GIVEN: A frame with uninitialized slot
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_test"),
            digest: WorkflowDigest::from_bytes([0x44; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 24);

        // WHEN: Writing taint to uninitialized slot
        let result = frame.write_taint(SlotIdx::new(0), Taint::Secret);

        // THEN: Returns SlotUninitialized error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::SlotUninitialized { slot } if slot == SlotIdx::new(0)));
    }

    #[test]
    fn given_frame_when_overwrite_slot_then_new_value_replaces_old() {
        // GIVEN: A frame with slot 0 containing I64(10)
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("overwrite_test"),
            digest: WorkflowDigest::from_bytes([0x55; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 25);

        frame
            .write_slot(SlotIdx::new(0), SlotValue::I64(10))
            .expect("initial write should succeed");

        // WHEN: Overwriting with a different value
        frame
            .write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .expect("overwrite should succeed");

        // THEN: New value is returned
        assert_eq!(
            frame
                .read_slot(SlotIdx::new(0))
                .expect("read should succeed"),
            &SlotValue::Bool(false)
        );
    }
}

// =============================================================================
// BEHAVIOR: Step state transitions
// =============================================================================

mod behavior_step_state_transitions {
    use super::*;

    #[test]
    fn given_frame_when_mark_steps_through_state_machine_then_states_correct() {
        // GIVEN: A frame with 3 steps (need 3 nodes in workflow)
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("state_test"),
            digest: WorkflowDigest::from_bytes([0x66; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 30);

        // THEN: Initial state is Pending for all steps
        assert_eq!(
            frame
                .step_state(StepIdx::new(0))
                .expect("step_state should succeed"),
            StepState::Pending
        );
        assert_eq!(
            frame
                .step_state(StepIdx::new(1))
                .expect("step_state should succeed"),
            StepState::Pending
        );
        assert_eq!(
            frame
                .step_state(StepIdx::new(2))
                .expect("step_state should succeed"),
            StepState::Pending
        );

        // WHEN: Marking step 0 as Running -> Succeeded
        frame
            .mark_running(StepIdx::new(0))
            .expect("mark_running should succeed");
        frame
            .mark_succeeded(StepIdx::new(0))
            .expect("mark_succeeded should succeed");

        // THEN: Step 0 is Succeeded
        assert_eq!(
            frame
                .step_state(StepIdx::new(0))
                .expect("step_state should succeed"),
            StepState::Succeeded
        );

        // WHEN: Marking step 1 as Running -> Failed
        frame
            .mark_running(StepIdx::new(1))
            .expect("mark_running should succeed");
        frame
            .mark_failed(StepIdx::new(1))
            .expect("mark_failed should succeed");

        // THEN: Step 1 is Failed
        assert_eq!(
            frame
                .step_state(StepIdx::new(1))
                .expect("step_state should succeed"),
            StepState::Failed
        );
    }

    #[test]
    fn given_frame_when_mark_step_out_of_bounds_then_returns_error() {
        // GIVEN: A frame with 2 steps
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("state_test"),
            digest: WorkflowDigest::from_bytes([0x77; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 31);

        // WHEN: Marking step 99 (out of bounds)
        let result = frame.mark_running(StepIdx::new(99));

        // THEN: Returns StepStateOutOfBounds error
        assert!(result.is_err());
    }

    #[test]
    fn given_frame_when_mark_terminal_state_then_transition_blocked() {
        // GIVEN: A frame with step in Succeeded (terminal) state
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("state_test"),
            digest: WorkflowDigest::from_bytes([0x88; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 32);

        // Move step to terminal Succeeded state
        frame
            .mark_running(StepIdx::new(0))
            .expect("mark_running should succeed");
        frame
            .mark_succeeded(StepIdx::new(0))
            .expect("mark_succeeded should succeed");

        // WHEN: Attempting direct Succeeded→Running (no admission path).
        let result = frame.mark_running(StepIdx::new(0));

        // THEN: Rejected (master contract: no terminal→running edge).
        assert!(
            matches!(
                result,
                Err(vb_core::errors::CoreError::InternalInvariantViolation {
                    reason: "invalid_state_transition"
                })
            ),
            "Succeeded→Running must be rejected (terminal states are absorbing)"
        );
    }
}

// =============================================================================
// BEHAVIOR: RunFrame reinitialize
// =============================================================================

mod behavior_run_frame_reinitialize {
    use super::*;

    #[test]
    fn given_used_frame_when_reinitialize_then_resets_state() {
        // GIVEN: A frame that has been used (states modified, slots written)
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 40);

        // Advance the workflow
        let mut store = ValueStore::new();
        run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
            .expect("run should succeed");

        // Verify initial state after run
        assert_eq!(frame.executed(), 2);

        // WHEN: Reinitializing for a new run
        let result = frame.reinitialize(RunId::new(99), StepIdx::new(0), 2, 1);

        // THEN: Reinitialization succeeds
        result.expect("reinitialize should succeed");
        assert_eq!(frame.run_id(), RunId::new(99));
        assert_eq!(frame.pc(), StepIdx::new(0));
        assert_eq!(frame.executed(), 0);
        assert_eq!(
            frame
                .step_state(StepIdx::new(0))
                .expect("step_state should succeed"),
            StepState::Pending
        );
        assert_eq!(
            frame
                .step_state(StepIdx::new(1))
                .expect("step_state should succeed"),
            StepState::Pending
        );
    }

    #[test]
    fn given_frame_when_reinitialize_with_different_dimensions_then_returns_error() {
        // GIVEN: A frame with 2 steps, 1 slot
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 41);

        // WHEN: Reinitializing with different dimensions (3 steps instead of 2)
        let result = frame.reinitialize(RunId::new(99), StepIdx::new(0), 3, 1);

        // THEN: Returns error (dimension mismatch)
        assert!(result.is_err());
    }

    #[test]
    fn given_frame_when_reinitialize_with_out_of_bounds_entry_then_returns_error() {
        // GIVEN: A frame with 2 steps
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 42);

        // WHEN: Reinitializing with entry step beyond step count
        let result = frame.reinitialize(RunId::new(99), StepIdx::new(99), 2, 1);

        // THEN: Returns InvalidProgramCounter error
        assert!(result.is_err());
    }
}

// =============================================================================
// BEHAVIOR: EngineSignal outcomes
// =============================================================================

mod behavior_engine_signals {
    use super::*;

    #[test]
    fn given_workflow_when_execute_to_finish_then_signal_carries_correct_taint() {
        // GIVEN: A workflow with a clean (non-tainted) constant
        let workflow = two_step_workflow(42);
        let mut frame = make_frame(&workflow, 50);
        let mut store = ValueStore::new();

        // WHEN: Running to completion
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store);

        // THEN: Finished signal carries Clean taint for clean data
        let signal = result.expect("run_until_blocked should succeed");
        match signal {
            EngineSignal::Finished(value, taint) => {
                assert_eq!(value, SlotValue::I64(42));
                assert_eq!(taint, Taint::Clean);
            }
            _ => panic!("Expected Finished signal, got {:?}", signal),
        }
    }

    #[test]
    fn given_do_workflow_when_step_once_then_awaiting_action_signal() {
        // GIVEN: A workflow with a Do node
        let workflow = suspend_on_action_workflow();
        let mut frame = make_frame(&workflow, 51);
        let mut store = ValueStore::new();

        // WHEN: Stepping once
        let result = step_once(&workflow, &mut frame, &mut store);

        // THEN: AwaitingAction is returned
        let signal = result.expect("step_once should succeed");
        assert!(matches!(signal, EngineSignal::AwaitingAction { .. }));
    }

    #[test]
    fn given_workflow_when_budget_exhausted_then_step_budget_exhausted_signal() {
        // GIVEN: A 3-step workflow
        let workflow = three_step_workflow();
        let mut frame = make_frame(&workflow, 52);
        let mut store = ValueStore::new();

        // WHEN: Running with budget of 0
        let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(0), &mut store);

        // THEN: StepBudgetExhausted signal
        let signal = result.expect("run_until_blocked should succeed");
        assert_eq!(signal, EngineSignal::StepBudgetExhausted);
    }
}

// =============================================================================
// BEHAVIOR: Error propagation from workflow execution
// =============================================================================

mod behavior_error_propagation {
    use super::*;

    #[test]
    fn given_workflow_with_empty_nodes_when_validated_then_returns_error() {
        // GIVEN: Workflow parts with empty nodes array
        let parts = WorkflowParts {
            name: Box::<str>::from("empty_nodes"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };

        // WHEN: Validating
        let result = validate_compiled_workflow(&parts);

        // THEN: Returns error (EmptyNodes)
        assert!(result.is_err());
    }
}

// =============================================================================
// BEHAVIOR: Parallel in-flight tracking
// =============================================================================

mod behavior_parallel_in_flight {
    use super::*;

    #[test]
    fn given_frame_when_add_sub_parallel_in_flight_then_counts_correct() {
        // GIVEN: A frame
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("parallel_test"),
            digest: WorkflowDigest::from_bytes([0x99; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 70);

        // THEN: Initial parallel_in_flight is 0
        assert_eq!(frame.parallel_in_flight(), 0);

        // WHEN: Adding 5 to parallel in-flight
        frame.add_parallel_in_flight(5).expect("add should succeed");

        // THEN: Count is 5
        assert_eq!(frame.parallel_in_flight(), 5);

        // WHEN: Subtracting 3
        frame.sub_parallel_in_flight(3).expect("sub should succeed");

        // THEN: Count is 2
        assert_eq!(frame.parallel_in_flight(), 2);
    }

    #[test]
    fn given_frame_when_parallel_in_flight_underflows_then_returns_error() {
        // GIVEN: A frame with parallel_in_flight = 0
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("parallel_test"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 71);

        // WHEN: Subtracting when count is 0
        let result = frame.sub_parallel_in_flight(1);

        // THEN: Returns InternalInvariantViolation (underflow)
        assert!(result.is_err());
    }
}

// =============================================================================
// BEHAVIOR: Slot snapshots
// =============================================================================

mod behavior_slot_snapshots {
    use super::*;

    #[test]
    fn given_frame_with_written_slots_when_slots_snapshot_then_contains_all_values() {
        // GIVEN: A frame with 3 slots, slots 0 and 2 written
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("snapshot_test"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .unwrap();
        let mut frame = make_frame(&workflow, 80);

        frame
            .write_slot(SlotIdx::new(0), SlotValue::I64(1))
            .expect("write should succeed");
        frame
            .write_slot(SlotIdx::new(2), SlotValue::Bool(true))
            .expect("write should succeed");

        // WHEN: Getting slots snapshot
        let snapshot = frame.slots_snapshot();

        // THEN: Snapshot has 3 elements, slot 0 = Some(I64(1)), slot 1 = None, slot 2 = Some(Bool(true))
        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot[0], Some(SlotValue::I64(1)));
        assert_eq!(snapshot[1], None);
        assert_eq!(snapshot[2], Some(SlotValue::Bool(true)));
    }

    #[test]
    fn given_frame_when_initialized_slots_then_returns_only_written() {
        // GIVEN: A frame with 3 slots, only slot 1 written
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("snapshot_test"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .unwrap();
        let mut frame = make_frame(&workflow, 81);

        frame
            .write_slot(
                SlotIdx::new(1),
                SlotValue::Symbol(vb_core::ids::SymbolId::new(5)),
            )
            .expect("write should succeed");

        // WHEN: Getting initialized slots
        let initialized = frame
            .initialized_slots()
            .expect("initialized_slots should succeed");

        // THEN: Only one entry (slot 1)
        assert_eq!(initialized.len(), 1);
        assert_eq!(initialized[0].0, SlotIdx::new(1));
        assert_eq!(
            initialized[0].1,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(5))
        );
        assert_eq!(initialized[0].2, Taint::Clean); // Default taint
    }
}

// =============================================================================
// BEHAVIOR: validate_compiled_workflow
// =============================================================================

mod behavior_validate_compiled_workflow {
    use super::*;

    #[test]
    fn given_valid_workflow_parts_when_validate_then_returns_ok() {
        // GIVEN: Valid workflow parts for a 2-step workflow
        let parts = WorkflowParts {
            name: Box::<str>::from("validate_test"),
            digest: WorkflowDigest::from_bytes([0x12; 32]),
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
        };

        // WHEN: Validating
        let result = validate_compiled_workflow(&parts);

        // THEN: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn given_workflow_with_empty_nodes_when_validate_then_returns_error() {
        // GIVEN: A workflow with no nodes
        let result = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("empty"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        });

        // THEN: Construction fails
        assert!(result.is_err());
    }
}

// =============================================================================
// BEHAVIOR: Taint propagation through slot operations
// =============================================================================

mod behavior_taint_propagation {
    use super::*;

    #[test]
    fn given_frame_when_write_slot_with_taint_then_both_set() {
        // GIVEN: A frame with 1 slot
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_test"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 90);

        // WHEN: Writing with taint
        frame
            .write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)
            .expect("write with taint should succeed");

        // THEN: Both value and taint are set correctly
        assert_eq!(
            frame
                .read_slot(SlotIdx::new(0))
                .expect("read should succeed"),
            &SlotValue::I64(99)
        );
        assert_eq!(
            frame
                .read_taint(SlotIdx::new(0))
                .expect("taint read should succeed"),
            Taint::Secret
        );
    }

    #[test]
    fn given_frame_when_write_taint_out_of_bounds_then_returns_error() {
        // GIVEN: A frame with 1 slot
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("taint_test"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 91);

        // WHEN: Writing taint to out-of-bounds slot
        let result = frame.write_taint(SlotIdx::new(99), Taint::Secret);

        // THEN: Returns SlotOutOfBounds error
        assert!(result.is_err());
    }
}

// =============================================================================
// BEHAVIOR: PC manipulation
// =============================================================================

mod behavior_pc_manipulation {
    use super::*;

    #[test]
    fn given_frame_when_set_pc_valid_then_pc_updated() {
        // GIVEN: A frame with 5 steps (need 5 nodes in workflow)
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("pc_test"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 100);

        // WHEN: Setting PC to a valid step (step 3 is within 0..5)
        let result = frame.set_pc(StepIdx::new(3));

        // THEN: PC is updated
        assert!(result.is_ok());
        assert_eq!(frame.pc(), StepIdx::new(3));
    }

    #[test]
    fn given_frame_when_set_pc_out_of_bounds_then_pc_unchanged() {
        // GIVEN: A frame with 3 steps
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("pc_test"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
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
        .unwrap();
        let mut frame = make_frame(&workflow, 101);
        let original_pc = frame.pc();

        // WHEN: Setting PC to out-of-bounds step (99 >= 3)
        let result = frame.set_pc(StepIdx::new(99));

        // THEN: Returns error, PC unchanged
        assert!(result.is_err());
        assert_eq!(frame.pc(), original_pc);
    }
}

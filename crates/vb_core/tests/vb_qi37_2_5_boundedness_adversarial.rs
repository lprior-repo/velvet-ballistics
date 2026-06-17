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
    unused_variables,
)]
#![forbid(unsafe_code)]

use bytes::Bytes;
use proptest::prelude::*;
use vb_core::{
    AggregateResourceBudget, BoundednessPolicy, BudgetError, CompiledNode, CompiledNodeKind,
    ConstValue, CoreError, EngineSignal, ObjectField, ResourceContract, RunId, SlotIdx, SlotValue,
    StepBudget, StepIdx, Taint, ValueStore, WholeWorkflowBudget, WorkflowDigest, WorkflowError,
    WorkflowParts,
    limits::{
        MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
        MAX_STEP_BUDGET, MAX_SYMBOL_BYTES_PER_VALUE,
    },
    new_run_frame, run_until_blocked,
    workflow::CompiledWorkflow,
};

fn policy_limit_budget(policy: BoundednessPolicy) -> WholeWorkflowBudget {
    WholeWorkflowBudget {
        max_total_steps: policy.max_total_steps,
        max_total_slots: policy.max_total_slots,
        max_fanout: policy.max_fanout,
        max_nesting_depth: policy.max_nesting_depth,
        max_steps_executable: policy.absolute_max_steps_executable,
        max_action_tickets: policy.absolute_max_action_tickets,
        max_parallel_in_flight: policy.absolute_max_parallel,
        max_retries_per_action: 3,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: policy.absolute_max_run_time_seconds,
        max_result_bytes: policy.absolute_max_result_bytes,
        max_total_slots_written: 1,
        max_timer_entries: 0,
        max_trace_events: policy.max_total_steps,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: 0,
    }
}

fn two_step_workflow() -> Result<CompiledWorkflow, WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("vb-qi37-2-5-two-step"),
        digest: WorkflowDigest::from_bytes([37; 32]),
        nodes: Box::from([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
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
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(99)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    })
}

fn workflow_parts(name: &'static str, nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes([45; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    }
}

fn finite_nested_nodes(collect_limit: u32) -> Box<[CompiledNode]> {
    Box::from([
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: collect_limit,
                page_size: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::from([StepIdx::new(2), StepIdx::new(4)]),
                join: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 2,
                body: StepIdx::new(3),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: Some(StepIdx::new(6)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ])
}

fn finite_nested_budget(collect_limit: u32) -> Result<WholeWorkflowBudget, WorkflowError> {
    let nodes = finite_nested_nodes(collect_limit);
    WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
}

fn step_count_overflow_nodes() -> Box<[CompiledNode]> {
    Box::from([
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: u16::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: u16::MAX,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
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
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ])
}

fn assert_budget_exceeded(actual: Result<vb_core::SymbolId, CoreError>, limit: u64) {
    assert_eq!(
        actual,
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit,
        })
    );
}

fn bounded_fixture_len(requested: usize, hard_limit: usize) -> Result<usize, CoreError> {
    if requested > hard_limit {
        Err(CoreError::ResourceLimitExceeded {
            resource: "fixture_allocation_len",
        })
    } else {
        Ok(requested)
    }
}

#[test]
fn given_public_constructors_when_adversarial_workflow_built_then_no_private_invalid_state_required()
-> Result<(), String> {
    let parts = workflow_parts("vb-qi37-2-5-public-nested", finite_nested_nodes(3));

    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;

    assert_eq!(workflow.name(), "vb-qi37-2-5-public-nested");
    assert_eq!(workflow.entry(), StepIdx::new(0));
    assert_eq!(workflow.node_count(), 7);
    assert_eq!(workflow.resource_contract(), ResourceContract::DEFAULT);
    Ok(())
}

#[test]
fn given_adversarial_size_parameters_when_generators_run_then_all_allocations_are_prebounded() {
    let accepted_symbol_len =
        bounded_fixture_len(MAX_SYMBOL_BYTES_PER_VALUE, MAX_SYMBOL_BYTES_PER_VALUE);
    let rejected_symbol_len =
        bounded_fixture_len(MAX_SYMBOL_BYTES_PER_VALUE + 1, MAX_SYMBOL_BYTES_PER_VALUE);
    let accepted_list_len = bounded_fixture_len(MAX_LIST_ITEMS_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE);
    let rejected_list_len =
        bounded_fixture_len(MAX_LIST_ITEMS_PER_VALUE + 1, MAX_LIST_ITEMS_PER_VALUE);

    assert_eq!(accepted_symbol_len, Ok(MAX_SYMBOL_BYTES_PER_VALUE));
    assert_eq!(
        rejected_symbol_len,
        Err(CoreError::ResourceLimitExceeded {
            resource: "fixture_allocation_len",
        })
    );
    assert_eq!(accepted_list_len, Ok(MAX_LIST_ITEMS_PER_VALUE));
    assert_eq!(
        rejected_list_len,
        Err(CoreError::ResourceLimitExceeded {
            resource: "fixture_allocation_len",
        })
    );
}

#[test]
fn given_explicit_step_budget_when_workflow_runs_then_step_budget_exhausted_is_returned()
-> Result<(), String> {
    let workflow = two_step_workflow().map_err(|error| error.to_string())?;
    let mut run = new_run_frame(RunId::new(37), &workflow).map_err(|error| error.to_string())?;
    let mut store = ValueStore::new();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

    assert_eq!(result, Ok(EngineSignal::StepBudgetExhausted));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.pc(), StepIdx::new(1));
    Ok(())
}

#[test]
fn given_runaway_loop_when_budget_reaches_zero_then_execution_returns_step_budget_exhausted_without_panic()
-> Result<(), String> {
    let workflow = two_step_workflow().map_err(|error| error.to_string())?;
    let mut run = new_run_frame(RunId::new(38), &workflow).map_err(|error| error.to_string())?;
    let mut store = ValueStore::new();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

    assert_eq!(result, Ok(EngineSignal::StepBudgetExhausted));
    assert_eq!(run.executed(), 0);
    assert_eq!(run.pc(), StepIdx::new(0));
    Ok(())
}

#[test]
fn given_any_u64_budget_when_step_budget_new_then_remaining_is_clamped_and_try_take_is_monotonic()
-> Result<(), String> {
    let mut budget = StepBudget::new(u64::MAX);

    assert_eq!(budget.remaining(), MAX_STEP_BUDGET);
    assert_eq!(budget.try_take().map_err(|error| error.to_string())?, true);
    assert_eq!(budget.remaining(), MAX_STEP_BUDGET - 1);

    let mut zero = StepBudget::new(0);
    assert_eq!(zero.remaining(), 0);
    assert_eq!(zero.try_take().map_err(|error| error.to_string())?, false);
    assert_eq!(zero.remaining(), 0);
    Ok(())
}

#[test]
fn given_policy_limits_when_validate_runs_then_at_limit_budget_is_accepted() {
    let policy = BoundednessPolicy::DEFAULT;
    let budget = policy_limit_budget(policy);

    assert_eq!(policy.validate(&budget), Ok(()));
}

#[test]
fn given_each_policy_dimension_above_limit_when_validate_runs_then_matching_budget_error_variant_returns()
 {
    let policy = BoundednessPolicy::DEFAULT;

    let mut budget = policy_limit_budget(policy);
    budget.max_total_steps = policy.max_total_steps + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::TotalStepsExceeded {
            actual: policy.max_total_steps + 1,
            limit: policy.max_total_steps,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_total_slots = policy.max_total_slots + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::TotalSlotsExceeded {
            actual: policy.max_total_slots + 1,
            limit: policy.max_total_slots,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_fanout = policy.max_fanout + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::FanoutExceeded {
            actual: policy.max_fanout + 1,
            limit: policy.max_fanout,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_nesting_depth = policy.max_nesting_depth + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::NestingDepthExceeded {
            actual: policy.max_nesting_depth + 1,
            limit: policy.max_nesting_depth,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_parallel_in_flight = policy.absolute_max_parallel + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::ParallelExceeded {
            actual: policy.absolute_max_parallel + 1,
            limit: policy.absolute_max_parallel,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_action_tickets = policy.absolute_max_action_tickets + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::ActionTicketsExceeded {
            actual: policy.absolute_max_action_tickets + 1,
            limit: policy.absolute_max_action_tickets,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_run_time_seconds = policy.absolute_max_run_time_seconds + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::RunTimeExceeded {
            actual: policy.absolute_max_run_time_seconds + 1,
            limit: policy.absolute_max_run_time_seconds,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_result_bytes = policy.absolute_max_result_bytes + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::ResultBytesExceeded {
            actual: policy.absolute_max_result_bytes + 1,
            limit: policy.absolute_max_result_bytes,
        })
    );

    let mut budget = policy_limit_budget(policy);
    budget.max_steps_executable = policy.absolute_max_steps_executable + 1;
    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::StepsExecutableExceeded {
            actual: policy.absolute_max_steps_executable + 1,
            limit: policy.absolute_max_steps_executable,
        })
    );
}

#[test]
fn given_entry_out_of_bounds_when_budget_compute_runs_then_typed_workflow_error_returns() {
    let nodes: Box<[CompiledNode]> = Box::from([]);

    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT);

    assert_eq!(
        result,
        Err(WorkflowError::EntryOutOfBounds {
            entry: StepIdx::new(0),
        })
    );
}

#[test]
fn given_finite_nested_composition_when_budget_computed_then_each_growth_dimension_is_explicit()
-> Result<(), String> {
    let budget = finite_nested_budget(3).map_err(|error| error.to_string())?;

    assert_eq!(budget.max_gather_pages, 1);
    assert_eq!(budget.max_gather_items, 3);
    assert_eq!(budget.max_fanout, 2);
    assert_eq!(budget.max_parallel_in_flight, 2);
    assert_eq!(budget.max_together_branches, 2);
    assert_eq!(budget.max_repeat_attempts, 2);
    assert_eq!(budget.max_nesting_depth, 3);
    Ok(())
}

#[test]
fn given_nested_repeat_together_collect_exceeds_policy_when_verified_then_typed_diagnostic_rejects_before_runtime()
-> Result<(), String> {
    let budget = finite_nested_budget(3).map_err(|error| error.to_string())?;
    let policy = BoundednessPolicy {
        max_total_steps: budget.max_total_steps,
        max_total_slots: budget.max_total_slots,
        max_fanout: 1,
        max_nesting_depth: budget.max_nesting_depth,
        absolute_max_action_tickets: budget.max_action_tickets,
        absolute_max_parallel: budget.max_parallel_in_flight,
        absolute_max_run_time_seconds: budget.max_run_time_seconds,
        absolute_max_result_bytes: budget.max_result_bytes,
        absolute_max_steps_executable: budget.max_steps_executable,
        ..BoundednessPolicy::DEFAULT
    };

    assert_eq!(
        policy.validate(&budget),
        Err(BudgetError::FanoutExceeded {
            actual: 2,
            limit: 1,
        })
    );
    Ok(())
}

#[test]
fn given_bounded_workflow_within_policy_when_computed_and_validated_then_budget_is_accepted()
-> Result<(), String> {
    let budget = finite_nested_budget(3).map_err(|error| error.to_string())?;

    // Cold-AST-conservative iter count (master §45) is 1 for RepeatStart, so
    // the nested RepeatStart body is counted once instead of being multiplied
    // by `max_attempts`. The previously-asserted value was 20 with the old
    // buggy multiplier; the new correct value is 17.
    assert_eq!(budget.max_total_steps, 17);
    assert_eq!(
        budget.max_total_slots,
        u64::from(ResourceContract::DEFAULT.max_slots)
    );
    assert_eq!(budget.max_steps_executable, 17);
    assert_eq!(BoundednessPolicy::DEFAULT.validate(&budget), Ok(()));
    Ok(())
}

#[test]
fn given_step_count_overflow_when_budget_compute_runs_then_typed_workflow_error_returns() {
    let nodes = step_count_overflow_nodes();

    // Cold-AST-conservative iter count (master §45) caps the RepeatStart
    // multiplier to 1, so the deeply-nested u16::MAX repeats no longer
    // overflow `max_total_steps`. The declared `max_attempts` is still
    // tracked separately in `WholeWorkflowBudget.max_repeat_attempts`.
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT);

    let budget = result
        .expect("RepeatStart cold-AST-conservative iter count should prevent step-count overflow");
    assert_eq!(
        budget.max_total_steps, 5,
        "max_total_steps should be 1 (outer header) + 1 (inner header) + 1 (inner body) + 1 (inner body nop) + 1 (finish) = 5 with conservative iter count"
    );
    assert_eq!(
        budget.max_repeat_attempts,
        u16::MAX,
        "max_repeat_attempts should still track the declared u16::MAX"
    );
}

#[test]
fn given_each_adversarial_failure_path_when_executed_then_result_is_typed_not_panic_oom_or_timeout()
-> Result<(), String> {
    let workflow = two_step_workflow().map_err(|error| error.to_string())?;
    let mut run = new_run_frame(RunId::new(39), &workflow).map_err(|error| error.to_string())?;
    let mut store = ValueStore::with_max_slots(1);
    let empty_nodes: Box<[CompiledNode]> = Box::from([]);
    let overflow_nodes = step_count_overflow_nodes();

    assert_eq!(
        WholeWorkflowBudget::compute(&empty_nodes, StepIdx::new(0), &ResourceContract::DEFAULT),
        Err(WorkflowError::EntryOutOfBounds {
            entry: StepIdx::new(0),
        })
    );
    // Cold-AST-conservative iter count (master §45) caps the RepeatStart
    // multiplier to 1, so the deeply-nested u16::MAX repeats no longer
    // overflow `max_total_steps`. The previously-asserted value was a
    // `StepCountOverflow { actual: 4_294_967_297 }`; the budget now
    // returns `Ok` with a small, conservative `max_total_steps`.
    let overflow_budget =
        WholeWorkflowBudget::compute(&overflow_nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
            .expect(
                "RepeatStart cold-AST-conservative iter count should prevent step-count overflow",
            );
    assert_eq!(
        overflow_budget.max_total_steps, 5,
        "overflow_nodes with conservative iter count should produce max_total_steps=5"
    );
    assert_eq!(
        overflow_budget.max_repeat_attempts,
        u16::MAX,
        "max_repeat_attempts should still track the declared u16::MAX"
    );
    assert_eq!(
        BoundednessPolicy::DEFAULT.validate(&WholeWorkflowBudget {
            max_total_steps: BoundednessPolicy::DEFAULT.max_total_steps + 1,
            ..policy_limit_budget(BoundednessPolicy::DEFAULT)
        }),
        Err(BudgetError::TotalStepsExceeded {
            actual: BoundednessPolicy::DEFAULT.max_total_steps + 1,
            limit: BoundednessPolicy::DEFAULT.max_total_steps,
        })
    );
    assert_eq!(store.insert_symbol("one"), Ok(vb_core::SymbolId::new(0)));
    assert_eq!(
        store.insert_symbol("two"),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(
        run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store),
        Ok(EngineSignal::StepBudgetExhausted)
    );
    Ok(())
}

#[test]
fn given_capped_value_store_when_insertions_hit_cap_then_budget_exceeded_preserves_count() {
    let mut store = ValueStore::with_max_slots(1);

    assert_eq!(store.insert_symbol("first"), Ok(vb_core::SymbolId::new(0)));
    assert_budget_exceeded(store.insert_symbol("second"), 1);
    assert_eq!(store.total_arena_count(), 1);
    assert_eq!(store.max_arena_entries(), 1);
}

#[test]
fn given_value_growth_at_cap_when_next_insert_attempted_then_budget_exceeded_and_count_stays_capped()
-> Result<(), String> {
    let mut symbol_store = ValueStore::with_max_slots(1);
    assert_eq!(
        symbol_store.insert_symbol("first"),
        Ok(vb_core::SymbolId::new(0))
    );
    assert_eq!(
        symbol_store.insert_symbol("second"),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(symbol_store.total_arena_count(), 1);

    let mut list_store = ValueStore::with_max_slots(1);
    assert_eq!(
        list_store.insert_list(Box::from([SlotValue::I64(1)])),
        Ok(vb_core::ListId::new(0))
    );
    assert_eq!(
        list_store.insert_list(Box::from([SlotValue::I64(2)])),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(list_store.total_arena_count(), 1);

    let mut tainted_list_store = ValueStore::with_max_slots(1);
    assert_eq!(
        tainted_list_store
            .insert_list_with_taint(Box::from([SlotValue::I64(1)]), Box::from([Taint::Clean]),),
        Ok(vb_core::ListId::new(0))
    );
    assert_eq!(
        tainted_list_store
            .insert_list_with_taint(Box::from([SlotValue::I64(2)]), Box::from([Taint::Clean]),),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(tainted_list_store.total_arena_count(), 1);

    let mut object_store = ValueStore::with_max_slots(1);
    let field = ObjectField::clean(vb_core::SymbolId::new(0), SlotValue::I64(1));
    assert_eq!(
        object_store.insert_object(Box::from([field])),
        Ok(vb_core::ObjectId::new(0))
    );
    assert_eq!(
        object_store.insert_object(Box::from([field])),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(object_store.total_arena_count(), 1);

    let mut blob_store = ValueStore::with_max_slots(1);
    assert_eq!(
        blob_store.insert_blob(Bytes::from_static(b"a")),
        Ok(vb_core::BlobId::new(0))
    );
    assert_eq!(
        blob_store.insert_blob(Bytes::from_static(b"b")),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1,
        })
    );
    assert_eq!(blob_store.total_arena_count(), 1);
    Ok(())
}

#[test]
fn given_overlarge_payloads_when_inserted_then_resource_limit_exceeded_names_dimension() {
    let mut symbol_store = ValueStore::with_max_slots(u16::MAX);
    let symbol = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE + 1);
    assert_eq!(
        symbol_store.insert_symbol(symbol),
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        })
    );

    let mut list_store = ValueStore::with_max_slots(u16::MAX);
    let list = vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE + 1].into_boxed_slice();
    assert_eq!(
        list_store.insert_list(list),
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        })
    );

    let mut object_store = ValueStore::with_max_slots(u16::MAX);
    let field = ObjectField::clean(vb_core::SymbolId::new(0), SlotValue::Null);
    let fields = vec![field; MAX_OBJECT_FIELDS_PER_VALUE + 1].into_boxed_slice();
    assert_eq!(
        object_store.insert_object(fields),
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        })
    );

    let mut blob_store = ValueStore::with_max_slots(u16::MAX);
    let blob = Bytes::from(vec![0_u8; MAX_BLOB_BYTES_PER_VALUE + 1]);
    assert_eq!(
        blob_store.insert_blob(blob),
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        })
    );
}

#[test]
fn given_capped_store_when_success_and_failure_insertions_interleave_then_total_count_never_exceeds_cap()
 {
    let mut store = ValueStore::with_max_slots(3);

    assert_eq!(store.insert_symbol("a"), Ok(vb_core::SymbolId::new(0)));
    assert_eq!(store.total_arena_count(), 1);
    assert_eq!(
        store.insert_list(Box::from([SlotValue::I64(1)])),
        Ok(vb_core::ListId::new(0))
    );
    assert_eq!(store.total_arena_count(), 2);
    assert_eq!(
        store.insert_blob(Bytes::from_static(b"c")),
        Ok(vb_core::BlobId::new(0))
    );
    assert_eq!(store.total_arena_count(), 3);
    assert_eq!(
        store.insert_symbol("d"),
        Err(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 3,
        })
    );
    assert_eq!(store.total_arena_count(), 3);
    assert_eq!(store.max_arena_entries(), 3);
}

#[test]
fn given_larger_nested_dimensions_when_budget_computed_then_aggregate_bound_does_not_decrease()
-> Result<(), String> {
    let smaller = finite_nested_budget(2).map_err(|error| error.to_string())?;
    let larger = finite_nested_budget(3).map_err(|error| error.to_string())?;

    assert_eq!(larger.max_total_steps >= smaller.max_total_steps, true);
    assert_eq!(larger.max_total_slots >= smaller.max_total_slots, true);
    assert_eq!(larger.max_fanout >= smaller.max_fanout, true);
    assert_eq!(larger.max_nesting_depth >= smaller.max_nesting_depth, true);
    assert_eq!(
        larger.max_parallel_in_flight >= smaller.max_parallel_in_flight,
        true
    );
    assert_eq!(
        larger.max_action_tickets >= smaller.max_action_tickets,
        true
    );
    assert_eq!(larger.max_gather_pages >= smaller.max_gather_pages, true);
    assert_eq!(larger.max_gather_items >= smaller.max_gather_items, true);
    assert_eq!(
        larger.max_steps_executable >= smaller.max_steps_executable,
        true
    );
    Ok(())
}

#[test]
fn given_malformed_resource_budget_bytes_when_fuzzed_then_no_panic_and_input_stays_bounded() {
    let seeds: &[&[u8]] = &[
        b"",
        b"\0\0\0\0",
        b"\xff\xff\xff\xff\xff\xff\xff\xff",
        b"resource-budget-overflow-marker",
    ];

    for seed in seeds {
        let decoded = postcard::from_bytes::<AggregateResourceBudget>(seed);
        let expected = decoded.map_err(|_| CoreError::InvalidCompiledWorkflow {
            reason: "malformed_resource_budget_bytes",
        });
        match expected {
            Ok(budget) => {
                assert_eq!(budget.max_steps_executable <= u32::MAX, true);
            }
            Err(error) => {
                assert_eq!(
                    error,
                    CoreError::InvalidCompiledWorkflow {
                        reason: "malformed_resource_budget_bytes",
                    }
                );
            }
        }
    }
}

proptest! {
    #[test]
    fn proptest_step_budget_new_clamps_and_try_take_is_monotonic(
        initial in any::<u64>(),
        takes in 0_u64..=128,
    ) {
        let mut budget = StepBudget::new(initial);
        let expected_start = initial.min(MAX_STEP_BUDGET);
        prop_assert_eq!(budget.remaining(), expected_start);

        let mut observed_remaining = expected_start;
        let mut completed = 0_u64;
        while completed < takes {
            let result = budget.try_take();
            if observed_remaining == 0 {
                prop_assert_eq!(result, Ok(false));
                prop_assert_eq!(budget.remaining(), 0);
            } else {
                prop_assert_eq!(result, Ok(true));
                observed_remaining = observed_remaining.saturating_sub(1);
                prop_assert_eq!(budget.remaining(), observed_remaining);
            }
            completed = completed.saturating_add(1);
        }
    }

    #[test]
    fn proptest_boundedness_policy_validate_rejects_one_over_dimension(
        dimension in 0_u8..9,
    ) {
        let policy = BoundednessPolicy::DEFAULT;
        let mut budget = policy_limit_budget(policy);

        match dimension {
            0 => {
                budget.max_total_steps = policy.max_total_steps + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::TotalStepsExceeded { actual: policy.max_total_steps + 1, limit: policy.max_total_steps }));
            }
            1 => {
                budget.max_total_slots = policy.max_total_slots + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::TotalSlotsExceeded { actual: policy.max_total_slots + 1, limit: policy.max_total_slots }));
            }
            2 => {
                budget.max_fanout = policy.max_fanout + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::FanoutExceeded { actual: policy.max_fanout + 1, limit: policy.max_fanout }));
            }
            3 => {
                budget.max_nesting_depth = policy.max_nesting_depth + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::NestingDepthExceeded { actual: policy.max_nesting_depth + 1, limit: policy.max_nesting_depth }));
            }
            4 => {
                budget.max_action_tickets = policy.absolute_max_action_tickets + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::ActionTicketsExceeded { actual: policy.absolute_max_action_tickets + 1, limit: policy.absolute_max_action_tickets }));
            }
            5 => {
                budget.max_parallel_in_flight = policy.absolute_max_parallel + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::ParallelExceeded { actual: policy.absolute_max_parallel + 1, limit: policy.absolute_max_parallel }));
            }
            6 => {
                budget.max_run_time_seconds = policy.absolute_max_run_time_seconds + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::RunTimeExceeded { actual: policy.absolute_max_run_time_seconds + 1, limit: policy.absolute_max_run_time_seconds }));
            }
            7 => {
                budget.max_result_bytes = policy.absolute_max_result_bytes + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::ResultBytesExceeded { actual: policy.absolute_max_result_bytes + 1, limit: policy.absolute_max_result_bytes }));
            }
            _ => {
                budget.max_steps_executable = policy.absolute_max_steps_executable + 1;
                prop_assert_eq!(policy.validate(&budget), Err(BudgetError::StepsExecutableExceeded { actual: policy.absolute_max_steps_executable + 1, limit: policy.absolute_max_steps_executable }));
            }
        }
    }

    #[test]
    fn proptest_capped_value_store_interleavings_preserve_cap(
        operations in prop::collection::vec(0_u8..4, 0..16),
    ) {
        let mut store = ValueStore::with_max_slots(3);
        let mut observed_count = 0_u64;

        for operation in operations {
            let before = store.total_arena_count();
            let result = match operation {
                0 => store.insert_symbol("prop").map(|_| ()),
                1 => store.insert_list(Box::from([SlotValue::I64(7)])).map(|_| ()),
                2 => store.insert_object(Box::from([ObjectField::clean(vb_core::SymbolId::new(0), SlotValue::Null)])).map(|_| ()),
                _ => store.insert_blob(Bytes::from_static(b"p")).map(|_| ()),
            };
            match result {
                Ok(()) => {
                    observed_count = observed_count.saturating_add(1);
                    prop_assert_eq!(store.total_arena_count(), before.saturating_add(1));
                    prop_assert_eq!(store.total_arena_count(), observed_count);
                }
                Err(error) => {
                    prop_assert_eq!(error, CoreError::BudgetExceeded { budget: "max_slots", limit: 3 });
                    prop_assert_eq!(store.total_arena_count(), before);
                    prop_assert_eq!(store.total_arena_count() <= store.max_arena_entries(), true);
                }
            }
        }
    }
}

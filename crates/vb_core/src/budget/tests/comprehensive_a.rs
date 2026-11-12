// =========================================================================
// Comprehensive test coverage for budget.rs
// =========================================================================

const fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants: 1,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 0,
        max_fanout: 64,
        max_collect_items: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

const fn test_budget(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> WholeWorkflowBudget {
    WholeWorkflowBudget {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 3,
        max_trace_events: 4,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 5,
        max_blob_bytes: 6,
        max_input_bytes: 7,
        max_queue_depth: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    }
}

#[test]
fn payload_budget_reports_first_exceeded_dimension_in_check_order() -> Result<(), String> {
    let policy = BoundednessPolicy {
        absolute_max_journal_batch_bytes: 10,
        absolute_max_queue_depth: 10,
        absolute_max_ipc_payload_bytes: 10,
        absolute_max_blob_bytes: 10,
        absolute_max_input_bytes: 10,
        ..BoundednessPolicy::DEFAULT
    };
    let mut both_over = test_budget(1, 1, 0, 0);
    both_over.max_blob_bytes = 11;
    both_over.max_input_bytes = 11;
    ensure_equal(
        policy.validate(&both_over),
        Err(BudgetError::BlobBytesExceeded {
            actual: 11,
            limit: 10,
        }),
    )?;
    let mut input_only = test_budget(1, 1, 0, 0);
    input_only.max_input_bytes = 11;
    ensure_equal(
        policy.validate(&input_only),
        Err(BudgetError::InputBytesExceeded {
            actual: 11,
            limit: 10,
        }),
    )
}

const fn test_policy(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> BoundednessPolicy {
    BoundednessPolicy {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    }
}

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

fn single_node_workflow() -> Vec<CompiledNode> {
    vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )]
}

fn test_node(id: u16, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    node(StepIdx::new(id), kind, None, next.map(StepIdx::new))
}

fn finish_node(id: u16) -> CompiledNode {
    test_node(
        id,
        None,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    )
}

fn nested_fanout_loop_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(5),
            },
        ),
        test_node(
            1,
            None,
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(2), StepIdx::new(3)].into_boxed_slice(),
                join: StepIdx::new(4),
            },
        ),
        test_node(2, None, CompiledNodeKind::Nop),
        test_node(3, None, CompiledNodeKind::Nop),
        test_node(
            4,
            None,
            CompiledNodeKind::TogetherJoin {
                branch_count: 2.into(),
                accumulator: SlotIdx::new(2),
            },
        ),
        test_node(
            5,
            Some(6),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
        ),
        finish_node(6),
    ]
}

fn sequential_collect_reduce_repeat_wait_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 3.into(),
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        ),
        test_node(1, None, CompiledNodeKind::Nop),
        test_node(
            2,
            Some(3),
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(2),
            },
        ),
        test_node(
            3,
            None,
            CompiledNodeKind::ReduceStart {
                limit: 65_535,
                input: SlotIdx::new(2),
                accumulator: SlotIdx::new(3),
                initial: ConstIdx::new(0),
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        ),
        test_node(4, None, CompiledNodeKind::Nop),
        test_node(
            5,
            Some(6),
            CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(3),
            },
        ),
        test_node(
            6,
            None,
            CompiledNodeKind::RepeatStart {
                max_attempts: 2.into(),
                body: StepIdx::new(7),
                done: StepIdx::new(8),
            },
        ),
        test_node(
            7,
            None,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(4),
            },
        ),
        test_node(
            8,
            Some(9),
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(5),
            },
        ),
        finish_node(9),
    ]
}

fn conditional_max_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(3),
                    },
                ]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(5)).into(),
            },
        ),
        test_node(1, Some(2), CompiledNodeKind::Nop),
        finish_node(2),
        test_node(3, Some(4), CompiledNodeKind::Nop),
        test_node(4, Some(5), CompiledNodeKind::Nop),
        finish_node(5),
    ]
}

// -------------------------------------------------------------------------
// 1. Step budget creation and validation
// -------------------------------------------------------------------------

#[test]
fn step_budget_creation_at_one() -> Result<(), String> {
    let b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)
}

#[test]
fn step_budget_creation_at_max() -> Result<(), String> {
    let b = StepBudget::new(crate::limits::MAX_STEP_BUDGET);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

#[test]
fn step_budget_creation_at_zero() -> Result<(), String> {
    let b = StepBudget::new(0);
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_creation_clamps_large_value() -> Result<(), String> {
    let b = StepBudget::new(u64::MAX);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// 2. Budget consumption tracking (single step, multi-step)
// -------------------------------------------------------------------------

#[test]
fn step_budget_single_consumption_decrements() -> Result<(), String> {
    let mut b = StepBudget::new(5);
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 4)
}

#[test]
fn step_budget_multi_step_consumption_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(3);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 2)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 1)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_consumption_returns_true_each_time_until_exhausted() -> Result<(), String> {
    let mut b = StepBudget::new(4);
    for i in 0..4 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, true)?;
        ensure_equal(b.remaining(), 3 - i)?;
    }
    let final_take = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(final_take, false)
}

// -------------------------------------------------------------------------
// 3. Budget exhaustion detection
// -------------------------------------------------------------------------

#[test]
fn step_budget_exhausted_returns_false() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, false)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_exhaustion_stays_at_zero() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    for _ in 0..5 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

// -------------------------------------------------------------------------
// 4. Sub-graph budget accounting
// -------------------------------------------------------------------------

// 4a. ForEach body cost multiplication with limit=1 (single iteration)
#[test]
fn foreach_limit_one_counts_body_once() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// 4b. Together branch budget counts all branches
#[test]
fn together_start_counts_parallel_branches() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(3), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(4), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(5),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(6, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_fanout, 4)?;
    ensure_equal(budget.max_parallel_in_flight, 4)?;
    ensure_equal(budget.max_together_branches, 4)?;
    ensure_equal(budget.max_total_steps, 6)
}

// 4c. Collect loop body cost
#[test]
fn collect_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 3.into(),
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 5)?;
    ensure_equal(budget.max_gather_pages, 3)?;
    ensure_equal(budget.max_gather_items, 3)
}

// 4d. Reduce body cost
#[test]
fn reduce_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ReduceStart {
                limit: 65_535,
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let expected_iters = u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
    ensure_equal(budget.max_total_steps, 1 + expected_iters + 1)
}

// 4e. Repeat body cost
#[test]
fn repeat_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 7.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 9)?;
    ensure_equal(budget.max_repeat_attempts, 7)
}

// -------------------------------------------------------------------------
// 5. Max step budget boundary (exactly at limit, one step over)
// -------------------------------------------------------------------------

#[test]
fn policy_allows_budget_at_exact_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_000, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_rejects_budget_one_over_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_001, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_001)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_fanout() -> Result<(), String> {
    let budget = test_budget(1, 0, 64, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_fanout_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 65, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded { actual, limit }) => {
            ensure_equal(actual, 65)?;
            ensure_equal(limit, 64)
        }
        other => Err(format!("expected FanoutExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_nesting_depth() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 8);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_nesting_depth_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 9);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => {
            ensure_equal(actual, 9)?;
            ensure_equal(limit, 8)
        }
        other => Err(format!("expected NestingDepthExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// 6. Budget reset/reinitialization
// -------------------------------------------------------------------------

#[test]
fn step_budget_recreated_after_exhaustion() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(b.remaining(), 0)?;

    let mut b2 = StepBudget::new(2);
    ensure_equal(b2.remaining(), 2)?;
    ensure_equal(b2.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b2.remaining(), 1)
}

#[test]
fn whole_workflow_budget_recompute_produces_same_result() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 1);

    let budget1 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let budget2 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget1, budget2)
}

// -------------------------------------------------------------------------
// 7. Nested loop budget computation
// -------------------------------------------------------------------------

#[test]
fn nested_for_each_triple_depth() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 3.into(),
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(4),
                item_slot: SlotIdx::new(5),
                limit: 4.into(),
                body: StepIdx::new(3),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        node(StepIdx::new(3), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(4),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(6),
            },
            Some(SlotIdx::new(6)),
            None,
        ),
        node(
            StepIdx::new(5),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(7),
            },
            Some(SlotIdx::new(7)),
            None,
        ),
        node(
            StepIdx::new(6),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(8),
            },
            Some(SlotIdx::new(8)),
            None,
        ),
    ];
    let contract = test_contract(7, 9);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_nesting_depth, 3)?;
    ensure_equal(budget.max_total_steps > 0, true)
}

// -------------------------------------------------------------------------
// 8. Parallel branch budget splitting (TogetherStart)
// -------------------------------------------------------------------------

#[test]
fn together_start_tracks_max_parallel_in_flight() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(4, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 2)?;
    ensure_equal(budget.max_parallel_in_flight, 2)?;
    ensure_equal(budget.max_together_branches, 2)?;
    ensure_equal(budget.max_total_steps, 4)
}

#[test]
fn larger_together_start_dominates_fanout() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(3),
            CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(4),
                    StepIdx::new(5),
                    StepIdx::new(6),
                    StepIdx::new(7),
                    StepIdx::new(8),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(9),
            },
            None,
            None,
        ),
        node(StepIdx::new(4), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(5), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(6), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(7), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(8), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(9),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(10, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 5)?;
    ensure_equal(budget.max_parallel_in_flight, 5)?;
    ensure_equal(budget.max_together_branches, 5)
}

// -------------------------------------------------------------------------
// 9. Zero-budget edge cases
// -------------------------------------------------------------------------

#[test]
fn step_budget_zero_never_allows_consumption() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..10 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

#[test]
fn whole_workflow_budget_zero_slots_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 0);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 0)
}

#[test]
fn policy_validate_accepts_zero_budget() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(BoundednessPolicy::DEFAULT.validate(&budget), Ok(()))
}

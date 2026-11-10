// -------------------------------------------------------------------------
// 10. Budget arithmetic overflow protection
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_total_slots_derives_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 500);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 500)
}

#[test]
fn whole_workflow_budget_result_bytes_derive_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_output_bytes = 9999;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_result_bytes, 9999)
}

#[test]
fn whole_workflow_budget_max_retries_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_retry_attempts = 7;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_retries_per_action, 7)
}

#[test]
fn policy_rejects_action_tickets_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_action_tickets = 200_000;
    let policy = BoundednessPolicy {
        absolute_max_action_tickets: 100_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ActionTicketsExceeded { actual, limit }) => {
            ensure_equal(actual, 200_000)?;
            ensure_equal(limit, 100_000)
        }
        other => Err(format!("expected ActionTicketsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_parallel_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_parallel_in_flight = 512;
    let policy = BoundednessPolicy {
        absolute_max_parallel: 256,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ParallelExceeded { actual, limit }) => {
            ensure_equal(actual, 512)?;
            ensure_equal(limit, 256)
        }
        other => Err(format!("expected ParallelExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_result_bytes_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_result_bytes = 1_000_000;
    let policy = BoundednessPolicy {
        absolute_max_result_bytes: 262_144,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ResultBytesExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_000)?;
            ensure_equal(limit, 262_144)
        }
        other => Err(format!("expected ResultBytesExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_run_time_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_run_time_seconds = 5_000_000;
    let policy = BoundednessPolicy {
        absolute_max_run_time_seconds: 2_592_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::RunTimeExceeded { actual, limit }) => {
            ensure_equal(actual, 5_000_000)?;
            ensure_equal(limit, 2_592_000)
        }
        other => Err(format!("expected RunTimeExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_steps_executable_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_steps_executable = 2_000_000;
    let policy = BoundednessPolicy {
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::StepsExecutableExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected StepsExecutableExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: Do node action ticket counting
// -------------------------------------------------------------------------

#[test]
fn do_node_increments_action_tickets() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
            },
            None,
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 2);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 2)?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEach iteration accumulation
// -------------------------------------------------------------------------

#[test]
fn multiple_for_each_accumulates_iterations() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(2),
            },
            Some(SlotIdx::new(2)),
            None,
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(3),
                item_slot: SlotIdx::new(4),
                limit: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(StepIdx::new(4), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(5),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
            Some(SlotIdx::new(5)),
            None,
        ),
    ];
    let mut nodes = nodes;
    nodes[2].routing.next = NodeContinuation::Step(StepIdx::new(3));

    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_for_each_iterations, 15)?;
    ensure_equal(budget.max_total_steps, 19)
}

// -------------------------------------------------------------------------
// Additional coverage: Jump node handling
// -------------------------------------------------------------------------

#[test]
fn jump_chain_counts_all_nodes() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Jump {
                target: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

#[test]
fn jump_self_cycle_detected() -> Result<(), String> {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Jump {
            target: StepIdx::new(0),
        },
        None,
        None,
    )];
    let contract = test_contract(1, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(0))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy::DEFAULT sanity checks
// -------------------------------------------------------------------------

#[test]
fn default_policy_total_steps_is_one_million() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_total_steps, 1_000_000)
}

#[test]
fn default_policy_max_fanout_is_64() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_fanout, 64)
}

#[test]
fn default_policy_nesting_depth_is_8() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_nesting_depth, 8)
}

// -------------------------------------------------------------------------
// Additional coverage: BudgetError Display and Error trait
// -------------------------------------------------------------------------

#[test]
fn budget_error_implements_std_error() -> Result<(), String> {
    let err = BudgetError::TotalStepsExceeded {
        actual: 10,
        limit: 5,
    };
    let _: &dyn std::error::Error = &err;
    Ok(())
}

#[test]
fn budget_error_total_slots_display() -> Result<(), String> {
    let err = BudgetError::TotalSlotsExceeded {
        actual: 100,
        limit: 50,
    };
    ensure_equal(
        format!("{err}"),
        "total slots exceeded: 100 > 50".to_string(),
    )
}

#[test]
fn budget_error_parallel_display() -> Result<(), String> {
    let err = BudgetError::ParallelExceeded {
        actual: 300,
        limit: 256,
    };
    ensure_equal(format!("{err}"), "parallel exceeded: 300 > 256".to_string())
}

#[test]
fn budget_error_action_tickets_display() -> Result<(), String> {
    let err = BudgetError::ActionTicketsExceeded {
        actual: 150_000,
        limit: 100_000,
    };
    ensure_equal(
        format!("{err}"),
        "action tickets exceeded: 150000 > 100000".to_string(),
    )
}

#[test]
fn budget_error_run_time_display() -> Result<(), String> {
    let err = BudgetError::RunTimeExceeded {
        actual: 5_000_000,
        limit: 2_592_000,
    };
    ensure_equal(
        format!("{err}"),
        "run time exceeded: 5000000 > 2592000".to_string(),
    )
}

#[test]
fn budget_error_result_bytes_display() -> Result<(), String> {
    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    ensure_equal(
        format!("{err}"),
        "result bytes exceeded: 524288 > 262144".to_string(),
    )
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget Copy and Clone
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_is_copy() -> Result<(), String> {
    let budget = test_budget(10, 100, 4, 2);
    let copy = budget;
    ensure_equal(budget, copy)
}

#[test]
fn boundedness_policy_is_copy() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let copy = policy;
    ensure_equal(policy, copy)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEachStart limit=1 does not overcount
// -------------------------------------------------------------------------

#[test]
fn foreach_limit_one_exact_step_count() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1,
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

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart max_attempts=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn repeat_start_zero_attempts_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 0,
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

// -------------------------------------------------------------------------
// Additional coverage: Linear chain with varied node types
// -------------------------------------------------------------------------

#[test]
fn linear_chain_set_const_copy_eval() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
            Some(SlotIdx::new(2)),
            Some(StepIdx::new(3)),
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 4)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

// -------------------------------------------------------------------------
// Additional coverage: CollectStart limit=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn collect_start_zero_limit_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 0,
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
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: Multi-body ForEach (body with 3 steps)
// -------------------------------------------------------------------------

#[test]
fn foreach_multi_step_body() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(3)),
        ),
        node(StepIdx::new(3), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(4),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(5, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 11)
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart with max_attempts=1
// -------------------------------------------------------------------------

#[test]
fn repeat_start_one_attempt() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 1,
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
    ensure_equal(budget.max_total_steps, 3)?;
    ensure_equal(budget.max_repeat_attempts, 1)
}

// -------------------------------------------------------------------------
// Additional coverage: Policy validates first violation only
// -------------------------------------------------------------------------

#[test]
fn policy_reports_first_violation_steps_over_slots_over() -> Result<(), String> {
    let mut budget = test_budget(2_000_000, 200_000, 100, 20);
    budget.max_action_tickets = 500_000;
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget max_steps_executable derivation
// -------------------------------------------------------------------------

#[test]
fn max_steps_executable_is_fixed_ceiling_independent_of_total_steps() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(2, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 2)?;
    ensure_equal(
        budget.max_steps_executable,
        WholeWorkflowBudget::MAX_STEPS_EXECUTABLE,
    )?;
    ensure_equal(budget.max_steps_executable, 32_768)
}

// -------------------------------------------------------------------------
// Additional coverage: max_total_slots_written equals contract max_slots
// -------------------------------------------------------------------------

#[test]
fn max_total_slots_written_equals_contract_max_slots() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 42);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots_written, 42)
}

// -------------------------------------------------------------------------
// Additional coverage: ErrorHandler node step counting
// -------------------------------------------------------------------------

#[test]
fn error_handler_counts_body_and_handler() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy validate returns checks in order
// -------------------------------------------------------------------------

#[test]
fn policy_check_order_total_steps_before_slots() -> Result<(), String> {
    let budget = test_budget(2_000_000, 200_000, 0, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalStepsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalStepsExceeded (first check), got {other:?}"
        )),
    }
}

#[test]
fn policy_check_order_slots_before_fanout() -> Result<(), String> {
    let budget = test_budget(100, 200_000, 100, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalSlotsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalSlotsExceeded (second check), got {other:?}"
        )),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart max_attempts tracking uses max not add
// -------------------------------------------------------------------------

#[test]
fn repeat_start_max_attempts_tracks_maximum_not_sum() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(2),
            CompiledNodeKind::RepeatStart {
                max_attempts: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
            None,
            Some(StepIdx::new(3)),
        ),
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
    let contract = test_contract(6, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_repeat_attempts, 10)
}

// -------------------------------------------------------------------------
// Additional coverage: StepBudget::MAX is const
// -------------------------------------------------------------------------

#[test]
fn step_budget_max_is_const_compatible() -> Result<(), String> {
    const _MAX: StepBudget = StepBudget::MAX;
    ensure_equal(_MAX.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget debug format
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_debug_format() -> Result<(), String> {
    let budget = test_budget(10, 20, 4, 2);
    let debug = format!("{budget:?}");
    ensure_equal(debug.contains("WholeWorkflowBudget"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn boundedness_policy_debug_format() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let debug = format!("{policy:?}");
    ensure_equal(debug.contains("BoundednessPolicy"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn budget_error_debug_format() -> Result<(), String> {
    let err = BudgetError::FanoutExceeded {
        actual: 5,
        limit: 3,
    };
    let debug = format!("{err:?}");
    ensure_equal(debug.contains("FanoutExceeded"), true)
}

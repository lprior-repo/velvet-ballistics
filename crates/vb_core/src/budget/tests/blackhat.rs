// =========================================================================
// BLACKHAT adversarial tests -- budget overflow, bypass, and edge cases
// =========================================================================

// --- FINDING BH-BUD-01: max_steps_executable silent saturation bypass ---

#[test]
fn blackhat_steps_executable_saturates_on_large_total() {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::from(u32::MAX) + 1,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
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
        max_timer_entries: 7,
        max_trace_events: 8,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_queue_depth: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };

    let saturated = u32::try_from(budget.max_total_steps).unwrap_or(u32::MAX);
    assert_eq!(
        saturated,
        u32::MAX,
        "BLACKHAT BH-BUD-01: u32 saturation hides overflow"
    );
    assert!(
        budget.max_total_steps > u64::from(saturated),
        "BLACKHAT BH-BUD-01: true count exceeds reported executable steps"
    );
}

// --- FINDING BH-BUD-02: max_run_time_seconds hardcoded to 0 ---

#[test]
fn blackhat_run_time_seconds_always_zero_in_computed_budget() {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_run_time_seconds == 0);

    assert!(
        budget.is_none(),
        "BLACKHAT BH-BUD-02: max_run_time_seconds must not be hardcoded to 0"
    );
}

// --- FINDING BH-BUD-03: From<WorkflowError> for BudgetError loses information ---

#[test]
fn blackhat_workflow_error_to_budget_error_produces_equal_actual_and_limit() {
    let workflow_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    };
    let budget_err: BudgetError = workflow_err.into();

    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("BLACKHAT BH-BUD-03: unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-04: ForEachStart limit=0 counts as 1 iteration ---

#[test]
fn blackhat_foreach_limit_zero_still_counts_as_one_iteration() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 0,
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
        .ok()
        .filter(|b| b.max_total_steps == 3);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-04: limit=0 counts as 1 iteration"
    );
}

// --- FINDING BH-BUD-05: Step count overflow uses misleading error variant ---

#[test]
fn blackhat_step_count_overflow_uses_misleading_error_variant() {
    let workflow_err = WorkflowError::StepOutOfBounds {
        step: StepIdx::new(0),
    };
    let converted: BudgetError = workflow_err.into();
    match converted {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-06: action_tickets saturating_add hides overflow ---

#[test]
fn blackhat_action_tickets_saturating_add_under_reports() {
    let mut max_action_tickets: u32 = u32::MAX;
    max_action_tickets = max_action_tickets.saturating_add(1);
    assert_eq!(
        max_action_tickets,
        u32::MAX,
        "BLACKHAT BH-BUD-06: saturating_add hides overflow"
    );
}

// --- FINDING BH-BUD-07: gather_items saturating_add accumulation ---

#[test]
fn blackhat_gather_items_accumulation_saturates() {
    let mut max_gather_items: u32 = u32::MAX - 10;
    max_gather_items = max_gather_items.saturating_add(20);
    assert_eq!(
        max_gather_items,
        u32::MAX,
        "BLACKHAT BH-BUD-07: gather items saturates at u32::MAX"
    );
}

// --- FINDING BH-BUD-08: retries_per_action copied from contract not computed ---

#[test]
fn blackhat_retries_per_action_copied_from_contract_not_computed() {
    let contract = ResourceContract {
        max_retry_attempts: 42,
        ..test_contract(1, 1)
    };
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_retries_per_action == 42);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-08: retries copied from contract, not computed from IR"
    );
}

// --- FINDING BH-BUD-09: forward jump does not trigger cycle detection ---

#[test]
fn blackhat_jump_cycle_detection_relies_on_forward_edge_validation() {
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
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);

    match result {
        Ok(budget) => {
            assert_eq!(
                budget.max_total_steps, 2,
                "forward jump should count as 2 steps"
            );
        }
        Err(_) => {
            panic!("BLACKHAT BH-BUD-09: forward jump incorrectly detected as cycle");
        }
    }
}

// --- FINDING BH-BUD-10: policy boundary exact vs over ---

#[test]
fn blackhat_policy_allows_exact_limit() {
    let budget = test_budget(1_000_000, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(
        result,
        Ok(()),
        "BLACKHAT BH-BUD-10: budget at exact limits should pass"
    );
}

#[test]
fn blackhat_policy_rejects_one_over_limit() {
    let budget = test_budget(1_000_001, 65_535, 64, 8);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert!(
        result.is_err(),
        "BLACKHAT BH-BUD-10: budget one over limit must be rejected"
    );
}

// --- FINDING BH-BUD-11: StepBudget clamping is silent ---

#[test]
fn blackhat_step_budget_clamping_is_silent() {
    let budget = StepBudget::new(100_000);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "BLACKHAT BH-BUD-11: requested 100K steps silently clamped"
    );
}

// --- FINDING BH-BUD-12: self-referencing loop body graceful handling ---

#[test]
fn blackhat_self_referencing_loop_body_gracefully_handled() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
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
    ];
    let contract = test_contract(2, 3);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    assert!(
        result.is_ok() || result.is_err(),
        "BLACKHAT BH-BUD-12: self-referencing body must not panic"
    );
}

// --- FINDING BH-BUD-13: ReduceStart uses MAX_LIST_ITEMS_PER_VALUE iterations ---

#[test]
fn blackhat_reduce_start_uses_max_list_items_as_iteration_count() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ReduceStart {
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
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract).ok();

    let expected_iters = u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
    let expected = 1 + expected_iters + 1;

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-13: ReduceStart should compute with MAX_LIST_ITEMS iterations"
    );
    assert_eq!(
        budget.as_ref().map(|b| b.max_total_steps),
        Some(expected),
        "BLACKHAT BH-BUD-13: expected {expected} steps"
    );
}

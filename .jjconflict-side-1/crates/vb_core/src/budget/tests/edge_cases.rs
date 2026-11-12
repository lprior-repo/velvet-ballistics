// =========================================================================
// Additional edge-case tests — Budget construction, checked operations
// =========================================================================

#[test]
fn whole_workflow_budget_zero_fields_is_valid() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(budget.max_total_steps, 0)?;
    ensure_equal(budget.max_total_slots, 0)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

#[test]
fn whole_workflow_budget_max_fields() -> Result<(), String> {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::MAX,
        max_total_slots: u64::MAX,
        max_fanout: u16::MAX,
        max_nesting_depth: u16::MAX,
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
        max_timer_entries: u32::MAX,
        max_trace_events: u64::MAX,
        max_journal_batch_bytes: u32::MAX,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: u32::MAX,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    ensure_equal(budget.max_total_steps, u64::MAX)?;
    ensure_equal(budget.max_total_slots, u64::MAX)?;
    ensure_equal(budget.max_fanout, u16::MAX)?;
    ensure_equal(budget.max_nesting_depth, u16::MAX)
}

#[test]
fn boundedness_policy_default_values_are_sensible() -> Result<(), String> {
    let p = BoundednessPolicy::DEFAULT;
    ensure_equal(p.max_total_steps, 1_000_000)?;
    ensure_equal(p.max_total_slots, 65_535)?;
    ensure_equal(p.max_fanout, 64)?;
    ensure_equal(p.max_nesting_depth, 8)?;
    ensure_equal(p.absolute_max_action_tickets, 100_000)?;
    ensure_equal(p.absolute_max_parallel, 256)?;
    ensure_equal(p.absolute_max_run_time_seconds, 2_592_000)?;
    ensure_equal(p.absolute_max_result_bytes, 262_144)?;
    ensure_equal(p.absolute_max_steps_executable, 1_000_000)
}

#[test]
fn budget_error_all_variants_display_non_empty() -> Result<(), String> {
    let errors = [
        BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::TotalSlotsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::FanoutExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::NestingDepthExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ParallelExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ActionTicketsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::RunTimeExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ResultBytesExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::StepsExecutableExceeded {
            actual: 1,
            limit: 0,
        },
    ];
    for err in &errors {
        let display = format!("{err}");
        if display.is_empty() {
            return Err(format!("BudgetError display is empty for {err:?}"));
        }
    }
    Ok(())
}

#[test]
fn budget_error_equality_same_variants() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    ensure_equal(a, b)
}

#[test]
fn budget_error_inequality_different_actual() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 6,
        limit: 3,
    };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn budget_error_clone_preserves_equality() -> Result<(), String> {
    let a = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 5,
    };
    let b = a.clone();
    ensure_equal(a, b)
}

#[test]
fn budget_error_from_workflow_error_preserves_variant() -> Result<(), String> {
    let wf_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel for workflow error, got {other:?}"
        )),
    }
}

#[test]
fn step_budget_new_one_consumes_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)?;
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 0)?;
    let taken2 = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken2, false)
}

#[test]
fn step_budget_try_take_never_panics() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..100 {
        let result = b.try_take();
        match result {
            Ok(false) => {}
            other => return Err(format!("expected Ok(false), got {other:?}")),
        }
    }
    Ok(())
}

#[test]
fn boundedness_policy_custom_zero_limits_accept_zero_budget() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
        ..BoundednessPolicy::DEFAULT
    };
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn boundedness_policy_custom_zero_limits_reject_nonzero() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
        ..BoundednessPolicy::DEFAULT
    };
    let budget = test_budget(1, 0, 0, 0);
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        }) => Ok(()),
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

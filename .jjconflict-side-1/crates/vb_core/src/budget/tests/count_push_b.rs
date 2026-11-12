// -------------------------------------------------------------------------
// validate_aggregate_budget tests
// -------------------------------------------------------------------------

#[test]
fn validate_aggregate_budget_accepts_valid_budget() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    ensure_equal(
        crate::budget::validate_aggregate_budget(&budget, &policy),
        Ok(()),
    )
}

#[test]
fn validate_aggregate_budget_reports_extended_payload_dimensions() -> Result<(), String> {
    let policy = BoundednessPolicy {
        absolute_max_timer_entries: 12,
        absolute_max_trace_events: 13,
        absolute_max_queue_depth: 14,
        absolute_max_journal_batch_bytes: 15,
        absolute_max_ipc_payload_bytes: 16,
        absolute_max_blob_bytes: 17,
        absolute_max_input_bytes: 18,
        ..BoundednessPolicy::DEFAULT
    };
    let base = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 12,
        max_trace_events: 13,
        max_queue_depth: 14,
        max_journal_batch_bytes: 15,
        max_ipc_payload_bytes: 16,
        max_blob_bytes: 17,
        max_input_bytes: 18,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };

    let mut over = base;
    over.max_timer_entries = 13;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_timer_entries",
        13,
        12,
    )?;
    over = base;
    over.max_trace_events = 14;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_trace_events",
        14,
        13,
    )?;
    over = base;
    over.max_journal_batch_bytes = 16;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_journal_batch_bytes",
        16,
        15,
    )?;
    over = base;
    over.max_queue_depth = 15;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_queue_depth",
        15,
        14,
    )?;
    over = base;
    over.max_ipc_payload_bytes = 17;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_ipc_payload_bytes",
        17,
        16,
    )?;
    over = base;
    over.max_blob_bytes = 18;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_blob_bytes",
        18,
        17,
    )?;
    over = base;
    over.max_input_bytes = 19;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_input_bytes",
        19,
        18,
    )
}

fn assert_policy_exceeded(
    actual: Result<(), AggregateBudgetError>,
    expected_resource: &'static str,
    expected_actual: u64,
    expected_limit: u64,
) -> Result<(), String> {
    match actual {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        }) if resource == expected_resource
            && actual == expected_actual
            && limit == expected_limit =>
        {
            Ok(())
        }
        other => Err(format!(
            "expected PolicyExceeded {{ resource: {expected_resource}, actual: {expected_actual}, limit: {expected_limit} }}, got {other:?}"
        )),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_steps() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 2_000_000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_action_tickets() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 200_000,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_parallel_in_flight() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 512,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_run_time() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3_000_000,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_run_time_seconds")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_result_bytes() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 300_000,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_result_bytes",
            actual: 300_000,
            limit: 262_144,
        }) => Ok(()),
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_total_slots() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 100_000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
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
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_together_branches() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 100,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
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
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_together_branches")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// validate_step_ceilings tests
// -------------------------------------------------------------------------

#[test]
fn validate_step_ceilings_accepts_valid() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

#[test]
fn validate_step_ceilings_rejects_zero_step_budget() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded(0), got {:?}", other)),
    }
}

#[test]
fn validate_step_ceilings_rejects_zero_transitions() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 0,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!(
            "expected PerTickCeilingExceeded(0), got {:?}",
            other
        )),
    }
}

#[test]
fn validate_step_ceilings_rejects_step_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 2_000_000,
        max_transitions_per_tick: 500,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_step_ceilings_rejects_transitions_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 2_000_000,
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected PerTickCeilingExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateCapacity and AggregateReservation
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_capacity_is_copy() -> Result<(), String> {
    let cap = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    };
    let copy = cap;
    ensure_equal(cap.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_resource_usage_is_copy() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
    let copy = usage;
    ensure_equal(usage.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_reservation_debug_format() -> Result<(), String> {
    let reservation = crate::budget::AggregateReservation {
        run: crate::ids::RunId::new(42),
        requested: AggregateResourceBudget {
            max_steps_executable: 1000,
            max_action_tickets: 100,
            max_parallel_in_flight: 10,
            max_retries_per_action: 3,
            max_gather_pages: 5,
            max_gather_items: 100,
            max_for_each_iterations: 50,
            max_together_branches: 5,
            max_repeat_attempts: 3,
            max_run_time_seconds: 3600,
            max_result_bytes: 65536,
            max_total_slots_written: 1000,
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 50,
            max_journal_batch_bytes: 4096,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 1000,
            max_transitions_per_tick: 500,
            max_open_gates: 0,
            max_signal_subscriptions: 0,
            max_incident_emissions: 0,
            max_transitions: 0,
            max_journal_bytes_total: 0,
            max_value_store_entries: 0,
            max_cost_units: 0,
        },
    };
    let debug = format!("{:?}", reservation);
    ensure_equal(debug.contains("AggregateReservation"), true)
}

// -------------------------------------------------------------------------
// AggregateBudgetError Debug
// -------------------------------------------------------------------------

#[test]
fn aggregate_budget_error_workflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    });
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_policy_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PolicyExceeded {
        resource: "max_steps",
        actual: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_capacity_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::CapacityExceeded {
        resource: "max_steps",
        requested: 100,
        available: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_overflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Overflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_underflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Underflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_invalid_capacity_debug() -> Result<(), String> {
    let err = AggregateBudgetError::InvalidCapacity { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_reservation_not_found_debug() -> Result<(), String> {
    let err = AggregateBudgetError::ReservationNotFound {
        run: crate::ids::RunId::new(42),
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_step_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::StepCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_per_tick_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PerTickCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

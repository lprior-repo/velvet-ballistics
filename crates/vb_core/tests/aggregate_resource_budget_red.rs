use vb_core::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, RunId, validate_aggregate_budget,
};

#[test]
fn usage_adds_and_subtracts_budget_with_exact_observable_usage() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
        max_action_tickets: 20,
        ..AggregateResourceUsage::default()
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 2,
        ..zero_budget()
    };

    let added = usage.try_add_budget(&budget);

    assert_eq!(
        added,
        Ok(AggregateResourceUsage {
            max_steps_executable: 11,
            max_action_tickets: 22,
            max_active_runs: 1,
            ..AggregateResourceUsage::default()
        })
    );
    assert_eq!(
        AggregateResourceUsage {
            max_steps_executable: 11,
            max_action_tickets: 22,
            max_active_runs: 1,
            ..AggregateResourceUsage::default()
        }
        .try_subtract_budget(&budget),
        Ok(usage)
    );
}

#[test]
fn usage_subtract_returns_exact_underflow_resource_when_dimension_is_short() {
    let actual = AggregateResourceUsage::default().try_subtract_budget(&AggregateResourceBudget {
        max_action_tickets: 1,
        ..zero_budget()
    });

    assert_eq!(
        actual,
        Err(AggregateBudgetError::Underflow {
            resource: "max_action_tickets"
        })
    );
}

#[test]
fn usage_add_returns_exact_overflow_resource_when_dimension_overflows() {
    let actual = AggregateResourceUsage {
        max_trace_events: u64::MAX,
        ..AggregateResourceUsage::default()
    }
    .try_add_budget(&AggregateResourceBudget {
        max_trace_events: 1,
        ..zero_budget()
    });

    assert_eq!(
        actual,
        Err(AggregateBudgetError::Overflow {
            resource: "max_trace_events"
        })
    );
}

#[test]
fn fits_within_returns_exact_capacity_exceeded_resource_requested_and_available() {
    let actual = AggregateResourceUsage {
        max_queue_depth: 9,
        ..AggregateResourceUsage::default()
    }
    .fits_within(&AggregateResourceCapacity {
        max_queue_depth: 8,
        ..capacity(100)
    });

    assert_eq!(
        actual,
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "max_queue_depth",
            requested: 9,
            available: 8,
        })
    );
}

#[test]
fn usage_check_policy_returns_exact_policy_exceeded_resource_actual_and_limit() {
    let actual = AggregateResourceUsage {
        max_steps_executable: 6,
        ..AggregateResourceUsage::default()
    }
    .check_policy(&BoundednessPolicy {
        absolute_max_steps_executable: 5,
        ..BoundednessPolicy::DEFAULT
    });

    assert_eq!(
        actual,
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_steps_executable",
            actual: 6,
            limit: 5,
        })
    );
}

#[test]
fn validate_aggregate_budget_returns_exact_policy_exceeded_for_total_slots_written() {
    let actual = validate_aggregate_budget(
        &AggregateResourceBudget {
            max_total_slots_written: 4,
            ..zero_budget()
        },
        &BoundednessPolicy {
            max_total_slots: 3,
            ..BoundednessPolicy::DEFAULT
        },
    );

    assert_eq!(
        actual,
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_total_slots_written",
            actual: 4,
            limit: 3,
        })
    );
}

#[test]
fn reservation_preserves_run_id_and_requested_budget_exactly() {
    let requested = zero_budget();
    let reservation = AggregateReservation {
        run: RunId::new(42),
        requested,
    };

    assert_eq!(reservation.run, RunId::new(42));
    assert_eq!(reservation.requested, requested);
}

fn zero_budget() -> AggregateResourceBudget {
    AggregateResourceBudget {
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    }
}

fn capacity(value: u64) -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: value,
        max_action_tickets: value,
        max_parallel_in_flight: 100,
        max_gather_pages: value,
        max_gather_items: value,
        max_result_bytes: value,
        max_total_slots_written: value,
        max_timer_entries: value,
        max_trace_events: value,
        max_active_runs: value,
        max_queue_depth: value,
        max_journal_batch_bytes: value,
        max_ipc_payload_bytes: value,
        max_blob_bytes: value,
        max_input_bytes: value,
        max_step_budget_per_tick: value,
        max_transitions_per_tick: value,
    }
}

use vb_core::{AggregateBudgetError, RunId};

#[test]
fn aggregate_budget_error_display_preserves_capacity_values() {
    let error = AggregateBudgetError::CapacityExceeded {
        resource: "max_queue_depth",
        requested: 9,
        available: 8,
    };

    assert_eq!(
        format!("{error}"),
        "capacity exceeded: max_queue_depth requested 9, available 8"
    );
}

#[test]
fn aggregate_budget_error_display_preserves_policy_values() {
    let error = AggregateBudgetError::PolicyExceeded {
        resource: "max_steps_executable",
        actual: 6,
        limit: 5,
    };

    assert_eq!(
        format!("{error}"),
        "policy exceeded: max_steps_executable 6 > 5"
    );
}

#[test]
fn aggregate_budget_error_display_preserves_underflow_resource() {
    let error = AggregateBudgetError::Underflow {
        resource: "max_action_tickets",
    };

    assert_eq!(format!("{error}"), "underflow: max_action_tickets");
}

#[test]
fn aggregate_budget_error_debug_preserves_reservation_not_found_run_id() {
    let error = AggregateBudgetError::ReservationNotFound {
        run: RunId::new(77),
    };

    assert_eq!(
        format!("{error:?}"),
        "ReservationNotFound { run: RunId(77) }"
    );
}

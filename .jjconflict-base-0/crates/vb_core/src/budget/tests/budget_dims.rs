// =========================================================================
// §77.18 resource budget dimensions (vb/budget-dims)
// =========================================================================

fn ask_node(id: u16, next: Option<u16>) -> CompiledNode {
    test_node(
        id,
        next,
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None.into(),
        },
    )
}

fn wait_event_node(id: u16, next: Option<u16>) -> CompiledNode {
    test_node(
        id,
        next,
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(0),
            timeout_slot: None.into(),
        },
    )
}

fn do_node(id: u16, action: u16, next: Option<u16>) -> CompiledNode {
    test_node(
        id,
        next,
        CompiledNodeKind::Do {
            action: ActionId::new(action),
            input: SlotIdx::new(0),
        },
    )
}

fn nop_node(id: u16, next: Option<u16>) -> CompiledNode {
    test_node(id, next, CompiledNodeKind::Nop)
}

#[test]
fn budget_dimensions_count_ask_and_wait_event_nodes() {
    let nodes = vec![
        nop_node(0, Some(1)),
        ask_node(1, Some(2)),
        wait_event_node(2, Some(3)),
        finish_node(3),
    ];
    let contract = test_contract(4, 2);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("budget compute must succeed");
    assert_eq!(budget.max_open_gates, 1, "one Ask node is one open gate");
    assert_eq!(
        budget.max_signal_subscriptions, 1,
        "one WaitEvent node is one signal subscription"
    );
    assert_eq!(
        budget.max_timer_entries, 2,
        "Ask and WaitEvent still count as timer entries"
    );
}

#[test]
fn budget_dimensions_small_linear_counts_gates_and_signals() {
    let nodes = vec![ask_node(0, Some(1)), wait_event_node(1, None)];
    let contract = test_contract(2, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("small-linear budget compute must succeed");
    assert_eq!(budget.max_open_gates, 1);
    assert_eq!(budget.max_signal_subscriptions, 1);
}

#[test]
fn budget_dimensions_transitions_include_retry_factor() {
    let contract = ResourceContract {
        max_retry_attempts: 3,
        ..test_contract(1, 1)
    };
    let nodes = vec![finish_node(0)];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("budget compute must succeed");
    assert_eq!(
        budget.max_transitions, 4,
        "one step times the retry factor (1 + 3) is four transitions"
    );
}

#[test]
fn budget_dimensions_journal_total_is_batch_times_steps() {
    let contract = ResourceContract {
        max_journal_batch_bytes: 100,
        ..test_contract(3, 1)
    };
    let nodes = vec![nop_node(0, Some(1)), nop_node(1, Some(2)), finish_node(2)];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("budget compute must succeed");
    assert_eq!(
        budget.max_journal_bytes_total, 300,
        "three steps at a 100-byte batch cap is 300 cumulative bytes"
    );
}

#[test]
fn budget_dimensions_cost_units_equal_action_tickets() {
    let nodes = vec![
        do_node(0, 0, Some(1)),
        do_node(1, 1, Some(2)),
        finish_node(2),
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("budget compute must succeed");
    assert_eq!(budget.max_action_tickets, 2);
    assert_eq!(budget.max_cost_units, 2, "one cost unit per action ticket");
}

#[test]
fn budget_dimensions_incidents_and_value_entries_bounded_by_steps() {
    let contract = test_contract(3, 2);
    let nodes = vec![nop_node(0, Some(1)), nop_node(1, Some(2)), finish_node(2)];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .expect("budget compute must succeed");
    assert_eq!(
        budget.max_incident_emissions, 3,
        "one incident per step at most"
    );
    assert_eq!(
        budget.max_value_store_entries, 6,
        "three steps plus two input slots plus one constant"
    );
}

#[test]
fn budget_dimensions_policy_rejects_each_new_dimension_over_cap() {
    let over_gates = test_budget(1, 1, 0, 0);
    let over_gates = WholeWorkflowBudget {
        max_open_gates: 2,
        ..over_gates
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_open_gates: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_gates),
        Err(BudgetError::OpenGatesExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_signals = WholeWorkflowBudget {
        max_signal_subscriptions: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_signal_subscriptions: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_signals),
        Err(BudgetError::SignalSubscriptionsExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_incidents = WholeWorkflowBudget {
        max_incident_emissions: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_incident_emissions: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_incidents),
        Err(BudgetError::IncidentEmissionsExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_transitions = WholeWorkflowBudget {
        max_transitions: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_transitions: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_transitions),
        Err(BudgetError::TransitionsExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_journal = WholeWorkflowBudget {
        max_journal_bytes_total: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_journal_bytes_total: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_journal),
        Err(BudgetError::JournalBytesTotalExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_entries = WholeWorkflowBudget {
        max_value_store_entries: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_value_store_entries: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_entries),
        Err(BudgetError::ValueStoreEntriesExceeded {
            actual: 2,
            limit: 1
        })
    );

    let over_cost = WholeWorkflowBudget {
        max_cost_units: 2,
        ..test_budget(1, 1, 0, 0)
    };
    assert_eq!(
        BoundednessPolicy {
            absolute_max_cost_units: 1,
            ..BoundednessPolicy::DEFAULT
        }
        .validate(&over_cost),
        Err(BudgetError::CostUnitsExceeded {
            actual: 2,
            limit: 1
        })
    );
}

fn ample_capacity() -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    }
}

fn new_dimension_budget() -> AggregateResourceBudget {
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
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 1,
        max_transitions: 1,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

#[test]
fn budget_dimensions_aggregate_add_subtract_round_trip() {
    let budget = new_dimension_budget();
    let usage = AggregateResourceUsage::default()
        .try_add_budget(&budget)
        .expect("add must succeed");
    assert_eq!(usage.max_open_gates, 1);
    assert_eq!(usage.max_signal_subscriptions, 1);
    assert_eq!(usage.max_incident_emissions, 1);
    assert_eq!(usage.max_transitions, 1);
    assert_eq!(usage.max_journal_bytes_total, 1);
    assert_eq!(usage.max_value_store_entries, 1);
    assert_eq!(usage.max_cost_units, 1);
    assert_eq!(usage.max_active_runs, 1);
    let back = usage
        .try_subtract_budget(&budget)
        .expect("subtract must succeed");
    assert_eq!(back, AggregateResourceUsage::default());
}

#[test]
fn budget_dimensions_aggregate_fits_rejects_new_dimensions() {
    let budget = new_dimension_budget();
    let usage = AggregateResourceUsage::default()
        .try_add_budget(&budget)
        .expect("add must succeed");
    let mut capacity = ample_capacity();
    capacity.max_open_gates = 0;
    assert_eq!(
        usage.fits_within(&capacity),
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "max_open_gates",
            requested: 1,
            available: 0,
        })
    );
    let mut capacity = ample_capacity();
    capacity.global_quota_cost_units = 0;
    assert_eq!(
        usage.fits_within(&capacity),
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "global_quota_cost_units",
            requested: 1,
            available: 0,
        })
    );
    assert!(
        usage.fits_within(&ample_capacity()).is_ok(),
        "ample capacity fits all new dimensions"
    );
}

#[test]
fn budget_dimensions_aggregate_policy_rejects_new_dimensions() {
    let budget = new_dimension_budget();
    let mut policy = BoundednessPolicy::DEFAULT;
    policy.absolute_max_cost_units = 0;
    assert_eq!(
        validate_aggregate_budget(&budget, &policy),
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_cost_units",
            actual: 1,
            limit: 0,
        })
    );
    assert!(
        validate_aggregate_budget(&budget, &BoundednessPolicy::DEFAULT).is_ok(),
        "default policy admits the small new-dimension budget"
    );
}

#[test]
fn budget_dimensions_tenant_quota_checked_add_and_cap() {
    assert_eq!(
        check_tenant_quota(5, 6, 10),
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "tenant_quota_cost_units",
            requested: 11,
            available: 10,
        })
    );
    assert!(check_tenant_quota(5, 5, 10).is_ok(), "at-quota admits");
    assert_eq!(
        check_tenant_quota(u64::MAX, 1, u64::MAX),
        Err(AggregateBudgetError::Overflow {
            resource: "tenant_quota_cost_units",
        })
    );
}

#[test]
fn shard_usage_snapshot_records_and_sums_per_tenant() {
    let mut snapshot = ShardUsageSnapshot::default();
    assert_eq!(
        snapshot.tenant_cost_usage(TenantId::new(7)),
        Ok(0),
        "unrecorded tenant has zero usage"
    );
    snapshot
        .record_tenant_cost(TenantId::new(7), 5)
        .expect("first record fits");
    snapshot
        .record_tenant_cost(TenantId::new(7), 6)
        .expect("second record merges into the same entry");
    snapshot
        .record_tenant_cost(TenantId::new(8), 3)
        .expect("other tenant records independently");
    assert_eq!(snapshot.tenant_cost_usage(TenantId::new(7)), Ok(11));
    assert_eq!(snapshot.tenant_cost_usage(TenantId::new(8)), Ok(3));
    assert_eq!(snapshot.tenant_entries().len(), 2);
    assert_eq!(
        snapshot.tenant_cost_usage(TenantId::new(9)),
        Ok(0),
        "unrecorded tenant stays zero after other tenants record"
    );
    assert!(
        check_tenant_quota(
            snapshot
                .tenant_cost_usage(TenantId::new(7))
                .expect("summed usage"),
            0,
            11,
        )
        .is_ok()
    );
}

#[test]
fn shard_usage_snapshot_record_overflow_is_typed() {
    let mut snapshot = ShardUsageSnapshot::default();
    snapshot
        .record_tenant_cost(TenantId::new(7), u64::MAX)
        .expect("max value itself records");
    assert_eq!(
        snapshot.record_tenant_cost(TenantId::new(7), 1),
        Err(AggregateBudgetError::Overflow {
            resource: "tenant_quota_cost_units",
        })
    );
}

#[test]
fn budget_dimensions_global_quota_checked_add_and_cap() {
    assert_eq!(
        check_global_quota(5, 6, 10),
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "global_quota_cost_units",
            requested: 11,
            available: 10,
        })
    );
    assert!(check_global_quota(5, 5, 10).is_ok(), "at-quota admits");
    assert_eq!(
        check_global_quota(u64::MAX, 1, u64::MAX),
        Err(AggregateBudgetError::Overflow {
            resource: "global_quota_cost_units",
        })
    );
}

#[test]
fn budget_dimensions_remaining_gates_release_cycle() {
    let mut counters = RemainingBudget::from_budget(&AggregateResourceBudget {
        max_open_gates: 1,
        ..new_dimension_budget()
    });
    assert!(counters.try_open_gate().is_ok());
    let err = counters
        .try_open_gate()
        .expect_err("second gate must exhaust the single slot");
    assert_eq!(err, RemainingBudgetError::OpenGatesExhausted);
    assert_eq!(err.outcome(), BudgetExhaustionOutcome::Suspend);
    assert!(counters.release_gate().is_ok());
    assert_eq!(counters.remaining_open_gates(), 1);
    assert_eq!(
        counters.release_gate(),
        Err(RemainingBudgetError::ReleaseAboveLimit {
            resource: "max_open_gates",
        })
    );
}

#[test]
fn budget_dimensions_remaining_signals_release_cycle() {
    let mut counters = RemainingBudget::from_budget(&AggregateResourceBudget {
        max_signal_subscriptions: 1,
        ..new_dimension_budget()
    });
    assert!(counters.try_subscribe_signal().is_ok());
    let err = counters
        .try_subscribe_signal()
        .expect_err("second subscription must exhaust the single slot");
    assert_eq!(err, RemainingBudgetError::SignalSubscriptionsExhausted);
    assert_eq!(err.outcome(), BudgetExhaustionOutcome::Suspend);
    assert!(counters.release_signal().is_ok());
    assert_eq!(
        counters.release_signal(),
        Err(RemainingBudgetError::ReleaseAboveLimit {
            resource: "max_signal_subscriptions",
        })
    );
}

#[test]
fn budget_dimensions_remaining_monotonic_dimensions_fail_on_exhaustion() {
    let mut counters = RemainingBudget::from_budget(&AggregateResourceBudget {
        max_transitions: 2,
        max_journal_bytes_total: 10,
        max_value_store_entries: 3,
        max_incident_emissions: 1,
        max_cost_units: 5,
        ..new_dimension_budget()
    });
    assert!(counters.try_consume_transition().is_ok());
    assert!(counters.try_consume_transition().is_ok());
    let err = counters
        .try_consume_transition()
        .expect_err("third transition must exhaust");
    assert_eq!(err, RemainingBudgetError::TransitionsExhausted);
    assert_eq!(err.outcome(), BudgetExhaustionOutcome::Fail);

    assert!(counters.try_append_journal_bytes(6).is_ok());
    assert_eq!(counters.remaining_journal_bytes_total(), 4);
    assert_eq!(
        counters.try_append_journal_bytes(5),
        Err(RemainingBudgetError::JournalBytesExhausted {
            requested: 5,
            remaining: 4,
        })
    );
    assert_eq!(
        counters.remaining_journal_bytes_total(),
        4,
        "a failed append consumes nothing"
    );

    assert!(counters.try_create_value_store_entries(3).is_ok());
    assert_eq!(
        counters.try_create_value_store_entries(1),
        Err(RemainingBudgetError::ValueStoreEntriesExhausted {
            requested: 1,
            remaining: 0,
        })
    );

    assert!(counters.try_emit_incident().is_ok());
    assert_eq!(
        counters.try_emit_incident(),
        Err(RemainingBudgetError::IncidentEmissionsExhausted)
    );

    assert!(counters.try_spend_cost_units(5).is_ok());
    assert_eq!(
        counters.try_spend_cost_units(1),
        Err(RemainingBudgetError::CostUnitsExhausted {
            requested: 1,
            remaining: 0,
        })
    );
}

#[test]
fn budget_cost_units_cover_retried_action_tickets() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        ),
    ];
    let contract = ResourceContract {
        max_retry_attempts: 3,
        ..test_contract(2, 1)
    };
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_action_tickets == 1 && b.max_cost_units == 4);
    assert!(
        budget.is_some(),
        "one action ticket retried up to 3 extra attempts must budget 4 cost units"
    );
}

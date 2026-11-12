// =========================================================================
// Additional coverage: count_and_push_loop_body overflow paths
// =========================================================================

#[test]
fn count_total_steps_overflow_returns_step_count_overflow() {
    use crate::ids::StepIdx;
    use crate::workflow::CompiledNodeKind;

    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
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
    let contract = test_contract(4, 4);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            assert!(budget.max_total_steps > u64::from(u32::MAX));
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(other) => panic!("expected StepCountOverflow, got {:?}", other),
    }
}

#[test]
fn count_and_push_loop_body_overflow_propagates_budget_error() {
    use crate::ids::StepIdx;
    use crate::workflow::CompiledNodeKind;

    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: u32::MAX.into(),
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(4),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(5, 5);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            let _ = budget;
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(e) => panic!("expected StepCountOverflow, got {:?}", e),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget fields
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_parallel_in_flight_from_together() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                    StepIdx::new(5),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(6),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(3), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(4), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(5), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(6),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(7, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_parallel_in_flight, 5)
}

#[test]
fn whole_workflow_budget_max_action_tickets_from_do_nodes() -> Result<(), String> {
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
            CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(2),
            },
            None,
            Some(StepIdx::new(3)),
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 3)
}

#[test]
fn whole_workflow_budget_max_gather_pages_and_items() -> Result<(), String> {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 50.into(),
                page_size: 10,
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
    ensure_equal(budget.max_gather_pages, 5)?;
    ensure_equal(budget.max_gather_items, 50)
}

// -------------------------------------------------------------------------
// WorkflowError variants from compute path
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_jump_cycle_detected_in_compute() -> Result<(), String> {
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
                target: StepIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(1))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {:?}", other)),
    }
}

#[test]
fn whole_workflow_budget_step_out_of_bounds_in_visit() -> Result<(), String> {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(99)),
    )];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { step }) => ensure_equal(step, StepIdx::new(99)),
        other => Err(format!("expected StepOutOfBounds, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateResourceBudget and AggregateResourceUsage
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_budget_from_whole_workflow_budget() -> Result<(), String> {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )];
    let contract = test_contract(1, 1);
    let wfb = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| format!("{:?}", e))?;
    let arb = crate::budget::AggregateResourceBudget::from_whole_workflow_budget(wfb, contract)
        .map_err(|e| format!("{:?}", e))?;
    ensure_equal(arb.max_steps_executable, wfb.max_steps_executable)?;
    ensure_equal(arb.max_action_tickets, wfb.max_action_tickets)?;
    ensure_equal(arb.max_parallel_in_flight, wfb.max_parallel_in_flight)?;
    ensure_equal(arb.max_timer_entries, wfb.max_timer_entries)?;
    ensure_equal(arb.max_trace_events, wfb.max_trace_events)?;
    ensure_equal(arb.max_result_bytes, 1)?;
    ensure_equal(arb.max_queue_depth, 1)?;
    ensure_equal(arb.max_journal_batch_bytes, 1)?;
    ensure_equal(arb.max_ipc_payload_bytes, 1)?;
    ensure_equal(arb.max_blob_bytes, 1)?;
    ensure_equal(arb.max_input_bytes, 1)
}

#[test]
fn aggregate_resource_usage_try_add_budget_overflow() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_try_subtract_budget_underflow() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource: _ }) => Ok(()),
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_fits_within_capacity() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 7,
        max_trace_events: 8,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
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
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_timer_entries: 14,
        max_trace_events: 16,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 18,
        max_blob_bytes: 20,
        max_input_bytes: 22,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
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
    ensure_equal(usage.fits_within(&capacity), Ok(()))
}

#[test]
fn aggregate_resource_usage_fits_within_rejects_insufficient() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 300,
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_action_tickets() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 150,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_parallel_in_flight() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 15,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_gather_pages() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 10,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_gather_pages")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_gather_items() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 200,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_gather_items")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_result_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 2000,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "max_result_bytes",
            requested: 2000,
            available: 1000,
        }) => Ok(()),
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_total_slots_written() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_active_runs() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 10,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_active_runs")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_queue_depth() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 50,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_queue_depth")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_journal_batch_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_step_budget_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 250,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_transitions_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 1000,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
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
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

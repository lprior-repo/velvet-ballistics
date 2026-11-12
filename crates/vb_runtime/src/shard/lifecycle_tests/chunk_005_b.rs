#[test]
fn drive_state_at_timer_entry_limit_suspends_on_wait_until() {
    let Some(workflow) = timer_wait_workflow() else {
        return;
    };
    let run = RunId::new(721);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        timer_entry_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_fails_when_ask_timer_exceeds_admitted_limit() {
    let Some(workflow) = timer_ask_workflow() else {
        return;
    };
    let run = RunId::new(722);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        timer_entry_budget(0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "timer_entries",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_without_admitted_budget_ignores_timer_counts() {
    let Some(workflow) = timer_wait_workflow() else {
        return;
    };
    let run = RunId::new(723);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}
fn incident_budget(max_incident_emissions: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn incident_state(
    run_id: u64,
    max_incident_emissions: u64,
    executed: u64,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    for _ in 0..executed {
        let _ = state.frame.increment_executed();
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        incident_budget(max_incident_emissions),
    ));
    Some(state)
}

#[test]
fn drive_state_within_incident_limit_issues_action() {
    let Some(mut state) = incident_state(750, 10, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_when_executed_steps_exceed_incident_limit() {
    let Some(mut state) = incident_state(751, 2, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "incident_emissions",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_fails_on_exceeded_incidents_without_new_steps() {
    let Some(mut state) = incident_state(752, 2, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "incident_emissions",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_ignores_incident_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(753);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    for _ in 0..50 {
        let _ = state.frame.increment_executed();
    }
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn retry_budget(max_retries_per_action: u16) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn retry_state(
    run_id: u64,
    max_retries_per_action: u16,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        retry_budget(max_retries_per_action),
    ));
    Some(state)
}

#[test]
fn drive_state_first_attempt_within_zero_retry_limit_issues_action() {
    let Some(mut state) = retry_state(740, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_at_retry_limit_issues_action() {
    let Some(mut state) = retry_state(741, 1, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_when_recorded_retries_exceed_admitted_limit() {
    let Some(mut state) = retry_state(742, 1, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "retries_per_action",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_fails_on_recorded_retries_without_new_issue() {
    let Some(mut state) = retry_state(743, 0, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "retries_per_action",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_ignores_retry_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(744);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn ticket_budget(max_action_tickets: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units: 1000,
    }
}

fn ticket_state(
    run_id: u64,
    max_action_tickets: u32,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        ticket_budget(max_action_tickets),
    ));
    Some(state)
}

#[test]
fn drive_state_fails_when_action_tickets_exceed_admitted_limit() {
    let Some(mut state) = ticket_state(730, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "action_tickets",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_action_ticket_limit_issues_action() {
    let Some(mut state) = ticket_state(731, 1, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_retry_of_same_action_does_not_count_again() {
    let Some(mut state) = ticket_state(732, 1, 3) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}

#[test]
fn drive_state_fails_on_recorded_tickets_without_new_issue() {
    let Some(mut state) = ticket_state(733, 0, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 0, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "action_tickets",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_ignores_ticket_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(734);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn cost_budget(max_cost_units: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 1000,
        max_result_bytes: 1024,
        max_total_slots_written: 10,
        max_timer_entries: 10,
        max_trace_events: 1000,
        max_queue_depth: 10,
        max_journal_batch_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 1024,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 1000,
        max_journal_bytes_total: 100_000,
        max_value_store_entries: 1000,
        max_cost_units,
    }
}

fn cost_state(
    run_id: u64,
    max_cost_units: u64,
    recorded_attempt: u16,
) -> Option<crate::shard::types::RunState> {
    let Some(workflow) = suspended_workflow() else {
        return None;
    };
    let run = RunId::new(run_id);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)?;
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = recorded_attempt;
    }
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
        cost_budget(max_cost_units),
    ));
    Some(state)
}

#[test]
fn drive_state_fails_when_cost_units_exceed_admitted_limit() {
    let Some(mut state) = cost_state(720, 0, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "cost_units",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_cost_unit_limit_issues_action() {
    let Some(mut state) = cost_state(721, 1, 0) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket))
            if ticket.step == StepIdx::ZERO && ticket.attempt == 1
    ));
}

#[test]
fn drive_state_fails_when_recorded_tickets_exceed_limit() {
    let Some(mut state) = cost_state(722, 1, 2) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "cost_units",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_cost_unit_limit_with_recorded_ticket_issues_action() {
    let Some(mut state) = cost_state(723, 1, 1) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket))
            if ticket.step == StepIdx::ZERO && ticket.attempt == 1
    ));
}

#[test]
fn drive_state_ignores_cost_check_without_budget() {
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(724);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let _ = state.frame.write_slot(SlotIdx::ZERO, SlotValue::I64(0));
    state.action_contracts =
        crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0));
    state.admission = Some(RunAdmission::new(
        digest,
        run,
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        vb_core::policy::RuntimePolicy::Relaxed,
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ticket)) if ticket.step == StepIdx::ZERO
    ));
}
fn suspension_budget(max_open_gates: u32, max_signal_subscriptions: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 100,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates,
        max_signal_subscriptions,
        max_incident_emissions: 100,
        max_transitions: 10,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

fn wait_event_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_event = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let set_timeout = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst { value: ConstIdx::new(1) },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let wait = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::WaitEvent { event: SlotIdx::ZERO, timeout_slot: Some(SlotIdx::new(1)).into() },
        None,
        Some(StepIdx::new(3)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish { result: SlotIdx::ZERO },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("wait_event_budget"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([set_event, set_timeout, wait, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(7), ConstValue::I64(10)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_open_gates_exceed_admitted_limit() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(710);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(0, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_open_gate_limit_suspends_on_ask() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(711);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}

#[test]
fn drive_state_fails_when_manual_gate_pushes_open_gates_over_limit() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(725);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 1, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_combined_open_gate_limit_suspends_on_ask() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(726);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(2, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 1, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}

#[test]
fn drive_state_fails_when_manual_gates_alone_exceed_limit() {
    let Some(workflow) = tiny_journal_finished_workflow() else {
        return;
    };
    let run = RunId::new(727);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(0, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 1, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "open_gates",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_fails_when_signal_subscriptions_exceed_admitted_limit() {
    let Some(workflow) = wait_event_workflow() else {
        return;
    };
    let run = RunId::new(712);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "signal_subscriptions",
            limit: 0,
        }))
    ));
}

#[test]
fn drive_state_at_signal_subscription_limit_suspends_on_wait_event() {
    let Some(workflow) = wait_event_workflow() else {
        return;
    };
    let run = RunId::new(713);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_timer_wait_is_not_a_signal_subscription() {
    let Some(workflow) = wait_workflow() else {
        return;
    };
    let run = RunId::new(714);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        suspension_budget(1, 0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingWait)));
}

#[test]
fn drive_state_without_admitted_budget_ignores_suspension_counts() {
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(715);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::AwaitingAsk)));
}
fn value_store_budget(max_value_store_entries: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 100,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 1,
        max_signal_subscriptions: 1,
        max_incident_emissions: 100,
        max_transitions: 10,
        max_journal_bytes_total: 1,
        max_value_store_entries,
        max_cost_units: 1,
    }
}

fn value_store_budget_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    nodes.push(vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::BuildList {
            items: Box::from([SlotIdx::ZERO]),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("value_store_budget"),
        digest: WorkflowDigest::from_bytes([8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_value_store_entries_exceed_admitted_limit() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(702);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        value_store_budget(0),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "value_store_entries",
            limit: 0,
        }))
    ));
    assert_eq!(state.store.total_arena_count(), 1);
}

#[test]
fn drive_state_at_value_store_limit_drives_to_finish() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(703);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        value_store_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.store.total_arena_count(), 1);
}

#[test]
fn drive_state_without_admitted_budget_ignores_value_store_count() {
    let Some(workflow) = value_store_budget_workflow() else {
        return;
    };
    let run = RunId::new(704);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.store.total_arena_count(), 1);
}
fn gather_items_budget(max_gather_items: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes: 100,
        max_total_slots_written: 100,
        max_timer_entries: 100,
        max_trace_events: 100,
        max_queue_depth: 100,
        max_journal_batch_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_blob_bytes: 100,
        max_input_bytes: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        max_open_gates: 10,
        max_signal_subscriptions: 10,
        max_incident_emissions: 100,
        max_transitions: 100,
        max_journal_bytes_total: 100,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn seed_collect_items_consumed(
    state: &mut crate::shard::types::RunState,
    run: vb_core::ids::RunId,
    cursor: usize,
) {
    state
        .collect_states
        .upsert(crate::primitives::collect::CollectPaginationState {
            run_id: run,
            collector_slot: SlotIdx::ZERO,
            source: vb_core::ids::ListId::new(1),
            current_page: vb_core::ids::ListId::new(1),
            cursor,
            page_size: cursor,
            item_count: cursor,
            limit: 100,
            time_limit_ms: None,
            start_millis: 0,
        })
        .expect("seed collect items");
}


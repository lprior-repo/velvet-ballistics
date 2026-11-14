#[test]
fn drive_state_fails_when_gather_items_exceed_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(750);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    assert_eq!(state.collect_states.total_items_consumed(), 3);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_items_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_items",
            limit: 2,
        }))
    ));
}

#[test]
fn drive_state_at_gather_items_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(751);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_items_budget(3),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_gather_items() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(752);
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    seed_collect_items_consumed(&mut state, run, 3);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn transition_budget(max_transitions: u64) -> AggregateResourceBudget {
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
        max_total_slots_written: 1,
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
        max_transitions,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: 1,
    }
}

fn transition_budget_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    nodes.push(vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    ));
    let mut index = 1;
    while index < 10 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(index),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(index + 1)),
        ));
        index += 1;
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(10),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("transition_budget"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_caps_tick_at_admitted_total_transitions_and_fails_on_exhaustion() {
    let Some(workflow) = transition_budget_workflow() else {
        return;
    };
    let run = RunId::new(700);
    let digest = workflow.digest();
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        transition_budget(3),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::StepBudgetExhausted)));
    assert_eq!(state.frame.executed(), 3);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "transitions",
            limit: 3,
        }))
    ));
    assert_eq!(state.frame.executed(), 3);
}

#[test]
fn drive_state_without_admitted_budget_drives_to_finish() {
    let Some(workflow) = transition_budget_workflow() else {
        return;
    };
    let run = RunId::new(701);
    let Some(mut state) = crate::shard::helpers::make_run_state(workflow, run) else {
        return;
    };
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
    assert_eq!(state.frame.executed(), 11);
}

#[test]
fn queued_retry_flushes_emitted_prefix_and_resumes_drive() -> Result<(), String> {
    let (journal, _queue, _adapter, shared) =
        queued_journal_fixture("vb-runtime-retry-watermark-", 16, 4)?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(710);
    submit_run(
        &mut shard,
        run,
        retry_workflow()?,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    // Submission events sit in the queue: nothing durable yet.
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // The retry flushed the emitted prefix (durable) and resumed the drive.
    assert!(
        !journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let state = shard
        .run_state_get(run)
        .ok_or("run should exist after queued retry")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(2));
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}

/// Journal stub that always reports a pending high-water, even after a
/// flush. Exercises the fail-closed arm of the retry watermark gate.
struct NeverDurableJournal;

impl crate::journal::RuntimeJournal for NeverDurableJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

    fn append(&self, _event: RuntimeJournalEvent) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn probe(&self) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn append_sequenced(
        &self,
        _event: RuntimeJournalEvent,
        _seq: vb_storage::EventSeq,
    ) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        _events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> Result<(), crate::RuntimeError> {
        Ok(())
    }

    fn pending_high_water_for_run(
        &self,
        _run: RunId,
    ) -> Result<Option<vb_storage::EventSeq>, crate::RuntimeError> {
        Ok(Some(vb_storage::EventSeq::new(0)))
    }
}

#[test]
fn blocked_retry_reports_typed_error_and_keeps_run() -> Result<(), String> {
    let shared: SharedRuntimeJournal = std::sync::Arc::new(NeverDurableJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(711);
    submit_run(
        &mut shard,
        run,
        retry_workflow()?,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: pending_ticket(&shard, run, StepIdx::new(1), 1),
            failure: retryable_failure(),
        }),
        Ok(())
    );
    match shard.tick() {
        Err(crate::RuntimeError::RetryBlockedOnUndrainedJournal { run: blocked }) => {
            assert_eq!(blocked, run);
        }
        other => {
            return Err(format!(
                "expected RetryBlockedOnUndrainedJournal, got {other:?}"
            ));
        }
    }
    // Nothing mutated: run, pending action, and attempt counter are intact.
    let state = shard
        .run_state_get(run)
        .ok_or("blocked retry must keep the run")?;
    assert_eq!(state.action_attempts.get(1).copied(), Some(1));
    pending_ticket(&shard, run, StepIdx::new(1), 1);
    Ok(())
}

fn result_bytes_budget(max_result_bytes: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds: 100,
        max_result_bytes,
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

fn list_result_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let first = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let second = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let build = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::BuildList {
            items: Box::from([SlotIdx::ZERO, SlotIdx::new(1)]),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(3)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("list_result_budget"),
        digest: WorkflowDigest::from_bytes([13; 32]),
        nodes: Box::from([first, second, build, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(20)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn drive_state_fails_when_finished_list_deep_size_exceeds_admitted_limit() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(750);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        result_bytes_budget(15),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "result_bytes",
            limit: 15,
        }))
    ));
}

#[test]
fn drive_state_at_finished_list_deep_size_limit_finishes() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(751);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        result_bytes_budget(16),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_finished_list_size() {
    let workflow = list_result_workflow().expect("list result workflow builds");
    let run = RunId::new(752);
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

fn gather_pages_budget(max_gather_pages: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages,
        max_gather_items: 10,
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

fn deliver_collect_page(
    states: &mut crate::primitives::collect::CollectStates,
    run: RunId,
    page: u32,
) {
    states
        .upsert(crate::primitives::collect::CollectPaginationState {
            run_id: run,
            collector_slot: SlotIdx::new(1),
            source: vb_core::ids::ListId::new(1),
            current_page: vb_core::ids::ListId::new(page),
            cursor: 1,
            page_size: 1,
            item_count: 1,
            limit: 1,
            time_limit_ms: None,
            start_millis: 0,
        })
        .expect("page upsert must succeed");
}

#[test]
fn drive_state_fails_when_delivered_pages_exceed_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(764);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_pages",
            limit: 1,
        }))
    ));
}

#[test]
fn drive_state_at_delivered_pages_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(765);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(2),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_admitted_budget_ignores_delivered_pages() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(766);
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_fails_when_delivered_pages_exceed_limit_after_removal() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(767);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    deliver_collect_page(&mut state.collect_states, run, 1);
    deliver_collect_page(&mut state.collect_states, run, 2);
    state.collect_states.remove(run, SlotIdx::new(1));
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        gather_pages_budget(1),
    ));
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "gather_pages",
            limit: 1,
        }))
    ));
}

fn run_time_budget(max_run_time_seconds: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_for_each_iterations: 10,
        max_together_branches: 10,
        max_repeat_attempts: 10,
        max_run_time_seconds,
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

#[test]
fn drive_state_fails_when_run_time_exceeds_admitted_limit() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(744);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    state.run_started_millis = Some(0);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "run_time",
            limit: 10,
        }))
    ));
}

#[test]
fn drive_state_within_run_time_limit_finishes() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(745);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(1000),
    ));
    state.run_started_millis = crate::shard::helpers::wall_clock_millis_since_epoch();
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_without_recorded_start_fails_closed() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(746);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(1000),
    ));
    state.run_started_millis = None;
    assert_eq!(state.run_started_millis, None);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(
            EngineError::InternalInvariantViolation { .. }
        ))
    ));
    assert_eq!(state.run_started_millis, None);
}

#[test]
fn drive_state_without_admitted_budget_ignores_run_time() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(747);
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.run_started_millis = Some(0);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    assert!(matches!(result, Ok(RuntimeSignal::Finished(_))));
}

#[test]
fn drive_state_run_time_overflow_fails_closed() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(760);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(u64::MAX),
    ));
    state.run_started_millis = Some(0);
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(1);
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "run_time",
            limit: u64::MAX,
        }))
    ));
}

#[test]
fn drive_state_run_time_exact_limit_is_live_and_plus_one_fails() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let digest = workflow.digest();

    let run = RunId::new(761);
    let mut exact = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    exact.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    exact.run_started_millis = Some(10_000);
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(20_000);
    let mut evidence = EvidenceCollector::new();
    let exact_result = Shard::drive_state_with_manual_gates(&mut exact, 100, 0, &mut evidence);
    assert!(matches!(exact_result, Ok(RuntimeSignal::Finished(_))));

    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(762);
    let mut over = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    over.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    over.run_started_millis = Some(10_000);
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(20_001);
    let mut evidence = EvidenceCollector::new();
    let over_result = Shard::drive_state_with_manual_gates(&mut over, 100, 0, &mut evidence);
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert!(matches!(
        over_result,
        Err(RuntimeEngineError::Core(EngineError::BudgetExceeded {
            budget: "run_time",
            limit: 10,
        }))
    ));
}

#[test]
fn drive_state_run_time_repeated_checks_keep_original_origin() {
    let workflow = two_slot_workflow().expect("two slot workflow builds");
    let run = RunId::new(763);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run).expect("run state builds");
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    state.run_started_millis = Some(10_000);
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(20_000);
    assert_eq!(Shard::check_run_time_budget(&mut state), Ok(()));
    assert_eq!(state.run_started_millis, Some(10_000));
    assert_eq!(Shard::check_run_time_budget(&mut state), Ok(()));
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert_eq!(
        state.run_started_millis,
        Some(10_000),
        "repeated checks must not restart the admitted origin"
    );
}

#[test]
fn suspended_run_over_lifetime_fails_on_tick() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let workflow = suspended_workflow().ok_or_else(|| "suspended workflow builds".to_string())?;
    let run = RunId::new(764);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    state.run_started_millis = Some(10_000);
    state
        .frame
        .mark_running(StepIdx::ZERO)
        .map_err(|error| format!("mark running failed: {error:?}"))?;
    state
        .frame
        .mark_waiting(StepIdx::ZERO)
        .map_err(|error| format!("mark waiting failed: {error:?}"))?;
    shard
        .admit_run_state(run, state, super::super::RuntimeState::Resumable)
        .map_err(|error| error.to_string())?;

    crate::shard::helpers::timer::install_wall_clock_millis_for_test(20_001);
    let ticked = shard.tick().map_err(|error| error.to_string())?;
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert!(ticked);
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.run_state_contains(run));
    assert_eq!(shard.last_lifetime_expiry(), Some((run, 10)));
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    assert!(
        events.iter().any(
            |event| matches!(event, RuntimeJournalEvent::RunFailed { run: failed, .. } if *failed == run)
        ),
        "lifetime expiry must journal RunFailed"
    );
    Ok(())
}

#[test]
fn multiple_expired_suspended_runs_progress_one_per_tick() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal);
    let workflow = suspended_workflow().ok_or_else(|| "suspended workflow builds".to_string())?;
    let digest = workflow.digest();
    let first = RunId::new(765);
    let second = RunId::new(766);
    for run in [first, second] {
        let mut state = crate::shard::helpers::make_run_state(workflow.clone(), run)
            .ok_or_else(|| "run state builds".to_string())?;
        state.admission = Some(RunAdmission::with_budget(
            digest,
            run,
            CapabilitySet::empty(),
            vb_core::policy::RuntimePolicy::Relaxed,
            run_time_budget(10),
        ));
        state.run_started_millis = Some(10_000);
        state
            .frame
            .mark_running(StepIdx::ZERO)
            .map_err(|error| format!("mark running failed: {error:?}"))?;
        state
            .frame
            .mark_waiting(StepIdx::ZERO)
            .map_err(|error| format!("mark waiting failed: {error:?}"))?;
        shard
            .admit_run_state(run, state, super::super::RuntimeState::Resumable)
            .map_err(|error| error.to_string())?;
    }
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(20_001);
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(first));
    assert!(shard.run_state_contains(second));
    assert_eq!(shard.last_lifetime_expiry(), Some((first, 10)));
    assert_eq!(shard.tick(), Ok(true));
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert!(shard.terminal_runs_contains(second));
    assert_eq!(shard.last_lifetime_expiry(), Some((second, 10)));
    Ok(())
}

fn tiny_journal_contract() -> ResourceContract {
    ResourceContract {
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

fn tiny_journal_finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let finish = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("tiny_journal_finished"),
        digest: WorkflowDigest::from_bytes([21; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn tiny_journal_chain_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let mut nodes = Vec::new();
    for step in 0..6u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(step + 1)),
        ));
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(6),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("tiny_journal_chain"),
        digest: WorkflowDigest::from_bytes([22; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn tiny_journal_ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    );
    let set_timeout = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    let ask = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)).into(),
        },
        Some(vb_core::ids::SlotIdx::new(2)),
        Some(StepIdx::new(3)),
    );
    let resume = vb_core::workflow::node(
        StepIdx::new(3),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
        None,
        Some(StepIdx::new(4)),
    );
    let finish_node = vb_core::workflow::node(
        StepIdx::new(4),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("tiny_journal_ask"),
        digest: WorkflowDigest::from_bytes([23; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 2,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: tiny_journal_contract(),
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn journal_bytes_budget(total: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_retries_per_action: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
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
        max_journal_bytes_total: total,
        max_value_store_entries: 100,
        max_cost_units: 100,
    }
}

fn submit_with_journal(
    shard: &mut Shard,
    run: RunId,
    workflow: vb_core::workflow::CompiledWorkflow,
) {
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

fn run_failed_count(events: &[RuntimeJournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RuntimeJournalEvent::RunFailed { run: r, .. } if *r == run
            )
        })
        .count()
}

#[test]
fn charge_drive_journal_bytes_enforces_cumulative_total() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(770);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    let shard = Shard::new(small_config());
    let event = RuntimeJournalEvent::RunFailed { run, attempt: 1 };
    let bytes =
        postcard::to_allocvec(&event).map_err(|error| format!("event encode failed: {error:?}"))?;
    let len = u64::try_from(bytes.len()).map_err(|_| "event length overflow".to_string())?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        journal_bytes_budget(
            len.checked_mul(2)
                .ok_or_else(|| "budget overflow".to_string())?,
        ),
    ));
    assert!(
        shard
            .charge_drive_journal_bytes(&mut state, &[event.clone()])
            .map_err(|error| format!("first charge failed: {error:?}"))?
    );
    assert_eq!(state.journal_bytes_written, len);
    assert!(
        shard
            .charge_drive_journal_bytes(&mut state, &[event.clone()])
            .map_err(|error| format!("second charge failed: {error:?}"))?
    );
    assert_eq!(
        state.journal_bytes_written,
        len.checked_mul(2)
            .ok_or_else(|| "counter overflow".to_string())?
    );
    assert!(
        !shard
            .charge_drive_journal_bytes(&mut state, &[event])
            .map_err(|error| format!("third charge failed: {error:?}"))?
    );
    assert_eq!(
        state.journal_bytes_written,
        len.checked_mul(2)
            .ok_or_else(|| "counter overflow".to_string())?
    );
    Ok(())
}

#[test]
fn charge_drive_journal_bytes_without_budget_charges_without_enforcing() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| "two slot workflow builds".to_string())?;
    let run = RunId::new(771);
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| "run state builds".to_string())?;
    let shard = Shard::new(small_config());
    let event = RuntimeJournalEvent::RunFailed { run, attempt: 1 };
    let bytes =
        postcard::to_allocvec(&event).map_err(|error| format!("event encode failed: {error:?}"))?;
    let len = u64::try_from(bytes.len()).map_err(|_| "event length overflow".to_string())?;
    assert!(
        shard
            .charge_drive_journal_bytes(&mut state, &[event])
            .map_err(|error| format!("charge failed: {error:?}"))?
    );
    assert_eq!(state.journal_bytes_written, len);
    Ok(())
}

#[test]
fn drive_finished_run_falls_back_to_single_run_failed_when_journal_budget_exceeded()
-> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_finished", tiny_journal_finished_workflow())?;
    let run = RunId::new(772);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 0);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunFinished { run: r, .. } if *r == run
        )),
        "finished event must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn drive_continue_run_falls_back_to_single_run_failed_when_journal_budget_exceeded()
-> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_chain", tiny_journal_chain_workflow())?;
    let run = RunId::new(773);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::StepSucceeded { run: r, .. } if *r == run
        )),
        "drive evidence must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn drive_awaiting_run_falls_back_to_single_run_failed_when_journal_budget_exceeded()
-> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("tiny_journal_ask", tiny_journal_ask_workflow())?;
    let run = RunId::new(774);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert!(shard.terminal_runs_contains(run));
    let events = require_snapshot(&journal)?;
    assert_eq!(run_failed_count(&events, run), 1);
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::AskScheduled { run: r, .. } if *r == run
        )),
        "ask schedule must not be journaled after fallback: {events:?}"
    );
    Ok(())
}

#[test]
fn journal_bytes_counter_matches_appended_header_and_drive_events() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask_then_finish", ask_workflow())?;
    let run = RunId::new(775);
    submit_with_journal(&mut shard, run, wf);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?;
    let events = require_snapshot(&journal)?;
    let mut measured: u64 = 0;
    for event in &events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    assert!(measured > 0, "journal must contain events");
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

#[test]
fn storage_journal_byte_counter_matches_durable_storage_events() -> Result<(), String> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix("vb-runtime-storage-bytes-")
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let fjall = std::sync::Arc::new(
        vb_storage::FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?,
    );
    let shared: SharedRuntimeJournal =
        crate::journal::StorageRuntimeJournal::shared_strict(fjall.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let workflow = suspended_workflow().ok_or_else(|| "suspended workflow".to_string())?;
    let run = RunId::new(779);
    submit_run(
        &mut shard,
        run,
        workflow,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
    );
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should be suspended".to_string())?;
    let events = fjall
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    assert!(events.len() >= 2, "header must produce multiple events");
    let measured = crate::recovery::recover_journal_bytes_written(&events)
        .map_err(|error| error.to_string())?;
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

#[test]
fn pre_persisted_submit_seeds_counter_with_admission_event_only() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask_then_finish", ask_workflow())?;
    let run = RunId::new(776);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitPrePersisted {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let state = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?;
    let events = require_snapshot(&journal)?;
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunSubmitted { run: r, .. } if *r == run
        )),
        "pre-persisted submit must not journal a second submitted event: {events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            RuntimeJournalEvent::RunAdmission { admission } if admission.run_id() == run
        )),
        "pre-persisted submit must journal the admission event: {events:?}"
    );
    let mut measured: u64 = 0;
    for event in &events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    assert!(measured > 0, "journal must contain events");
    assert_eq!(state.journal_bytes_written, measured);
    Ok(())
}

fn measured_journal_bytes(events: &[RuntimeJournalEvent]) -> Result<u64, String> {
    let mut measured: u64 = 0;
    for event in events {
        let encoded = postcard::to_allocvec(event)
            .map_err(|error| format!("event encode failed: {error:?}"))?;
        measured = measured
            .checked_add(u64::try_from(encoded.len()).map_err(|_| "length overflow")?)
            .ok_or_else(|| "measured total overflow".to_string())?;
    }
    Ok(measured)
}

#[test]
fn empty_evidence_await_timer_charges_ask_scheduled_bytes() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("ask", ask_workflow())?;
    let run = RunId::new(777);
    submit_run(&mut shard, run, wf, &[]);
    assert_eq!(shard.tick(), Ok(true));
    let before_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active awaiting ask".to_string())?
        .journal_bytes_written;
    let before_events = require_snapshot(&journal)?;
    let before_measured = measured_journal_bytes(&before_events)?;
    let before_asks = before_events
        .iter()
        .filter(|event| {
            matches!(event, RuntimeJournalEvent::AskScheduled { run: ask_run, .. } if *ask_run == run)
        })
        .count();
    let state = shard
        .take_run_state(run)
        .map_err(|error| format!("suspended ask state must be checked out: {error:?}"))?;
    let _previous_timer = shard.pending_timer_remove(run);
    assert!(matches!(
        shard
            .await_timer(run, state, PendingTimerKind::Ask)
            .map_err(|error| format!("ask reschedule failed: {error:?}"))?,
        crate::shard::transitions::TransitionOutcome::Applied
    ));
    let after_counter = shard
        .run_state_get(run)
        .ok_or_else(|| "run should still be active after reschedule".to_string())?
        .journal_bytes_written;
    let after_events = require_snapshot(&journal)?;
    let after_measured = measured_journal_bytes(&after_events)?;
    let after_asks = after_events
        .iter()
        .filter(|event| {
            matches!(event, RuntimeJournalEvent::AskScheduled { run: ask_run, .. } if *ask_run == run)
        })
        .count();
    assert_eq!(after_asks, before_asks + 1);
    let charged = after_counter
        .checked_sub(before_counter)
        .ok_or_else(|| "counter must not decrease across the reschedule".to_string())?;
    let appended = after_measured
        .checked_sub(before_measured)
        .ok_or_else(|| "journal must not shrink across the reschedule".to_string())?;
    assert_eq!(charged, appended);
    assert!(charged > 0, "rescheduled ask must charge its journal event");
    Ok(())
}

#[test]
fn tenant_quota_charges_on_submit_and_releases_on_cancel() -> Result<(), String> {
    let workflow = suspended_workflow().ok_or_else(|| "suspended workflow".to_string())?;
    let tenant = vb_core::ids::TenantId::new(7);
    let run = RunId::new(8_001);
    let mut shard = Shard::new(small_config());
    shard.set_resource_quotas(u64::MAX, u64::MAX);
    shard
        .handle_submit_with_inputs_contracts_and_tenant(
            run,
            workflow,
            &[(SlotIdx::ZERO, SlotValue::I64(0))],
            crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            &crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
            tenant,
        )
        .map_err(|error| error.to_string())?;
    assert!(shard.run_state_contains(run));
    let cost = shard
        .run_state_get(run)
        .and_then(|state| state.admission.as_ref())
        .and_then(|admission| admission.budget())
        .map(|budget| budget.max_cost_units)
        .ok_or_else(|| "admitted budget".to_string())?;
    assert!(cost > 0);
    assert_eq!(shard.usage_snapshot().tenant_cost_usage(tenant), Ok(cost));
    assert_eq!(shard.global_cost_usage(), cost);
    shard
        .handle_cancel(run, Some(String::from("test")))
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.usage_snapshot().tenant_cost_usage(tenant), Ok(0));
    assert_eq!(shard.global_cost_usage(), 0);
    Ok(())
}

#[test]
fn tenant_quota_rejects_limit_plus_one_without_mutation() -> Result<(), String> {
    let tenant = vb_core::ids::TenantId::new(8);
    let run = RunId::new(8_002);
    let mut shard = Shard::new(small_config());
    shard.set_resource_quotas(0, u64::MAX);
    let result = shard.charge_admission_quota(run, Some(tenant), 1);
    assert!(matches!(
        result,
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "tenant_quota_cost_units",
            ..
        })
    ));
    assert_eq!(shard.usage_snapshot().tenant_cost_usage(tenant), Ok(0));
    assert_eq!(shard.global_cost_usage(), 0);
    Ok(())
}

#[test]
fn global_quota_rejects_limit_plus_one() -> Result<(), String> {
    let run = RunId::new(8_003);
    let mut shard = Shard::new(small_config());
    shard.set_resource_quotas(u64::MAX, 0);
    let result = shard.charge_admission_quota(run, None, 1);
    assert!(matches!(
        result,
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "global_quota_cost_units",
            ..
        })
    ));
    assert_eq!(shard.global_cost_usage(), 0);
    Ok(())
}

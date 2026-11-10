// BH-SHD-02: take_run_state removes run from map before drive.
// If an error occurs between take and apply_drive_result, the run is lost.
// Severity: Low. Current code structure is safe but fragile.
#[test]
fn bh_shd_02_run_removed_from_map_during_drive() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(802);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run: run,
            workflow: workflow,
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Run was removed and re-inserted by keep_run (Do node suspends again).
    assert_eq!(shard.active_run_count(), 1);
}

// BH-SHD-03: Verify exactly one ActionFailed trace event for non-retryable.
#[test]
fn bh_shd_03_action_failure_trace_events_count() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(803);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run: run,
            workflow: workflow,
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let drained_before_failure = shard.trace_ring_mut().drain();
    assert_ne!(drained_before_failure.len(), 0);
    let ticket = action_ticket(run, vb_core::ids::StepIdx::ZERO);
    let failure = timeout_failure();
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let events = shard.trace_ring_mut().drain();
    let action_failed_count = events.iter().filter(|e| {
        matches!(e, TraceEvent::ActionFailed { run: r, step: vb_core::ids::StepIdx::ZERO, code: _ } if *r == run)
    }).count();
    assert_eq!(
        action_failed_count, 1,
        "BH-SHD-03: expected exactly 1 ActionFailed trace event, got {action_failed_count}"
    );
}

// BH-SHD-04: find_error_handler_for_failure linear scan on large workflows.
// Severity: Low. Performance concern only.
#[test]
fn bh_shd_04_find_error_handler_linear_scan_fallback() {
    let handler_idx = 20u16;
    let first_node = vb_core::workflow::node(vb_core::ids::StepIdx::ZERO, CompiledNodeKind::ErrorHandler { body: vb_core::ids::StepIdx::new(1), handler: vb_core::ids::StepIdx::new(handler_idx), error_slot: None, }, None, None);
    let middle_nodes = (1u16..handler_idx).map(|i| vb_core::workflow::node(vb_core::ids::StepIdx::new(i), CompiledNodeKind::Nop, None, Some(vb_core::ids::StepIdx::new(i + 1))));
    let last_node = vb_core::workflow::node(vb_core::ids::StepIdx::new(handler_idx), CompiledNodeKind::SetConst { value: ConstIdx::new(0), }, Some(SlotIdx::new(0)), None);
    let nodes = std::iter::once(first_node)
        .chain(middle_nodes)
        .chain(std::iter::once(last_node))
        .collect::<Vec<_>>();
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("bh_large_wf"),
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(false)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let result = super::helpers::find_error_handler_for_failure(
        // The body step (step 1) is protected by the ErrorHandler at step 0.
        // Steps 2..N-1 are NOT protected since they are not the body.
        &workflow,
        vb_core::ids::StepIdx::new(1),
    );
    match result {
        Some((handler, _error_slot)) => {
            assert_eq!(
                handler,
                vb_core::ids::StepIdx::new(handler_idx),
                "BH-SHD-04: linear scan should find handler at end of workflow"
            );
        }
        None => {
            panic!("BH-SHD-04: expected to find error handler via linear scan");
        }
    }
}

// BH-SHD-05: drain_for_shutdown processes all queued commands.
#[test]
fn bh_shd_05_drain_for_shutdown_processes_all_queued_commands() {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 8,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let result = shard.drain_for_shutdown();
    assert_eq!(result, Ok(()));
    assert!(shard.is_shutting_down());
}

// BH-SHD-06: SubmitWithInputs allows arbitrary slot writes before validation.
// Severity: Medium. Within-range writes of unexpected types could cause issues.
#[test]
fn bh_shd_06_submit_with_inputs_writes_slots_before_validation() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(806);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(true))]),
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(
                vb_core::ActionId::new(0)
            )
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(_)) => {}
        other => {
            let msg = format!("expected Found, got {other:?}");
            panic!("{msg}");
        }
    }
}

// BH-SHD-07: Frame pool is bounded by capacity — no fresh allocation as fallback.
// Severity: Low. Mitigated by max_active_runs.
#[test]
fn bh_shd_07_frame_pool_bounded_no_fresh_alloc() {
    let mut pool = crate::frame_pool::FramePool::new(2, 1, 2)
        .ok()
        .unwrap_or_else(|| panic!("FramePool::new failed"));
    // With pre-allocation, taking 2 frames exhausts the pool
    let f1 = pool.take(super::RunId::new(1), vb_core::ids::StepIdx::ZERO);
    let f2 = pool.take(super::RunId::new(2), vb_core::ids::StepIdx::ZERO);
    assert!(f1.is_ok(), "BH-SHD-07: f1 should succeed (within capacity)");
    assert!(f2.is_ok(), "BH-SHD-07: f2 should succeed (within capacity)");
    // A 3rd take returns AllocationFailed (no fresh allocation fallback)
    let f3 = pool.take(super::RunId::new(3), vb_core::ids::StepIdx::ZERO);
    assert!(
        matches!(f3, Err(vb_core::errors::CoreError::AllocationFailed)),
        "BH-SHD-07: f3 should fail with AllocationFailed (pool bounded, no fresh alloc)"
    );
}

// BH-SHD-08: pending_timers allows only one timer per run (last wins).
// Severity: Low. Invariant maintained by workflow structure.
#[test]
fn bh_shd_08_pending_timers_last_wins_per_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = super::RunId::new(808);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run: run,
            workflow: workflow,
            caps: test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    let timer1 = shard.pending_timer_get(run);
    assert!(matches!(
        shard.pending_timer_insert(
            run,
            super::types::PendingTimer {
                step: vb_core::ids::StepIdx::new(99),
                kind: super::types::PendingTimerKind::Ask,
                generation: 2,
                deadline: std::time::Instant::now(),
            },
        ),
        Ok(Some(_))
    ));
    let timer2 = shard.pending_timer_get(run);
    assert_ne!(timer1, timer2, "BH-SHD-08: second timer replaced first");
    assert_eq!(
        timer2.map(|t| t.step),
        Some(vb_core::ids::StepIdx::new(99)),
        "BH-SHD-08: replacement timer has different step"
    );
}

// BH-SHD-09: AskAnswer for non-existent run errors correctly.
#[test]
fn bh_shd_09_ask_answer_for_nonexistent_run_errors() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let run = super::RunId::new(809);
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),

            answer_slot: vb_core::ids::SlotIdx::ZERO,
            boundary_seq: vb_storage::EventSeq::ZERO,
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

fn live_resume_fixture(
    run: super::RunId,
) -> Result<
    (
        Shard,
        std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
        vb_core::action::ActionTicket,
    ),
    Box<dyn Error>,
> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow: action_with_error_handler_workflow().ok_or("workflow fixture must compile")?,
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        caps: test_fixtures::contract_capability_for(ActionId::new(0)),
        action_contracts: test_fixtures::contracts_through(ActionId::new(0)),
    })?;
    if !shard.tick()? {
        return Err("submission command was not processed".into());
    }
    let ticket = shard
        .pending_action_get(run)
        .ok_or("action must be pending")?;
    Ok((shard, journal, ticket))
}

#[test]
fn repeated_resume_preserves_live_action_ticket_and_original_completion()
-> Result<(), Box<dyn Error>> {
    let run = super::RunId::new(882);
    let (mut shard, journal, ticket) = live_resume_fixture(run)?;
    let frame = shard
        .run_state_get(run)
        .ok_or("run must exist")?
        .frame
        .checkpoint_v1();
    let before = journal.snapshot()?.len();
    for _ in 0..2 {
        assert_eq!(
            shard
                .handle_resume(run)
                .map_err(|error| format!("resume failed: {error:?}"))?
                .status,
            super::ResumeStatus::Resumed
        );
        assert_eq!(shard.pending_action_get(run), Some(ticket));
        let state = shard
            .run_state_get(run)
            .ok_or("live action must remain active")?;
        assert_eq!(state.frame.checkpoint_v1(), frame);
        assert_eq!(state.action_attempts.get(ticket.step.as_usize()), Some(&1));
    }
    let events = journal.snapshot()?;
    assert_eq!(
        events.len(),
        before.checked_add(2).ok_or("event count overflow")?
    );
    assert!(events.iter().skip(before).all(|event| matches!(event, RuntimeJournalEvent::Resumed { run: resumed, .. } if *resumed == run)));
    shard.enqueue(ShardCommand::ActionCompleted {
        ticket,
        output: vb_core::action::ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(99),
            taint: vb_core::value::Taint::Clean,
            encoded_len: u32::try_from(postcard::to_allocvec(&SlotValue::I64(99))?.len())?,
        },
    })?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(shard.pending_action_get(run), None);
    Ok(())
}

#[test]
fn repeated_resume_rejects_missing_or_stale_live_action_authority() -> Result<(), Box<dyn Error>> {
    enum InvalidAuthority {
        Missing,
        StaleAttempt,
        StaleFence,
    }
    for invalid in [
        InvalidAuthority::Missing,
        InvalidAuthority::StaleAttempt,
        InvalidAuthority::StaleFence,
    ] {
        let run = super::RunId::new(883);
        let (mut shard, journal, ticket) = live_resume_fixture(run)?;
        match invalid {
            InvalidAuthority::Missing => {
                shard.pending_action_remove(run);
            }
            InvalidAuthority::StaleAttempt => {
                let attempts = &mut shard
                    .run_state_get_mut(run)
                    .ok_or("run must exist")?
                    .action_attempts;
                *attempts
                    .get_mut(ticket.step.as_usize())
                    .ok_or("action counter must exist")? = 2;
            }
            InvalidAuthority::StaleFence => {
                let _token = shard.fencing_tokens.mint(run);
            }
        }
        shard
            .handle_resume(run)
            .map_err(|error| format!("resume failed: {error:?}"))?;
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.pending_action_get(run), None);
        assert_eq!(
            journal
                .snapshot()?
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn repeated_resume_enforces_live_action_budget() -> Result<(), Box<dyn Error>> {
    let run = super::RunId::new(884);
    let (mut shard, journal, _) = live_resume_fixture(run)?;
    let state = shard.run_state_get_mut(run).ok_or("run must exist")?;
    let admission = state.admission.clone().ok_or("admission must exist")?;
    let mut budget = admission.budget().ok_or("admission budget must exist")?;
    budget.max_cost_units = 0;
    state.admission = Some(admission.with_attached_budget(budget));
    shard
        .handle_resume(run)
        .map_err(|error| format!("resume failed: {error:?}"))?;
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    assert_eq!(
        journal
            .snapshot()?
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    Ok(())
}

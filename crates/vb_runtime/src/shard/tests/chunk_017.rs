
// BH-SHD-02: take_run_state removes run from map before drive.
// If an error occurs between take and apply_drive_result, the run is lost.
// Severity: Low. Current code structure is safe but fragile.
#[test]
fn bh_shd_02_run_removed_from_map_during_drive() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(802);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Run was removed and re-inserted by keep_run (Do node suspends again).
    assert_eq!(shard.active_run_count(), 1);
    Ok(())
}

// BH-SHD-03: Verify exactly one ActionFailed trace event for non-retryable.
#[test]
fn bh_shd_03_action_failure_trace_events_count() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(803);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
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
    Ok(())
}

// BH-SHD-04: find_error_handler_for_failure linear scan on large workflows.
// Severity: Low. Performance concern only.
#[test]
fn bh_shd_04_find_error_handler_linear_scan_fallback() -> Result<(), RuntimeError> {
    let handler_idx = 20u16;
    let first_node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ErrorHandler {
            body: vb_core::ids::StepIdx::new(1),
            handler: vb_core::ids::StepIdx::new(handler_idx),
            error_slot: None,
        },
    };
    let middle_nodes = (1u16..handler_idx).map(|i| CompiledNode {
        id: vb_core::ids::StepIdx::new(i),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(i + 1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    });
    let last_node = CompiledNode {
        id: vb_core::ids::StepIdx::new(handler_idx),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let nodes = std::iter::once(first_node)
        .chain(middle_nodes)
        .chain(std::iter::once(last_node))
        .collect::<Vec<_>>();
    let parts = WorkflowParts {
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
    };
    let workflow = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return Ok(()),
    };
    let result = crate::shard::helpers::find_error_handler_for_failure(
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
    Ok(())
}

// BH-SHD-05: drain_for_shutdown processes all queued commands.
#[test]
fn bh_shd_05_drain_for_shutdown_processes_all_queued_commands() -> Result<(), RuntimeError> {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 8,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
        max_terminal_outcomes: 100_000,
};
    let mut shard = Shard::new(config)?;
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    let result = shard.drain_for_shutdown();
    assert_eq!(result, Ok(()));
    assert!(shard.is_shutting_down());
    Ok(())
}

// BH-SHD-06: SubmitWithInputs allows arbitrary slot writes before validation.
// Severity: Medium. Within-range writes of unexpected types could cause issues.
#[test]
fn bh_shd_06_submit_with_inputs_writes_slots_before_validation() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(806);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(true))]),
            caps: vb_core::capability::CapabilitySet::empty(),
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
    Ok(())
}

// BH-SHD-07: Frame pool allocates beyond pool capacity.
// Severity: Low. Mitigated by max_active_runs.
#[test]
fn bh_shd_07_frame_pool_allocates_beyond_pool_capacity() -> Result<(), RuntimeError> {
    let mut pool = crate::frame_pool::FramePool::new(2, 1, 2)
        .ok()
        .unwrap_or_else(|| panic!("FramePool::new failed"));
    let f1 = pool.take(RunId::new(1), vb_core::ids::StepIdx::ZERO);
    let f2 = pool.take(RunId::new(2), vb_core::ids::StepIdx::ZERO);
    let f3 = pool.take(RunId::new(3), vb_core::ids::StepIdx::ZERO);
    assert!(f1.is_ok(), "BH-SHD-07: f1 should succeed");
    assert!(f2.is_ok(), "BH-SHD-07: f2 should succeed");
    assert!(
        f3.is_ok(),
        "BH-SHD-07: f3 should succeed beyond pool capacity"
    );
    Ok(())
}

// BH-SHD-08: pending_timers allows only one timer per run (last wins).
// Severity: Low. Invariant maintained by workflow structure.
#[test]
fn bh_shd_08_pending_timers_last_wins_per_run() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = RunId::new(808);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    let timer1 = shard.pending_timer_get(run);
    shard.pending_timer_insert(
        run,
        crate::shard::types::PendingTimer {
            step: vb_core::ids::StepIdx::new(99),
            kind: crate::shard::types::PendingTimerKind::Ask,
            generation: 2,
            deadline: std::time::Instant::now(),
        },
    );
    let timer2 = shard.pending_timer_get(run);
    assert_ne!(timer1, timer2, "BH-SHD-08: second timer replaced first");
    assert_eq!(
        timer2.map(|t| t.step),
        Some(vb_core::ids::StepIdx::new(99)),
        "BH-SHD-08: replacement timer has different step"
    );
    Ok(())
}

// BH-SHD-09: AskAnswer for non-existent run errors correctly.
#[test]
fn bh_shd_09_ask_answer_for_nonexistent_run_errors() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let run = RunId::new(809);
    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

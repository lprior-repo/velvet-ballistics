#[test]
fn shard_step_budget_one_does_not_recount_completed_steps_on_later_ticks() -> Result<(), String> {
    // Given a shard with step_budget_per_tick = 1
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed, ..Default::default()
    };
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(finished_workflow(), "finished_workflow")?;
    // When submitting a 2-step finished workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then later ticks do not recount the already-accounted step.
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().steps_executed, 1);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().steps_executed, 1);
    Ok(())
}

#[test]
fn shard_executed_step_accounting_adds_only_new_delta() {
    // Given a shard with no accounted executed steps for the run
    let mut shard = Shard::new(small_config());
    let run = super::RunId::new(515);

    // When the same cumulative value is observed more than once
    shard.add_executed_step_delta(run, 1);
    shard.add_executed_step_delta(run, 1);

    // Then only the first newly observed step is counted.
    assert_eq!(shard.counters().snapshot().steps_executed, 1);

    // And a later cumulative increase counts only the delta.
    shard.add_executed_step_delta(run, 3);
    assert_eq!(shard.counters().snapshot().steps_executed, 3);
}

#[test]
fn shard_duplicate_run_id_returns_run_already_exists_after_first_accepted() -> Result<(), String> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let wf1 = workflow_fixture(suspended_workflow(), "suspended_workflow wf1")?;
    let wf2 = workflow_fixture(suspended_workflow(), "suspended_workflow wf2")?;
    let run = super::RunId::new(42);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When submitting the same run ID again with a different workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick returns RunAlreadyExists (cannot replace workflow)
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    Ok(())
}

#[test]
fn shard_action_failed_for_unknown_run_returns_run_not_found() {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(999),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let failure = vb_core::action::ActionFailure {
        code: vb_core::action::ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    // When failing an action for a non-existent run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_run_id_max_u64_accepted_as_valid_identifier() -> Result<(), String> {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(finished_workflow(), "finished_workflow")?;
    let run = super::RunId::new(u64::MAX);
    // When submitting a run with RunId::MAX
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the run is accepted and completes
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_ask_answered_for_unknown_run_returns_run_not_found() {
    // Given a shard with no active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(999),
            ask_step: vb_core::ids::StepIdx::new(0),
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::SlotValue::I64(42),
        taint: vb_core::Taint::Clean,
        encoded_len: 0,
    };
    // When answering an ask for a non-existent run
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn shard_snapshot_for_nonexistent_run_returns_not_found() {
    // Given a shard with no runs
    let config = small_config();
    let shard = Shard::new(config);
    // When snapshotting a non-existent run
    let response = shard.snapshot_run(super::RunId::new(999), 42);
    // Then NotFound is returned
    assert_eq!(
        response,
        InspectResponse::NotFound {
            run: super::RunId::new(999),
            correlation: 42,
        }
    );
}

#[test]
fn shard_cancel_then_resubmit_same_run_id_succeeds() -> Result<(), String> {
    // Given a shard with an active run
    let config = small_config();
    let mut shard = Shard::new(config);
    let wf1 = workflow_fixture(suspended_workflow(), "suspended_workflow")?;
    let wf2 = workflow_fixture(finished_workflow(), "finished_workflow")?;
    let run = super::RunId::new(55);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When cancelling and re-submitting with same ID
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the re-submitted run completes
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    Ok(())
}

#[test]
fn shard_trace_ring_records_submit_and_finish_events_in_order() -> Result<(), String> {
    // Given a shard
    let config = small_config();
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(finished_workflow(), "finished_workflow")?;
    let run = super::RunId::new(77);
    // When submitting a run that finishes immediately
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then trace ring has Submit and Finished events
    let events = shard.trace_ring_mut().drain();
    let found_submit = events
        .iter()
        .any(|e| *e == TraceEvent::RunSubmitted { run });
    let found_finish = events.iter().any(|e| *e == TraceEvent::RunFinished { run });
    assert_eq!(found_submit, true);
    assert_eq!(found_finish, true);
    Ok(())
}

#[test]
fn shard_with_zero_trace_capacity_does_not_crash_on_submit() -> Result<(), String> {
    // Given a shard with trace_capacity = 0
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 0,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed, ..Default::default()
    };
    let mut shard = Shard::new(config);
    let workflow = workflow_fixture(finished_workflow(), "finished_workflow")?;
    // When submitting a run
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    // Then tick succeeds (trace drops are non-fatal)
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_command_queue_len_starts_at_zero() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then queue length is 0
    assert_eq!(shard.command_queue_len(), 0);
}

#[test]
fn shard_command_queue_len_increments_on_enqueue() {
    // Given a shard with capacity 4
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed, ..Default::default()
    };
    let shard = Shard::new(config);
    assert_eq!(shard.command_queue_len(), 0);
    // When enqueuing commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.command_queue_len(), 2);
}

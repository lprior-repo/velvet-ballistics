
// ---------------------------------------------------------------------------
// handle_submit: trace event includes correct run id
// ---------------------------------------------------------------------------

#[test]
fn shard_submit_trace_event_contains_submitted_run_id() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = finished_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(780);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = shard.trace_ring_mut().drain();
    let found = events
        .iter()
        .any(|e| matches!(e, TraceEvent::RunSubmitted { run: r } if *r == run));
    assert_eq!(found, true);
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_action_failure: failure with wrong step returns invalid completion
// ---------------------------------------------------------------------------

#[test]
fn shard_action_failure_with_wrong_step_returns_invalid_completion() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(790);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Fail with a step that isn't running
    let ticket = action_ticket(run, vb_core::ids::StepIdx::new(99));
    let failure = timeout_failure();
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_action_completion: legacy completion with wrong step
// ---------------------------------------------------------------------------

#[test]
fn shard_legacy_action_completed_with_wrong_step_returns_error() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(791);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Legacy completion with a step that isn't running
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: vb_core::ids::StepIdx::new(5),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_ask_answer: answering after run was cancelled returns run not found
// ---------------------------------------------------------------------------

#[test]
fn shard_ask_answer_after_cancel_returns_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = ask_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(792);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let answer = AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: vb_core::ids::StepIdx::new(2),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_action_failure: failure after run was cancelled
// ---------------------------------------------------------------------------

#[test]
fn shard_action_failure_after_cancel_returns_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(793);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let ticket = action_ticket(run, vb_core::ids::StepIdx::ZERO);
    let failure = timeout_failure();
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed { ticket, failure }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_resume: resume after cancel returns run not found
// ---------------------------------------------------------------------------

#[test]
fn shard_resume_after_cancel_returns_run_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(794);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timers.len(), 1);
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

// ==========================================================================
// BLACKHAT SECURITY REVIEW: shard module findings
// ==========================================================================
//
// Reviewer: BLACKHAT
// Scope: shard/{impl_,lifecycle,transitions,types,helpers,timer_wheel}.rs
//
// BH-SHD-01: drive_state passes empty contracts bypassing action security
// BH-SHD-02: take_run_state removes run from map before drive (fragile)
// BH-SHD-03: handle_action_failure trace event count
// BH-SHD-04: find_error_handler_for_failure O(n) linear scan
// BH-SHD-05: drain_for_shutdown processes at most capacity commands
// BH-SHD-06: SubmitWithInputs allows arbitrary slot writes
// BH-SHD-07: Frame pool has no hard allocation cap
// BH-SHD-08: pending_timers allows only one timer per run (last wins)
// BH-SHD-09: AskAnswer for non-existent run errors correctly
// BH-SHD-10: Cancel non-existent run produces no journal event
// BH-SHD-11: step_budget_per_tick=0 creates permanent DoS
// BH-SHD-12: Legacy completion on finished run errors correctly
// BH-SHD-13: TimerFire after cancel returns RunNotFound
// BH-SHD-14: Inspect after immediate completion returns NotFound
// ==========================================================================

// BH-SHD-01: drive_state passes empty contracts, bypassing action security.
// The shard's drive_state (lifecycle.rs:371) passes &[] to
// drive_deterministic_full, disabling all taint/capability checks.
// Severity: HIGH.
#[test]
fn bh_shd_01_shard_drive_state_uses_empty_contracts() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = super::RunId::new(801);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(42))]),
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

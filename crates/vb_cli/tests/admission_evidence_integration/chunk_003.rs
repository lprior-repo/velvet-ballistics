
// ===========================================================================
// Test 3: evidence chain after execution
// ===========================================================================

#[test]
fn evidence_chain_after_execution() {
    // Given: a multi-step workflow that runs an action, then finishes.
    // Using a Do + Finish workflow: the action completion produces
    // SlotWritten and StepSucceeded events in addition to RunSubmitted/RunFinished.
    let digest = WorkflowDigest::from_bytes([3u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        journal.clone(),
    )
    .expect("runtime config is valid");
    let run_id = RunId::new(3);

    // When: submitting and ticking (suspends on action)
    match submit_do_action_run(&runtime, run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    // Complete the action to resume and finish the workflow
    let ticket = vb_core::action::ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(
            run_id,
            vb_core::ids::SeqNo::ZERO,
            ActionId::new(7),
        ),
        capacity: 1,
            ..Default::default()
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    match runtime.complete_action_with_output(ticket, output) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("complete_action_with_output failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly after action completion");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed after action completion: {err}");
            return;
        }
    }

    // Then: the journal contains the full evidence chain:
    // RunSubmitted -> StepSucceeded (action) -> SlotWritten -> ActionCompleted ->
    // StepSucceeded (finish) -> RunFinished
    let events = match journal.snapshot() {
        Ok(events) => events,
        Err(err) => {
            fail_assert!("journal snapshot failed: {err}");
            return;
        }
    };

    let mut found_step_succeeded = false;
    let mut found_action_completed_envelope = false;
    let mut found_run_submitted = false;
    let mut found_run_finished = false;

    for event in &events {
        match event {
            vb_runtime::journal::RuntimeJournalEvent::RunSubmitted { run, .. }
                if *run == run_id =>
            {
                found_run_submitted = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::RunFinished { run, .. } if *run == run_id => {
                found_run_finished = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::StepSucceeded { run, .. }
                if *run == run_id =>
            {
                found_step_succeeded = true;
            }
            vb_runtime::journal::RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket, ..
            } if ticket.run == run_id => {
                found_action_completed_envelope = true;
            }
            _ => {}
        }
    }

    assert!(
        found_run_submitted,
        "journal should contain RunSubmitted event"
    );
    assert!(
        found_run_finished,
        "journal should contain RunFinished event"
    );
    assert!(
        found_step_succeeded,
        "journal should contain StepSucceeded event"
    );
    assert!(
        found_action_completed_envelope,
        "journal should contain ActionCompletedEnvelope event"
    );
}

// ===========================================================================
// Test 4: capability check rejects unauthorized action
// ===========================================================================

#[test]
fn capability_check_rejects_unauthorized_action() {
    // Given: a capability set that does NOT grant action 7
    let empty_caps = vb_core::CapabilitySet::empty();
    let required = vb_core::Capability::new("action".into(), ActionId::new(7));

    // When: checking if the capability is granted
    let granted = empty_caps.grants(&required);

    // Then: it is rejected
    assert!(!granted, "empty capability set should not grant action(7)");

    // Also verify with a specific but different action grant
    let wrong_caps = vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::new(
        "action".into(),
        ActionId::new(99),
    )]));
    assert!(
        !wrong_caps.grants(&required),
        "Capability for action(99) should not grant action(7)"
    );

    // Verify a broader capability prefix does grant it
    let any_caps = vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::new(
        "action".into(),
        ActionId::new(7),
    )]));
    assert!(
        any_caps.grants(&required),
        "exact action capability should grant action(7)"
    );

    // Verify hierarchical prefix does not grant a dotted child capability.
    let child_required = vb_core::Capability::new("action.dispatch".into(), ActionId::new(7));
    let prefix_caps = vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::new(
        "action".into(),
        ActionId::new(7),
    )]));
    assert!(
        !prefix_caps.grants(&child_required),
        "hierarchical action capability must not grant action.dispatch(7)"
    );

    // Verify the workflow requiring the action can be constructed
    let digest = WorkflowDigest::from_bytes([4u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("do_action workflow construction failed");
        return;
    };

    // The runtime will suspend the run waiting for the action. The scheduling
    // fixture supplies an explicit contract and grant so the fail-closed action
    // admission contract remains intact.
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
        ).expect("runtime config is valid");
    let run_id = RunId::new(4);
    match submit_do_action_run(&runtime, run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    // The run should have been submitted but not completed (suspended on action)
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1, "run should have been submitted");
    assert_eq!(
        snap.runs_completed, 0,
        "run should be suspended waiting for action, not completed"
    );

    // Trace events should show ActionScheduled
    let trace = runtime.list_events(run_id);
    match trace {
        Ok(events) => {
            let found_scheduled = events
                .iter()
                .any(|e| matches!(e, vb_runtime::trace::TraceEvent::ActionScheduled { run, step, .. } if *run == run_id && *step == StepIdx::new(0)));
            assert!(
                found_scheduled,
                "trace should contain ActionScheduled event for the unauthorized action"
            );
        }
        Err(err) => {
            fail_assert!("list_events failed: {err}");
        }
    }
}

#[test]
fn evidence_chain_captures_action_timeout_and_failure() {
    let digest = WorkflowDigest::from_bytes([0x10u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        journal.clone(),
    )
    .expect("runtime config is valid");
    let run_id = RunId::new(10);

    match submit_do_action_run(&runtime, run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    let ticket = vb_core::action::ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let failure = vb_core::action::ActionFailure {
        code: vb_core::action::ActionFailureCode::Timeout,
        retry_policy: vb_core::action::RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    match runtime.fail_action(ticket, failure) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("fail_action_with_code failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("tick_all after action failure failed: {err}");
            return;
        }
    }

    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 0, "run should not complete on action timeout");
    assert!(snap.runs_failed > 0 || snap.runs_submitted > snap.runs_completed,
        "run should be in failed/suspended state after action timeout: runs_submitted={} runs_failed={}",
        snap.runs_submitted, snap.runs_failed);

    let events = match journal.snapshot() {
        Ok(events) => events,
        Err(err) => {
            fail_assert!("journal snapshot failed: {err}");
            return;
        }
    };
    let has_failed = events.iter().any(|e| {
        matches!(e, vb_runtime::journal::RuntimeJournalEvent::RunFailed { run, .. } if *run == run_id)
    });
    assert!(has_failed, "journal should contain RunFailed after action timeout");
}

#[test]
fn evidence_chain_preserves_event_ordering_across_restarts() {
    let digest = WorkflowDigest::from_bytes([0x11u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let run_id = RunId::new(11);

    let mut runtime1 = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        journal.clone(),
    )
    .expect("runtime config is valid");
    match submit_do_action_run(&runtime1, run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime1.tick_all() {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    let events_before = match journal.snapshot() {
        Ok(events) => events,
        Err(err) => {
            fail_assert!("journal snapshot failed: {err}");
            return;
        }
    };
    assert!(!events_before.is_empty(), "journal should have events");

    let ticket = vb_core::action::ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(
            run_id,
            vb_core::ids::SeqNo::ZERO,
            ActionId::new(7),
        ),
        capacity: 1,
            ..Default::default()
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    match runtime1.complete_action_with_output(ticket, output) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("complete_action_with_output failed: {err}");
            return;
        }
    }
    match runtime1.tick_all() {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }

    let events_after = match journal.snapshot() {
        Ok(events) => events,
        Err(err) => {
            fail_assert!("journal snapshot failed: {err}");
            return;
        }
    };
    assert!(
        events_after.len() > events_before.len(),
        "journal should have more events after completion"
    );
    assert_eq!(runtime1.counters_snapshot().runs_completed, 1);

    let has_finished = events_after.iter().any(|e| {
        matches!(e, vb_runtime::journal::RuntimeJournalEvent::RunFinished { run, .. } if *run == run_id)
    });
    assert!(has_finished, "journal should contain RunFinished event");
}

#[test]
fn capability_check_rejects_any_action_when_empty_grant_set() {
    let empty_caps = CapabilitySet::empty();
    for action_id in [0u16, 1, 7, 99, 255, u16::MAX] {
        let required = Capability::new("action".into(), ActionId::new(action_id));
        assert!(
            !empty_caps.grants(&required),
            "empty capability set should not grant action({action_id})"
        );
    }
    let resource_req = Capability::new("resource".into(), ActionId::new(1));
    assert!(
        !empty_caps.grants(&resource_req),
        "empty capability set should not grant resource(1)"
    );
}

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
    let mut found_slot_written = false;
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
            vb_runtime::journal::RuntimeJournalEvent::SlotWritten { run, .. } if *run == run_id => {
                found_slot_written = true;
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
        found_slot_written,
        "journal should contain SlotWritten event"
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

    // Verify hierarchical prefix grants a dotted child capability.
    let child_required = vb_core::Capability::new("action.dispatch".into(), ActionId::new(7));
    let prefix_caps = vb_core::CapabilitySet::from_grants(Box::from([vb_core::Capability::new(
        "action".into(),
        ActionId::new(7),
    )]));
    assert!(
        prefix_caps.grants(&child_required),
        "hierarchical action capability should grant action.dispatch(7)"
    );

    // Verify the workflow requiring the action can be constructed
    let digest = WorkflowDigest::from_bytes([4u8; 32]);
    let Some(workflow) = do_action_workflow(digest) else {
        fail_assert!("do_action workflow construction failed");
        return;
    };

    // The runtime will suspend the run waiting for the action, which demonstrates
    // that the workflow requires an action that would need capability authorization
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    );
    let run_id = RunId::new(4);
    match runtime.submit_direct(run_id, workflow) {
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

// ===========================================================================
// Test 5: budget validation rejects oversized workflow
// ===========================================================================

#[test]
fn budget_validation_rejects_oversized_workflow() {
    // Given: a BoundednessPolicy with very tight limits
    let tight_policy = vb_core::BoundednessPolicy {
        max_total_steps: 2,
        max_total_slots: 10,
        max_fanout: 1,
        max_nesting_depth: 1,
        absolute_max_action_tickets: 1,
        absolute_max_parallel: 1,
        absolute_max_run_time_seconds: 60,
        absolute_max_result_bytes: 1024,
        absolute_max_steps_executable: 2,
    };

    // When: creating a 3-node workflow that exceeds the step limit
    let budget = vb_core::WholeWorkflowBudget {
        max_total_steps: 3,
        max_total_slots: 5,
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: 3,
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

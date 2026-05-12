            assert_eq!(
                artifact.digest, digest,
                "submit_artifact should return the workflow digest"
            );
        }
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    }

    // Then: the artifact is stored and the workflow can be loaded and run
    let stored = journal.compiled_ir(digest);
    match stored {
        Ok(Some(_record)) => {}
        Ok(None) => {
            fail_assert!("artifact should be stored after submit_artifact");
            return;
        }
        Err(err) => {
            fail_assert!("compiled_ir lookup failed: {err}");
            return;
        }
    }

    // Run the workflow through the runtime to verify end-to-end success
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    );
    let run_id = RunId::new(1);
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
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "workflow should complete successfully after artifact submission"
    );
}

// ===========================================================================
// Test 2: run without artifact under relaxed policy
// ===========================================================================

#[test]
fn run_without_artifact_under_relaxed_policy() {
    // Given: a compiled workflow and a relaxed policy
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let digest = WorkflowDigest::from_bytes([2u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    // When: submitting under Relaxed policy (no verification required)
    let result = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    match result {
        Ok(artifact) => {
            assert_eq!(artifact.digest, digest);
        }
        Err(err) => {
            fail_assert!("relaxed submit_artifact should succeed: {err}");
            return;
        }
    }

    // Then: the artifact is stored and the workflow can run
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    );
    let run_id = RunId::new(2);
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
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "relaxed policy should allow running without strict verification"
    );
}

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
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(shard_count, test_config(), journal.clone());
    let run_id = RunId::new(3);

    // When: submitting and ticking (suspends on action)
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

    // Complete the action to resume and finish the workflow
    let ticket = vb_core::action::ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
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



/// Workflow: SetConst(slot0=true) -> Ask(prompt=slot0, timeout=Some(slot1)) -> AskResume(answer=slot2) -> Finish(result=slot2)
fn ask_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let set_timeout = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
    };
    let resume = CompiledNode {
        id: vb_core::ids::StepIdx::new(3),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(4)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(4),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_then_finish"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            vb_core::value::ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn retryable_failure() -> vb_core::action::ActionFailure {
    vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: vb_core::action::RetryPolicy::Retryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// ---------------------------------------------------------------------------
// handle_submit: valid workflow with inputs via SubmitWithInputs
// ---------------------------------------------------------------------------

#[test]
fn shard_submit_with_inputs_seeds_slots_and_drives() -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    let run = super::RunId::new(700);
    let inputs = Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(true))]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

#[test]
fn shard_submit_with_inputs_rejects_duplicate_run() -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let run = super::RunId::new(701);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let inputs = Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::Bool(false))]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    Ok(())
}

#[test]
fn shard_submit_with_inputs_rejects_capacity_exceeded() -> Result<(), &'static str> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    
};
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let wf1 = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let wf2 = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let inputs = Box::from([]);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run: super::RunId::new(2),
            workflow: wf2,
            inputs,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
    Ok(())
}

struct RejectAdmissionHeaderJournal {
    reject_submitted: bool,
    reject_admission: bool,
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectAdmissionHeaderJournal {
    fn shared(reject_submitted: bool, reject_admission: bool) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            reject_submitted,
            reject_admission,
            events: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> crate::RuntimeResult<Vec<RuntimeJournalEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RejectAdmissionHeaderJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        let reject_submitted =
            matches!(event, RuntimeJournalEvent::RunSubmitted { .. }) && self.reject_submitted;
        let reject_admission =
            matches!(event, RuntimeJournalEvent::RunAdmission { .. }) && self.reject_admission;
        if reject_submitted || reject_admission {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

fn assert_admission_header_persistence_failed(result: crate::RuntimeResult<bool>) {
    let is_expected = matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    );
    assert!(is_expected, "expected AdmissionHeaderPersistenceFailed");
}

#[test]
fn shard_submit_run_submitted_append_failure_maps_to_admission_header_persistence_failed(
) -> Result<(), &'static str> {
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let journal = RejectAdmissionHeaderJournal::shared(true, false);
    let mut shard = match Shard::new_with_journal_and_artifact_store(        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let run = super::RunId::new(712);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    assert_admission_header_persistence_failed(shard.tick());

    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    assert_eq!(journal.snapshot(), Ok(Vec::new()));
    Ok(())
}

#[test]
fn shard_submit_run_admission_append_failure_maps_to_admission_header_persistence_failed(
) -> Result<(), &'static str> {
    let workflow = suspended_workflow().ok_or("suspended workflow fixture construction failed")?;
    let journal = RejectAdmissionHeaderJournal::shared(false, true);
    let mut shard = match Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    ) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let run = super::RunId::new(713);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    assert_admission_header_persistence_failed(shard.tick());

    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    match journal.snapshot() {
        Ok(events) => {
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events.first(),
                Some(RuntimeJournalEvent::RunSubmitted { .. })
            ));
        }
        Err(error) => panic!("journal snapshot failed: {error:?}"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_resume: resume a waiting run after timer was already removed
// ---------------------------------------------------------------------------

#[test]
fn shard_resume_on_waiting_run_after_timer_removed_still_suspends() -> Result<(), &'static str> {
    // Submit a timed wait workflow, which enters a wait-suspended state with a pending timer.
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow =
        timed_wait_then_finish_workflow().ok_or("timed wait workflow fixture construction failed")?;
    let run = super::RunId::new(710);
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

    // When resuming while the run is waiting, drive_run re-drives and re-suspends
    // because the WaitUntil deadline hasn't been met (no timer fire).
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Run is still active (re-suspended)
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 1
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run);
        }
        other => assert_eq!(other, None),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_cancel: cancel a finished run (no-op, already removed)
// ---------------------------------------------------------------------------

#[test]
fn shard_cancel_on_finished_run_succeeds_silently_without_counter_increment(
) -> Result<(), &'static str> {
    let config = small_config();
    let mut shard = match Shard::new(config) {
        Ok(s) => s,
        Err(_) => return Err("shard construction failed"),
    };
    let workflow = finished_workflow().ok_or("finished workflow fixture construction failed")?;
    let run = super::RunId::new(720);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // When cancelling the already-finished run
    assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    // Then no additional counter increment
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    Ok(())
}

use crate::journal::StorageRuntimeJournal;
use vb_storage::EventSeq;
use vb_storage::FjallJournal;

fn compat_gate_temp_journal() -> (tempfile::TempDir, std::sync::Arc<FjallJournal>) {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).expect("create target dir for temp journal");
    let dir = tempfile::Builder::new()
        .prefix("vb-runtime-compat-gate-")
        .tempdir_in(base)
        .expect("create temp journal dir");
    let journal = FjallJournal::open(dir.path(), None).expect("open temp journal");
    (dir, std::sync::Arc::new(journal))
}

fn compat_gate_journal_replay_only(journal: &FjallJournal, run: RunId, seq: EventSeq) {
    let decision = vb_storage::compat::gate_admission(
        journal,
        run,
        seq,
        vb_storage::compat::wire_version_inputs(
            vb_storage::compat::CompatBoundary::IrVersion,
            1,
            2,
            false,
            true,
        ),
        [1u8; 32],
        [2u8; 32],
    )
    .expect("journal replay-only decision");
    assert_eq!(
        decision.class,
        vb_storage::compat::CompatibilityClass::ReplayOnly
    );
}

fn compat_gate_blocked_error(run: RunId) -> RuntimeError {
    RuntimeError::CompatGateBlocked {
        run,
        source: vb_storage::compat::CompatError::ReplayOnlyLiveEffect {
            boundary: vb_storage::compat::CompatBoundary::IrVersion,
        },
    }
}

fn compat_gate_submit_suspended(shard: &mut Shard, run: RunId) {
    let Some(workflow) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let workflow = match honest_digest_workflow(workflow) {
        Ok(workflow) => workflow,
        Err(error) => {
            assert!(false, "compatibility workflow must be canonical: {error}");
            return;
        }
    };
    let contracts = strict_submit_contracts();
    let Some(journal) = crate::journal::RuntimeJournal::storage_journal(shard.journal.as_ref())
    else {
        assert!(false, "compatibility fixture requires Fjall journal");
        return;
    };
    let artifact = vb_storage::submit_artifact_with_contracts(
        journal.as_ref(),
        &workflow,
        vb_core::policy::RuntimePolicy::Journaled,
        &contracts,
    );
    if let Err(error) = artifact {
        assert!(false, "compatibility artifact must persist: {error}");
        return;
    }
    shard.policy = vb_core::policy::RuntimePolicy::Journaled;
    shard.artifact_store = crate::admission::StorageArtifactStore::shared(journal);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: strict_submit_caps(),
            action_contracts: contracts,
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        }),
        Ok(())
    );
}

fn compat_gate_next_seq(shard: &mut Shard, run: RunId) -> Option<EventSeq> {
    let seq = shard.journal_seq_get(run)?;
    let next = seq.get().checked_add(1).map(EventSeq::new)?;
    shard
        .seed_journal_sequence(run, next)
        .expect("reseed journal sequence past the decision");
    Some(next)
}

#[test]
fn compat_gate_blocks_submit_with_journaled_replay_only_decision() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_001);
    compat_gate_journal_replay_only(&journal, run, EventSeq::ZERO);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
    shard
        .seed_journal_sequence(run, EventSeq::new(1))
        .expect("seed journal sequence past the decision");
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Err(compat_gate_blocked_error(run)));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Initial)
    );
    let events = journal.events_for_run(run).expect("read journal events");
    assert_eq!(events.len(), 5);
}

#[test]
fn queued_resume_preserves_initial_state_after_compatibility_gate_blocks_start() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_005);
    compat_gate_journal_replay_only(&journal, run, EventSeq::ZERO);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .expect("valid test configuration");
    shard
        .seed_journal_sequence(run, EventSeq::new(1))
        .expect("seed journal sequence past the decision");
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Err(compat_gate_blocked_error(run)));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Initial)
    );
    assert!(shard.run_state_contains(run));

    let events_before = journal.events_for_run(run).expect("read journal events");
    let counters_before = shard.counters().snapshot();
    let pending_actions_before = shard.pending_action_clone();
    let pending_timers_before = shard.pending_timer_count();
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));

    let error = shard.tick().expect_err("initial run must not resume");
    assert_eq!(
        error,
        RuntimeError::NotResumable {
            run,
            current_state: super::super::RuntimeState::Initial,
        }
    );
    assert_ne!(
        error,
        RuntimeError::NotResumable {
            run: RunId::new(91_006),
            current_state: super::super::RuntimeState::Initial,
        }
    );
    assert_ne!(
        error,
        RuntimeError::NotResumable {
            run,
            current_state: super::super::RuntimeState::Running,
        }
    );
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::UNSUPPORTED_OPERATION_CODE
    );
    assert_eq!(error.symbolic_code().as_str(), "UNSUPPORTED_OPERATION");
    assert_eq!(error.runtime_code(), None);
    assert_eq!(
        error.to_string(),
        format!("run {run:?} cannot resume from state Initial")
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Initial)
    );
    assert!(shard.run_state_contains(run));
    assert_eq!(shard.pending_action_clone(), pending_actions_before);
    assert_eq!(shard.pending_timer_count(), pending_timers_before);
    assert_eq!(
        journal.events_for_run(run).expect("read journal events after resume"),
        events_before
    );
    assert_eq!(shard.counters().snapshot(), counters_before);
}

#[test]
fn compat_gate_blocks_resume_with_journaled_replay_only_decision() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_002);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    let Some(seq) = shard.journal_seq_get(run) else {
        assert_eq!(None::<()>, Some(()), "run must hold a journal sequence");
        return;
    };
    compat_gate_journal_replay_only(&journal, run, seq);
    if compat_gate_next_seq(&mut shard, run).is_none() {
        assert_eq!(None::<()>, Some(()), "journal sequence must advance");
        return;
    }
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Err(compat_gate_blocked_error(run)));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
}

#[test]
fn compat_gate_allows_continuation_after_journaled_decision() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_003);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Ok(true));
    let Some(seq) = shard.journal_seq_get(run) else {
        assert_eq!(None::<()>, Some(()), "run must hold a journal sequence");
        return;
    };
    compat_gate_journal_replay_only(&journal, run, seq);
    if compat_gate_next_seq(&mut shard, run).is_none() {
        assert_eq!(None::<()>, Some(()), "journal sequence must advance");
        return;
    }
    let ticket = pending_ticket(&shard, run, StepIdx::ZERO, 1);
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.run_state_get(run), None);
}

#[test]
fn compat_gate_passes_submit_with_no_journaled_decision() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_004);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared).expect("valid test configuration");
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
}

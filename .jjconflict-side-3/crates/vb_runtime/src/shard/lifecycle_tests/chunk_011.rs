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
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(
                ActionId::new(0)
            ),
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
    let mut shard = Shard::new_with_journal(small_config(), shared);
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
    assert_eq!(events.len(), 3);
}

#[test]
fn compat_gate_blocks_resume_with_journaled_replay_only_decision() {
    let (_temp, journal) = compat_gate_temp_journal();
    let run = RunId::new(91_002);
    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut shard = Shard::new_with_journal(small_config(), shared);
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
    let mut shard = Shard::new_with_journal(small_config(), shared);
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
    let mut shard = Shard::new_with_journal(small_config(), shared);
    compat_gate_submit_suspended(&mut shard, run);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
}

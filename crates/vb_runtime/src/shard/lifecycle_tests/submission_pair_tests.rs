fn submit_pair_fixture(shard: &mut Shard, run: RunId) -> crate::RuntimeResult<()> {
    let workflow = suspended_workflow().ok_or(RuntimeError::InvalidActionCompletion)?;
    shard.handle_submit_with_inputs_and_contracts(
        run,
        workflow,
        &[(SlotIdx::ZERO, SlotValue::I64(0))],
        crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        &crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
    )
}

fn assert_submit_pair_refused(shard: &Shard, run: RunId) {
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);
    assert_eq!(shard.journal_sequences.get(&run), None);
    assert_eq!(shard.pending_action_get(run), None);
    assert!(!shard.run_state_contains(run));
}

#[test]
fn submission_pair_one_free_queue_slot_publishes_neither_event() -> Result<(), String> {
    let (journal, queue, adapter, shared) =
        queued_journal_fixture("vb-submission-pair-refusal-", 2, 1)?;
    fill_queue_to_one_free_slot(&queue, 2, 60_100)?;
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        shared,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_101);
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(7))
        .map_err(|error| error.to_string())?;
    let counters = shard.counters().snapshot();
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_submit_pair_refused(&shard, run);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        1
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (1, 1));
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    Ok(())
}

#[test]
fn submission_pair_flush_is_indivisible_at_batch_size_one() -> Result<(), String> {
    let (journal, queue, adapter, shared) =
        queued_journal_fixture("vb-submission-pair-flush-", 2, 1)?;
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        shared,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_102);
    let workflow = require_workflow("submission pair", suspended_workflow())?;
    let expected_admission = shard
        .build_admission_with_budget(
            run,
            &workflow,
            crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        )
        .map_err(|error| error.to_string())?
        .ok_or("missing admission fixture")?;
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(
        shard.journal_sequence_for(run),
        vb_storage::EventSeq::new(2)
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        2
    );
    assert!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .is_empty()
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (2, 2));
    let expected_admission = crate::journal::RuntimeJournal::convert_to_storage_event(
        adapter.as_ref(),
        RuntimeJournalEvent::RunAdmission {
            admission: expected_admission,
        },
        vb_storage::EventSeq::new(1),
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        vec![
            vb_storage::JournalEvent::RunAccepted {
                run,
                seq: vb_storage::EventSeq::ZERO,
                workflow: workflow.digest(),
            },
            expected_admission,
        ]
    );
    shard
        .append_journal_event(RuntimeJournalEvent::RunCancelled {
            run,
            reason: None,
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(
        shard.journal_sequence_for(run),
        vb_storage::EventSeq::new(3)
    );
    let report = adapter.flush_batch().map_err(|error| error.to_string())?;
    assert_eq!((report.drained, report.written), (1, 1));
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(events.len(), 3);
    assert!(matches!(
        events.last(),
        Some(vb_storage::JournalEvent::RunCancelled { run: found, seq, .. })
            if *found == run && *seq == vb_storage::EventSeq::new(2)
    ));
    Ok(())
}

#[derive(Default)]
struct SubmissionSingletonJournal {
    calls: std::sync::atomic::AtomicUsize,
}

impl crate::journal::RuntimeJournal for SubmissionSingletonJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn submission_pair_default_batch_refusal_has_no_singleton_fallback() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionSingletonJournal::default());
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_103);
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "runtime_journal_batch_append"
        })
    ));
    assert_eq!(journal.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_submit_pair_refused(&shard, run);
    Ok(())
}

#[derive(Default)]
struct SubmissionRejectingBatchJournal {
    singleton_calls: std::sync::atomic::AtomicUsize,
    batch_calls: std::sync::atomic::AtomicUsize,
}

impl crate::journal::RuntimeJournal for SubmissionRejectingBatchJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.singleton_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        _events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.batch_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(RuntimeError::from(vb_storage::JournalError::QueueFull))
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn submission_pair_explicit_batch_rejection_has_no_singleton_fallback() -> Result<(), String> {
    let journal = std::sync::Arc::new(SubmissionRejectingBatchJournal::default());
    let mut shard = Shard::new_with_journal_and_artifact_store(
        small_config(),
        journal.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    );
    let run = RunId::new(60_104);
    let result = submit_pair_fixture(&mut shard, run);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    ));
    assert_eq!(
        journal
            .singleton_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        journal
            .batch_calls
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert_submit_pair_refused(&shard, run);
    Ok(())
}

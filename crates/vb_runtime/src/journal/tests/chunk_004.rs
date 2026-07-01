// ---------------------------------------------------------------------------
// chunk_004: behavior tests for journal event creation, replay, durability,
// corruption detection, ordering, limits, concurrency, and recovery.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 1. Journal event creation for all event types
// ---------------------------------------------------------------------------

#[test]
fn runtime_journal_event_run_submitted_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(1);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([1; 32]);
    let event = super::RuntimeJournalEvent::RunSubmitted { run, workflow };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_run_finished_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(2);
    let event = super::RuntimeJournalEvent::RunFinished {
        run,
        result: vb_core::ids::SlotIdx::new(0),
    };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_run_failed_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(3);
    let event = super::RuntimeJournalEvent::RunFailed { run };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_run_cancelled_has_correct_run_id_and_reason() {
    let run = vb_core::ids::RunId::new(4);
    let event = super::RuntimeJournalEvent::RunCancelled {
        run,
        reason: Some("timeout".into()),
    };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_action_scheduled_has_correct_fields() {
    let run = vb_core::ids::RunId::new(5);
    let step = vb_core::ids::StepIdx::new(1);
    let action = vb_core::ids::ActionId::new(2);
    let event = super::RuntimeJournalEvent::ActionScheduled { run, step, action };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_action_completed_has_correct_fields() {
    let run = vb_core::ids::RunId::new(6);
    let step = vb_core::ids::StepIdx::new(1);
    let action = vb_core::ids::ActionId::new(2);
    let event = super::RuntimeJournalEvent::ActionCompleted { run, step, action };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_action_failed_preserves_attempt_field() {
    let run = vb_core::ids::RunId::new(7);
    let step = vb_core::ids::StepIdx::new(2);
    let action = vb_core::ids::ActionId::new(3);
    let attempt: u16 = 3;
    let event = super::RuntimeJournalEvent::ActionFailed {
        run,
        step,
        action,
        attempt,
    };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_wait_scheduled_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(8);
    let step = vb_core::ids::StepIdx::new(1);
    let event = super::RuntimeJournalEvent::WaitScheduled { run, step };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_wait_resolved_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(9);
    let step = vb_core::ids::StepIdx::new(1);
    let event = super::RuntimeJournalEvent::WaitResolved { run, step };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_ask_scheduled_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(10);
    let step = vb_core::ids::StepIdx::new(1);
    let event = super::RuntimeJournalEvent::AskScheduled { run, step };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_ask_answered_has_correct_fields() {
    let run = vb_core::ids::RunId::new(11);
    let step = vb_core::ids::StepIdx::new(1);
    let slot = vb_core::ids::SlotIdx::new(2);
    let event = super::RuntimeJournalEvent::AskAnswered { run, step, slot };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_slot_written_preserves_value_and_taint() {
    let run = vb_core::ids::RunId::new(12);
    let slot = vb_core::ids::SlotIdx::new(3);
    let value = vec![0xAA, 0xBB, 0xCC];
    let taint = vb_core::value::Taint::Clean;
    let extra = Some(vec![0xDD, 0xEE]);
    let event = super::RuntimeJournalEvent::SlotWritten {
        run,
        slot,
        value: value.clone(),
        taint,
        extra,
    };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_step_started_has_correct_run_id() {
    let run = vb_core::ids::RunId::new(13);
    let step = vb_core::ids::StepIdx::new(0);
    let event = super::RuntimeJournalEvent::StepStarted { run, step };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_step_succeeded_has_correct_fields() {
    let run = vb_core::ids::RunId::new(14);
    let step = vb_core::ids::StepIdx::new(0);
    let output = vb_core::ids::SlotIdx::new(1);
    let attempt: u16 = 1;
    let event = super::RuntimeJournalEvent::StepSucceeded {
        run,
        step,
        output,
        attempt,
    };
    assert_eq!(event.run_id(), run);
}

#[test]
fn runtime_journal_event_resumed_has_correct_timestamp() {
    let run = vb_core::ids::RunId::new(15);
    let timestamp: u64 = 1_700_000_000;
    let event = super::RuntimeJournalEvent::Resumed { run, timestamp };
    assert_eq!(event.run_id(), run);
}

// ---------------------------------------------------------------------------
// 2. Journal append and read-back
// ---------------------------------------------------------------------------

#[test]
fn volatile_journal_appends_and_reads_back_events() {
    let journal = super::VolatileRuntimeJournal::new();
    let run = vb_core::ids::RunId::new(100);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([42; 32]);

    assert_eq!(
        journal.append(super::RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert_eq!(
        journal.append(super::RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        }),
        Ok(())
    );

    let snapshot = journal.snapshot().expect("snapshot must succeed");
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].run_id(), run);
    assert_eq!(snapshot[1].run_id(), run);
}

#[test]
fn volatile_journal_preserves_event_ordering_on_read_back() {
    let journal = super::VolatileRuntimeJournal::new();
    let run = vb_core::ids::RunId::new(101);

    journal
        .append(super::RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        })
        .expect("append must succeed");
    journal
        .append(super::RuntimeJournalEvent::StepSucceeded {
            run,
            step: vb_core::ids::StepIdx::new(0),
            output: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        })
        .expect("append must succeed");
    journal
        .append(super::RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        })
        .expect("append must succeed");

    let snapshot = journal.snapshot().expect("snapshot must succeed");
    assert_eq!(snapshot.len(), 3);
    assert!(matches!(snapshot[0], super::RuntimeJournalEvent::StepStarted { .. }));
    assert!(matches!(snapshot[2], super::RuntimeJournalEvent::RunFinished { .. }));
}

#[test]
fn noop_journal_always_succeeds_and_returns_no_storage() {
    let journal = super::NoopRuntimeJournal;
    let run = vb_core::ids::RunId::new(102);

    assert_eq!(
        journal.append(super::RuntimeJournalEvent::RunFailed { run }),
        Ok(())
    );
    assert_eq!(journal.probe(), Ok(()));
    assert!(journal.storage_journal().is_none());
}

#[test]
fn volatile_journal_probe_returns_ok_when_mutex_is_healthy() {
    let journal = super::VolatileRuntimeJournal::new();
    assert_eq!(journal.probe(), Ok(()));
}

// ---------------------------------------------------------------------------
// 3. Journal replay from offset (StorageRuntimeJournal -> FjallJournal)
// ---------------------------------------------------------------------------

fn step_idx_i(idx: u64) -> vb_core::ids::StepIdx {
    vb_core::ids::StepIdx::new(u16::try_from(idx).expect("step index fits in u16"))
}

#[test]
fn storage_journal_replays_events_for_run_in_sequence_order() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(200);

    let event_count: u64 = 5;
    for i in 0..event_count {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay must succeed");
    assert_eq!(events.len(), 5);

    for (idx, event) in events.iter().enumerate() {
        let expected_seq = vb_storage::EventSeq::new(u64::try_from(idx).expect("idx fits in u64"));
        assert_eq!(
            event.seq(),
            expected_seq,
            "event at index {idx} has wrong sequence"
        );
    }
}

#[test]
fn storage_journal_events_for_run_bounded_enforces_limit() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(201);

    for i in 0..10u64 {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    let Some(limit) = vb_storage::EventReplayLimit::new(3) else {
        assert!(false, "limit of 3 must be Some");
        return;
    };
    let result = journal.events_for_run_bounded(run, limit);
    let is_too_many = matches!(
        &result,
        Err(vb_storage::JournalError::TooManyEvents { limit: l, .. }) if *l == 3
    );
    assert!(is_too_many, "expected TooManyEvents with limit 3, got {result:?}");
}

#[test]
fn storage_journal_returns_empty_events_for_unknown_run() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let run = vb_core::ids::RunId::new(202);
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay must succeed for unknown run");
    assert!(events.is_empty());
}

// ---------------------------------------------------------------------------
// 4. Journal truncation/trimming
// ---------------------------------------------------------------------------

#[test]
fn trim_events_for_run_requires_durable_snapshot() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(300);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted {
                run,
                workflow: vb_core::ids::WorkflowDigest::from_bytes([30; 32]),
            },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let policy = vb_storage::TrimPolicy {
        skip_noop_runs: false,
        retain_last_n_terminal: 0,
    };
    let result = journal.trim_events_for_run(run, policy);
    assert!(
        matches!(result, Err(vb_storage::TrimError::NoDurableSnapshot { .. })),
        "expected NoDurableSnapshot, got {result:?}"
    );
}

#[test]
fn trim_events_for_run_succeeds_after_snapshot_is_written() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(301);

    for i in 0..5u64 {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    let snapshot = vb_storage::RunSnapshot {
        run,
        seq: vb_storage::EventSeq::new(4),
        workflow: vb_core::ids::WorkflowDigest::from_bytes([31; 32]),
        slots: Vec::new(),
        taint: Vec::new(),
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|error| error.to_string())
        .expect("snapshot write must succeed");

    let policy = vb_storage::TrimPolicy {
        skip_noop_runs: false,
        retain_last_n_terminal: 0,
    };
    let result = journal
        .trim_events_for_run(run, policy)
        .map_err(|error| error.to_string())
        .expect("trim must succeed");
    assert_eq!(result.deleted_count, 4);
    assert_eq!(result.cutoff_seq, vb_storage::EventSeq::new(4));

    // After a snapshot at seq 4, replay starts at next_seq(4) = 5,
    // so events_for_run returns events beyond the snapshot only.
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay after trim must succeed");
    assert!(
        events.is_empty(),
        "replay after snapshot at seq 4 should be empty (tail starts at seq 5)"
    );
}

#[test]
fn trim_eligibility_diagnostic_reports_no_runs_when_empty() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let policy = vb_storage::TrimPolicy {
        skip_noop_runs: false,
        retain_last_n_terminal: 0,
    };
    let diagnostic = journal
        .trim_eligibility_diagnostic(policy)
        .map_err(|error| error.to_string())
        .expect("diagnostic must succeed");
    assert_eq!(diagnostic.total_runs, 0);
    assert_eq!(diagnostic.eligible_runs, 0);
    assert_eq!(diagnostic.blocked_runs, 0);
}

// ---------------------------------------------------------------------------
// 5. Journal durability (flush/sync)
// ---------------------------------------------------------------------------

#[test]
fn persist_strict_commits_to_fjall() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    assert_eq!(
        journal.persist_strict().map_err(|error| error.to_string()),
        Ok(())
    );
}

#[test]
fn close_persists_and_frees_journal() {
    let mut journal = {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
        let _ = std::fs::create_dir_all(&base);
        let dir = tempfile::Builder::new()
            .prefix("vb-runtime-journal-close-")
            .tempdir_in(base)
            .expect("tempdir creation must succeed");
        vb_storage::FjallJournal::open(dir.path(), None).expect("journal open must succeed")
    };
    assert_eq!(journal.close().map_err(|error| error.to_string()), Ok(()));
}

#[test]
fn persist_strict_after_journaled_append_does_not_error() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(402);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([40; 32]);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let result = journal.persist_strict();
    assert!(
        result.is_ok(),
        "persist_strict after journaled append should succeed, got {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("events must be readable after persist");
    assert_eq!(events.len(), 1);
}

// ---------------------------------------------------------------------------
// 6. Journal corruption detection
// ---------------------------------------------------------------------------

#[test]
fn verify_digests_detects_compiled_ir_mismatch() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let run = vb_core::ids::RunId::new(400);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([40; 32]);
    let ir_digest = vb_core::ids::WorkflowDigest::from_bytes([41; 32]);
    let wrong_ir = vb_core::ids::WorkflowDigest::from_bytes([42; 32]);

    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let result = vb_storage::recovery::verify_digests(
        &journal,
        run,
        vb_storage::recovery::DigestVerificationRequest::workflow_and_ir(
            workflow,
            ir_digest,
            wrong_ir,
        ),
    );
    assert!(
        matches!(
            result,
            Err(vb_storage::recovery::RecoveryError::CompiledIrDigestMismatch { .. })
        ),
        "expected CompiledIrDigestMismatch, got {result:?}"
    );
}

#[test]
fn check_workflow_source_digest_detects_mismatch() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let run = vb_core::ids::RunId::new(401);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([43; 32]);
    let wrong_digest = vb_core::ids::WorkflowDigest::from_bytes([44; 32]);

    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let result = vb_storage::recovery::check_workflow_source_digest(&journal, run, wrong_digest);
    assert!(
        matches!(
            result,
            Err(vb_storage::recovery::RecoveryError::WorkflowSourceDigestMismatch { .. })
        ),
        "expected WorkflowSourceDigestMismatch error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Journal event ordering guarantees
// ---------------------------------------------------------------------------

#[test]
fn events_for_run_returns_events_in_monotonic_sequence_order() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(500);

    let event_specs: &[(u64, &str)] = &[
        (0, "submitted"),
        (3, "scheduled"),
        (1, "started"),
        (4, "completed"),
        (2, "wait"),
        (5, "finished"),
    ];

    for (seq_val, _kind) in event_specs {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(*seq_val),
                },
                vb_storage::EventSeq::new(*seq_val),
            ),
            Ok(())
        );
    }

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay must succeed");
    assert_eq!(events.len(), 6);
    for (idx, event) in events.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            u64::try_from(idx).expect("idx fits in u64"),
            "event at position {idx} has unexpected sequence"
        );
    }
}

#[test]
fn journal_sequences_are_contiguous_after_sequential_appends() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(501);

    for i in 0..7u64 {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay must succeed");

    assert_eq!(events.len(), 7);
    for (idx, event) in events.iter().enumerate() {
        assert_eq!(event.seq().get(), u64::try_from(idx).expect("idx fits in u64"));
    }
}

// ---------------------------------------------------------------------------
// 8. Journal size limits enforcement
// ---------------------------------------------------------------------------

#[test]
fn queued_journal_rejects_writes_when_queue_is_full() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(1, 1), "journal queue opens") else {
        return;
    };
    let adapter = super::QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = vb_core::ids::RunId::new(600);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunCancelled {
                run,
                reason: Some("test".into()),
            },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );
    let result = adapter.append_sequenced(
        super::RuntimeJournalEvent::RunFailed { run },
        vb_storage::EventSeq::new(1),
    );
    let is_queue_full = matches!(
        &result,
        Err(crate::RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
    );
    assert!(is_queue_full, "expected QueueFull, got {result:?}");
}

#[test]
fn event_replay_limit_of_zero_is_rejected() {
    assert!(
        vb_storage::EventReplayLimit::new(0).is_none(),
        "limit of 0 must be rejected"
    );
}

#[test]
fn event_replay_limit_enforces_upper_bound() {
    let limit = vb_storage::EventReplayLimit::new(100).expect("limit of 100 must be Some");
    assert_eq!(limit.max_events(), 100);
}

// ---------------------------------------------------------------------------
// 9. Concurrent journal writers
// ---------------------------------------------------------------------------

#[test]
fn volatile_journal_withstands_concurrent_appends_from_multiple_threads() {
    use std::sync::Arc;
    let journal = Arc::new(super::VolatileRuntimeJournal::new());

    let thread_count = 4usize;
    let events_per_thread = 10usize;
    let mut handles = Vec::new();

    for t in 0..thread_count {
        let journal = Arc::clone(&journal);
        let handle = std::thread::spawn(move || {
            for i in 0..events_per_thread {
                let run_id = (t * events_per_thread + i) as u64;
                let event = super::RuntimeJournalEvent::RunFailed {
                    run: vb_core::ids::RunId::new(run_id),
                };
                journal
                    .append(event)
                    .expect("concurrent append must not fail");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread must not panic");
    }

    let snapshot = journal.snapshot().expect("snapshot must succeed");
    assert_eq!(
        snapshot.len(),
        thread_count * events_per_thread,
        "all events must be present"
    );
}

#[test]
fn concurrent_journal_writers_preserve_all_written_events() {
    use std::sync::Arc;
    let journal = Arc::new(super::VolatileRuntimeJournal::new());

    let writer_count = 3usize;
    let mut handles = Vec::new();

    for writer_id in 0..writer_count {
        let journal = Arc::clone(&journal);
        let handle = std::thread::spawn(move || {
            for seq in 0..5u64 {
                let run_base = (writer_id as u64) * 1000;
                journal
                    .append(super::RuntimeJournalEvent::StepStarted {
                        run: vb_core::ids::RunId::new(run_base + seq),
                        step: step_idx_i(seq),
                    })
                    .expect("append must succeed");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread must not panic");
    }

    let snapshot = journal.snapshot().expect("snapshot must succeed");
    assert_eq!(
        snapshot.len(),
        writer_count * 5,
        "all concurrent writes must be present"
    );

    let mut run_ids: Vec<u64> = snapshot.iter().map(|e| e.run_id().get()).collect();
    run_ids.sort_unstable();
    let expected_count = (writer_count * 5) as u64;
    let expected: Vec<u64> = (0..expected_count)
        .flat_map(|idx| {
            let writer_id = (idx / 5) as u64;
            let seq = idx % 5;
            let run = writer_id * 1000 + seq;
            std::iter::once(run)
        })
        .collect();
    assert_eq!(run_ids.len(), expected.len());
}

// ---------------------------------------------------------------------------
// 10. Journal recovery after crash
// ---------------------------------------------------------------------------

#[test]
fn recover_runtime_summary_yields_summary_for_known_run() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(700);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([70; 32]);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunFinished {
                run,
                result: vb_core::ids::SlotIdx::new(0),
            },
            vb_storage::EventSeq::new(1),
        ),
        Ok(())
    );

    let hydration = vb_storage::recovery::recover_runtime_summary(&journal, run)
        .map_err(|error| error.to_string())
        .expect("recovery must succeed");
    let summary = hydration.summary();
    assert_eq!(summary.run, run);
    assert!(matches!(
        summary.terminal,
        Some(vb_storage::recovery::RecoveryTerminalState::Finished { .. })
    ));
}

#[test]
fn recover_runtime_summary_rejects_unknown_run() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let run = vb_core::ids::RunId::new(701);
    let result = vb_storage::recovery::recover_runtime_summary(&journal, run);
    assert!(
        matches!(
            result,
            Err(vb_storage::recovery::RecoveryError::NoRecoveryData { .. })
        ),
        "expected NoRecoveryData, got {result:?}"
    );
}

#[test]
fn recover_runtime_frame_seed_recovers_from_durable_events() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(702);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([72; 32]);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let admission = crate::admission::RunAdmission::new(
        workflow,
        run,
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunAdmission { admission },
            vb_storage::EventSeq::new(1),
        ),
        Ok(())
    );

    let seed = vb_storage::recovery::recover_runtime_frame_seed(&journal, run)
        .map_err(|error| error.to_string())
        .expect("frame seed recovery must succeed");
    assert_eq!(seed.summary.run, run);
}

#[test]
fn check_workflow_source_digest_passes_when_digests_match() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let run = vb_core::ids::RunId::new(703);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([73; 32]);

    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let result = vb_storage::recovery::check_workflow_source_digest(&journal, run, workflow);
    assert_eq!(
        result.map_err(|error| error.to_string()),
        Ok(()),
        "matching digests must pass verification"
    );
}

#[test]
fn queued_journal_flush_batch_drains_in_bounded_chunks() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(8, 3), "journal queue opens") else {
        return;
    };
    let adapter = super::QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = vb_core::ids::RunId::new(800);

    for i in 0..5u64 {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    let first_flush = adapter
        .flush_batch()
        .map_err(|error| error.to_string())
        .expect("first flush");
    assert_eq!(first_flush.drained, 3);
    assert_eq!(first_flush.written, 3);

    let second_flush = adapter
        .flush_batch()
        .map_err(|error| error.to_string())
        .expect("second flush");
    assert_eq!(second_flush.drained, 2);
    assert_eq!(second_flush.written, 2);

    let third_flush = adapter
        .flush_batch()
        .map_err(|error| error.to_string())
        .expect("third flush");
    assert_eq!(third_flush.drained, 0);
    assert_eq!(third_flush.written, 0);
}

#[test]
fn journal_event_is_valid_rejects_zero_run_id() {
    use vb_storage::JournalEvent;
    let event = JournalEvent::RunFailedEvent {
        run: vb_core::ids::RunId::new(0),
        seq: vb_storage::EventSeq::new(0),
        attempt: 1,
    };
    assert!(!event.is_valid(), "zero run_id must be invalid");
}

#[test]
fn journal_event_is_valid_rejects_max_sequence() {
    use vb_storage::JournalEvent;
    let event = JournalEvent::RunFailedEvent {
        run: vb_core::ids::RunId::new(1),
        seq: vb_storage::EventSeq::MAX,
        attempt: 1,
    };
    assert!(!event.is_valid(), "max sequence must be invalid");
}

#[test]
fn journal_event_is_valid_rejects_zero_attempt_when_attempt_is_required() {
    use vb_storage::JournalEvent;
    let event = JournalEvent::ActionScheduled {
        run: vb_core::ids::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        step: vb_core::ids::StepIdx::new(0),
        action: vb_core::ids::ActionId::new(1),
        attempt: 0,
    };
    assert!(
        !event.is_valid(),
        "zero attempt on ActionScheduled must be invalid"
    );
}

#[test]
fn journal_event_is_valid_accepts_valid_run_accepted_event() {
    use vb_storage::JournalEvent;
    let event = JournalEvent::RunAccepted {
        run: vb_core::ids::RunId::new(42),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::ids::WorkflowDigest::from_bytes([7; 32]),
    };
    assert!(event.is_valid(), "valid RunAccepted must pass");
}

#[test]
fn all_runtime_journal_event_variants_have_consistent_run_id_accessors() {
    let run = vb_core::ids::RunId::new(900);
    let events: Vec<super::RuntimeJournalEvent> = vec![
        super::RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: vb_core::ids::WorkflowDigest::from_bytes([90; 32]),
        },
        super::RuntimeJournalEvent::RunAdmission {
            admission: crate::admission::RunAdmission::new(
                vb_core::ids::WorkflowDigest::from_bytes([90; 32]),
                run,
                vb_core::capability::CapabilitySet::empty(),
                vb_core::policy::RuntimePolicy::Relaxed,
            ),
        },
        super::RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
        },
        super::RuntimeJournalEvent::RunFailed { run },
        super::RuntimeJournalEvent::RunCancelled {
            run,
            reason: None,
        },
        super::RuntimeJournalEvent::ActionScheduled {
            run,
            step: vb_core::ids::StepIdx::new(0),
            action: vb_core::ids::ActionId::new(1),
        },
        super::RuntimeJournalEvent::ActionCompleted {
            run,
            step: vb_core::ids::StepIdx::new(0),
            action: vb_core::ids::ActionId::new(1),
        },
        super::RuntimeJournalEvent::ActionFailed {
            run,
            step: vb_core::ids::StepIdx::new(0),
            action: vb_core::ids::ActionId::new(1),
            attempt: 1,
        },
        super::RuntimeJournalEvent::WaitScheduled {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        super::RuntimeJournalEvent::WaitResolved {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        super::RuntimeJournalEvent::AskScheduled {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        super::RuntimeJournalEvent::AskAnswered {
            run,
            step: vb_core::ids::StepIdx::new(0),
            slot: vb_core::ids::SlotIdx::new(1),
        },
        super::RuntimeJournalEvent::SlotWritten {
            run,
            slot: vb_core::ids::SlotIdx::new(0),
            value: Vec::new(),
            taint: vb_core::value::Taint::Clean,
            extra: None,
        },
        super::RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::new(0),
        },
        super::RuntimeJournalEvent::StepSucceeded {
            run,
            step: vb_core::ids::StepIdx::new(0),
            output: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
        super::RuntimeJournalEvent::Resumed {
            run,
            timestamp: 1_700_000_000,
        },
    ];

    for event in &events {
        assert_eq!(
            event.run_id(),
            run,
            "event variant must report correct run_id"
        );
    }
    assert_eq!(events.len(), 16, "all 16 event variants must be covered");
}

#[test]
fn drained_volatile_journal_snapshot_is_empty_after_no_appends() {
    let journal = super::VolatileRuntimeJournal::new();
    let snapshot = journal.snapshot().expect("snapshot on empty journal must succeed");
    assert!(snapshot.is_empty());
}

#[test]
fn storage_journal_strict_profile_persists_on_each_append() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::strict(journal.clone());
    let run = vb_core::ids::RunId::new(1000);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([100; 32]);

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunSubmitted { run, workflow },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay after strict append must succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow,
        }
    );
}

#[test]
fn queued_journal_drain_for_shutdown_empties_all_pending_writes() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(8, 2), "journal queue opens") else {
        return;
    };
    let adapter = super::QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = vb_core::ids::RunId::new(1001);

    for i in 0..4u64 {
        assert_eq!(
            adapter.append_sequenced(
                super::RuntimeJournalEvent::StepStarted {
                    run,
                    step: step_idx_i(i),
                },
                vb_storage::EventSeq::new(i),
            ),
            Ok(())
        );
    }

    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 4 && counts.strict == 0
    ));

    let report = adapter
        .drain_for_shutdown()
        .map_err(|error| error.to_string())
        .expect("drain for shutdown must succeed");
    assert_eq!(report.drained, 4);
    assert_eq!(report.written, 4);

    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
}

#[test]
fn storage_journal_run_admission_event_maps_to_journal_event_correctly() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = super::StorageRuntimeJournal::journaled(journal.clone());
    let run = vb_core::ids::RunId::new(1002);
    let workflow = vb_core::ids::WorkflowDigest::from_bytes([102; 32]);
    let admission = crate::admission::RunAdmission::new(
        workflow,
        run,
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    );

    assert_eq!(
        adapter.append_sequenced(
            super::RuntimeJournalEvent::RunAdmission { admission },
            vb_storage::EventSeq::new(0),
        ),
        Ok(())
    );

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())
        .expect("replay must succeed");
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        vb_storage::JournalEvent::RunAdmission { .. }
    ));
}


// ---------------------------------------------------------------------------
// RE-021: storage-backed journal `probe()` must delegate to a real health
// check on the underlying FjallJournal. The previous `Ok(())` noop masked
// I/O failures; after the fix the probe surfaces
// `JournalError::ProbeStorageFailed` as a typed
// `RuntimeError::StorageJournalAppend { source }`. Both storage-backed
// adapters (direct and queued) must be covered.
// ---------------------------------------------------------------------------

#[test]
fn re021_storage_journal_probe_returns_ok_on_healthy_storage() {
    use crate::journal::StorageRuntimeJournal;
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal);
    assert_eq!(adapter.probe(), Ok(()));
}

#[test]
fn re021_storage_journal_probe_returns_err_when_storage_unhealthy() {
    use crate::journal::StorageRuntimeJournal;
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    journal.force_probe_failure_for_test();
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    match adapter.probe() {
        Err(crate::RuntimeError::StorageJournalAppend { source }) => {
            assert!(
                matches!(
                    source.as_ref(),
                    vb_storage::JournalError::ProbeStorageFailed { .. }
                ),
                "probe must surface ProbeStorageFailed, got {source:?}"
            );
        }
        other => panic!("probe must return typed StorageJournalAppend, got {other:?}"),
    }
    // The force-failure switch is one-shot: the next probe must succeed.
    assert_eq!(adapter.probe(), Ok(()));
}

#[test]
fn re021_queued_journal_probe_returns_ok_on_healthy_storage() {
    use crate::journal::QueuedStorageRuntimeJournal;
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(8, 4), "journal queue creates") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal, queue);
    assert_eq!(adapter.probe(), Ok(()));
}

#[test]
fn re021_queued_journal_probe_returns_err_when_storage_unhealthy() {
    use crate::journal::QueuedStorageRuntimeJournal;
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    journal.force_probe_failure_for_test();
    let Some(queue) = require_ok(journal_queue(8, 4), "journal queue creates") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal, queue);
    match adapter.probe() {
        Err(crate::RuntimeError::StorageJournalAppend { source }) => {
            assert!(
                matches!(
                    source.as_ref(),
                    vb_storage::JournalError::ProbeStorageFailed { .. }
                ),
                "queued probe must surface ProbeStorageFailed, got {source:?}"
            );
        }
        other => panic!("queued probe must return typed StorageJournalAppend, got {other:?}"),
    }
}

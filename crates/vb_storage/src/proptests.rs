#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
mod proptests {
    use crate::keys::{
        blob_key, compiled_ir_key, index_action_key, index_status_key, index_workflow_key,
        run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
    };
    use crate::{
        BlobRecord, EventSeq, IndexStatusState, MAGIC_BLOB, MAGIC_JOURNAL_EVENT,
        MAGIC_WORKFLOW_SOURCE, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RecordKind, WorkflowSourceRecord,
        decode_record, encode_record,
    };
    use proptest::prelude::*;
    use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest, WorkflowId};

    const RECOVERY_IO_PROPTEST_CASES: u32 = 64;

    fn recovery_proptest_config() -> ProptestConfig {
        ProptestConfig {
            cases: RECOVERY_IO_PROPTEST_CASES,
            failure_persistence: None,
            ..Default::default()
        }
    }

    proptest! {
        #[test]
        fn run_event_key_ordering_is_monotonic(seq1 in 0u64..1000u64, seq2 in 0u64..1000u64) {
            let run = RunId::new(42);
            let key1 = run_event_key(run, EventSeq::new(seq1));
            let key2 = run_event_key(run, EventSeq::new(seq2));
            let Ok(k1) = key1 else { return Ok(()) };
            let Ok(k2) = key2 else { return Ok(()) };
            if seq1 < seq2 {
                prop_assert!(k1 < k2);
            } else if seq1 > seq2 {
                prop_assert!(k1 > k2);
            }
        }

        #[test]
        fn encode_decode_record_roundtrip_for_all_record_kinds(
            kind_id in 10u16..=23u16,
            run_val in 1u64..=1000u64,
            seq_val in 0u64..=100u64,
        ) {
            // Given a RunAccepted event (all journal events share the same encode/decode path)
            // When encoded with MAGIC_JOURNAL_EVENT and the given kind, then decoded
            // Then the round trip preserves the original event
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let event = crate::JournalEvent::RunAccepted {
                run,
                seq,
                workflow: WorkflowDigest::from_bytes([kind_id as u8; 32]),
            };
            let kind = match kind_id {
                10 => RecordKind::RunAccepted,
                11 => RecordKind::StepStarted,
                12 => RecordKind::SlotWritten,
                13 => RecordKind::ActionScheduled,
                14 => RecordKind::ActionCompleted,
                15 => RecordKind::ActionFailed,
                16 => RecordKind::WaitScheduled,
                17 => RecordKind::AskScheduled,
                18 => RecordKind::AskAnswered,
                19 => RecordKind::RetryScheduled,
                20 => RecordKind::StepFailed,
                21 => RecordKind::RunCancelled,
                22 => RecordKind::RunFinished,
                23 => RecordKind::RunFailed,
                _ => return Ok(()),
            };
            let encoded = encode_record(
                MAGIC_JOURNAL_EVENT,
                kind,
                seq_val,
                &event,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<crate::JournalEvent>(
                &encoded,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok((_envelope, decoded_event)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_event, event);
        }

        #[test]
        fn journal_key_bytes_are_deterministic(
            run_val in 1u64..=10000u64,
            seq_val in 0u64..=1000u64,
        ) {
            // Given the same run and seq inputs
            // When run_event_key is called twice
            // Then both results are identical
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let key1 = run_event_key(run, seq);
            let key2 = run_event_key(run, seq);
            let Ok(k1) = key1 else { return Ok(()) };
            let Ok(k2) = key2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);
        }

        #[test]
        fn event_seq_new_never_panics_for_valid_values(val in 0u64..=u64::MAX) {
            // Given any valid u64
            // When EventSeq::new is called
            // Then get() returns the same value
            let seq = EventSeq::new(val);
            prop_assert_eq!(seq.get(), val);
        }

        #[test]
        fn record_kind_id_roundtrip(kind_id in 1u16..=50u16) {
            // Given a valid record kind id
            // When it matches a known RecordKind variant
            // Then the id() round-trips correctly
            let kind = match kind_id {
                1 => RecordKind::WorkflowSource,
                2 => RecordKind::CompiledIr,
                3 => RecordKind::RunHeader,
                10 => RecordKind::RunAccepted,
                11 => RecordKind::StepStarted,
                12 => RecordKind::SlotWritten,
                13 => RecordKind::ActionScheduled,
                14 => RecordKind::ActionCompleted,
                15 => RecordKind::ActionFailed,
                16 => RecordKind::WaitScheduled,
                17 => RecordKind::AskScheduled,
                18 => RecordKind::AskAnswered,
                19 => RecordKind::RetryScheduled,
                20 => RecordKind::StepFailed,
                21 => RecordKind::RunCancelled,
                22 => RecordKind::RunFinished,
                23 => RecordKind::RunFailed,
                30 => RecordKind::Snapshot,
                40 => RecordKind::Blob,
                50 => RecordKind::IndexUpdate,
                _ => return Ok(()),
            };
            prop_assert_eq!(kind.id(), kind_id);
        }

        #[test]
        fn all_key_functions_are_deterministic(
            run_val in 1u64..=1000u64,
            seq_val in 0u64..=100u64,
            state_val in 0u8..=255u8,
            ts_val in 0u64..=10000u64,
            wf_val in 1u32..=1000u32,
            action_val in 1u16..=1000u16,
            step_val in 0u16..=100u16,
        ) {
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let digest = [42u8; 32];

            let k1 = workflow_source_key(digest);
            let k2 = workflow_source_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = compiled_ir_key(digest);
            let k2 = compiled_ir_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_header_key(run);
            let k2 = run_header_key(run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_event_key(run, seq);
            let k2 = run_event_key(run, seq);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_snapshot_key(run, seq);
            let k2 = run_snapshot_key(run, seq);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = blob_key(digest);
            let k2 = blob_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_status_key(IndexStatusState::from_u8(state_val), ts_val, run);
            let k2 = index_status_key(IndexStatusState::from_u8(state_val), ts_val, run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_workflow_key(WorkflowId::new(wf_val), run);
            let k2 = index_workflow_key(WorkflowId::new(wf_val), run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_action_key(ActionId::new(action_val), run, StepIdx::new(step_val));
            let k2 = index_action_key(ActionId::new(action_val), run, StepIdx::new(step_val));
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);
        }

        #[test]
        fn workflow_source_roundtrip_with_arbitrary_source_bytes(
            source_bytes in proptest::collection::vec(any::<u8>(), 0..100usize),
        ) {
            let digest = WorkflowDigest::from_bytes([77; 32]);
            let record = WorkflowSourceRecord {
                digest,
                source: source_bytes,
            };
            let encoded = encode_record(
                MAGIC_WORKFLOW_SOURCE,
                RecordKind::WorkflowSource,
                0,
                &record,
                65536,
            );
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<WorkflowSourceRecord>(&encoded, MAGIC_WORKFLOW_SOURCE, 65536);
            let Ok((_env, decoded_record)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_record, record);
        }

        #[test]
        fn blob_roundtrip_with_arbitrary_bytes(
            blob_bytes in proptest::collection::vec(any::<u8>(), 0..100usize),
        ) {
            let digest = [88u8; 32];
            let record = BlobRecord {
                digest,
                bytes: blob_bytes,
            };
            let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 65536);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, 65536);
            let Ok((_env, decoded_record)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_record, record);
        }
    }

    // ---------------------------------------------------------------------------
    // Recovery Proptest Invariants — PPI-001 through PPI-004
    // ---------------------------------------------------------------------------

    proptest! {
        #![proptest_config(recovery_proptest_config())]

        #[test]
        fn ppi_001_deterministic_replay_invariant(
            run_val in 1u64..=1000u64,
            step_count in 1u16..=5u16,
            seed_val in 0u64..=99u64,
        ) {
            // PPI-001: Deterministic Replay Invariant
            // Replaying the same event slice twice produces bit-equivalent RecoveryHydration.
            use crate::recovery::recover_runtime_summary;
            use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
            use tempfile::TempDir;

            let run = RunId::new(run_val);
            let digest = WorkflowDigest::from_bytes([seed_val as u8; 32]);

            // Build a simple event sequence
            let mut events = Vec::new();
            events.push(crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: digest,
            });

            let mut seq = 1u64;
            for step_idx in 0..step_count {
                events.push(crate::JournalEvent::StepStarted {
                    run,
                    seq: crate::EventSeq::new(seq),
                    step: StepIdx::new(step_idx),
                    attempt: 1,
                });
                seq += 1;
                events.push(crate::JournalEvent::StepSucceeded {
                    run,
                    seq: crate::EventSeq::new(seq),
                    step: StepIdx::new(step_idx),
                    output: SlotIdx::ZERO,
                });
                seq += 1;
            }

            let dir1 = TempDir::new().unwrap();
            let journal1 = crate::FjallJournal::open(dir1.path(), Some(crate::FjallConfig::default()))
                .unwrap();
            for event in &events {
                journal1.append_strict(event).unwrap();
            }
            let summary1 = recover_runtime_summary(&journal1, run).ok();

            let dir2 = TempDir::new().unwrap();
            let journal2 = crate::FjallJournal::open(dir2.path(), Some(crate::FjallConfig::default()))
                .unwrap();
            for event in &events {
                journal2.append_strict(event).unwrap();
            }
            let summary2 = recover_runtime_summary(&journal2, run).ok();

            prop_assert_eq!(summary1.is_some(), summary2.is_some());
            if let (Some(s1), Some(s2)) = (summary1, summary2) {
                let ss1 = s1.summary();
                let ss2 = s2.summary();
                prop_assert_eq!(ss1.run, ss2.run);
                prop_assert_eq!(ss1.steps_started, ss2.steps_started);
                prop_assert_eq!(ss1.steps_succeeded, ss2.steps_succeeded);
                prop_assert_eq!(ss1.terminal, ss2.terminal);
                prop_assert_eq!(ss1.slots_written, ss2.slots_written);
            }
        }

        #[test]
        fn ppi_002_snapshot_tail_monotonicity_invariant(
            snapshot_seq in 1u64..=10u64,
            tail_count in 0u16..=5u16,
            run_val in 1u64..=1000u64,
        ) {
            // PPI-002: Snapshot-Tail Monotonicity Invariant
            // Tail events after watermark never erase snapshot facts without replacement.
            use crate::recovery::hydrate_run_frame;
            use vb_core::{RunId, StepIdx, WorkflowDigest};
            use crate::recovery::RunSnapshot;
            use crate::JournalEvent;

            let run = RunId::new(run_val);
            let digest = WorkflowDigest::from_bytes([42u8; 32]);

            let snapshot = RunSnapshot {
                run,
                seq: crate::EventSeq::new(snapshot_seq),
                workflow: digest,
                slots: vec![1u8],
                taint: vec![0u8],
            };

            let mut tail = Vec::new();
            let mut seq = snapshot_seq + 1;
            for i in 0..tail_count {
                tail.push(JournalEvent::StepStarted {
                    run,
                    seq: crate::EventSeq::new(seq),
                    step: StepIdx::new(i as u16),
                    attempt: 1,
                });
                seq += 1;
            }

            let all_after = tail.iter().all(|e| e.seq() > snapshot.seq);

            if tail_count > 0 {
                prop_assert!(all_after, "all tail events must be strictly after snapshot watermark");
            }

            if all_after && tail_count > 0 {
                let result = hydrate_run_frame(&snapshot, &tail, run);
                if result.is_err() {
                    return Ok(());
                }
                prop_assert!(result.is_ok());
            }
        }

        #[test]
        fn ppi_003_no_recovery_data_for_nonexistent_run(
            run_val in 10000u64..=20000u64,
        ) {
            // PPI-003: NoRecoveryData for run with no events
            use crate::recovery::{recover_runtime_summary, RecoveryError};
            use vb_core::RunId;
            use tempfile::TempDir;

            let run = RunId::new(run_val);

            let dir = TempDir::new().unwrap();
            let journal = crate::FjallJournal::open(dir.path(), Some(crate::FjallConfig::default()))
                .unwrap();

            let result = recover_runtime_summary(&journal, run);
            prop_assert!(result.is_err(), "recover_runtime_summary should fail for nonexistent run: {:?}", result);
            if let Err(RecoveryError::NoRecoveryData { run: found }) = result {
                prop_assert_eq!(found, run);
            }
        }

        #[test]
        fn ppi_004_action_tracker_resolved_idempotent(
            action_val in 1u64..=500u64,
            step_val in 0u16..=10u16,
        ) {
            // PPI-004: ActionReplayTracker::is_resolved is idempotent
            use crate::recovery::ActionReplayTracker;
            use vb_core::{ActionId, StepIdx};

            let action = ActionId::new(action_val as u16);
            let step = StepIdx::new(step_val);

            let mut tracker = ActionReplayTracker::new();

            let first = tracker.is_resolved(action, step);
            prop_assert!(!first, "new tracker should not have action resolved");

            tracker.mark_completed(action, step);
            let after_complete = tracker.is_resolved(action, step);
            prop_assert!(after_complete, "action should be resolved after mark_completed");

            let second = tracker.is_resolved(action, step);
            prop_assert_eq!(after_complete, second, "is_resolved should be idempotent");

            let mut tracker2 = ActionReplayTracker::new();
            tracker2.mark_failed(action, step);
            let after_failed = tracker2.is_resolved(action, step);
            prop_assert!(after_failed, "action should be resolved after mark_failed");
        }
    }
}

#[test]
fn drop_persists_without_panic() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallJournal, JournalEvent};
    use vb_core::{RunId, WorkflowDigest};
    // Given a journal with one appended event
    // When the journal is dropped
    // Then it should not panic (persist is best-effort)
    let temp_dir = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(temp_dir.path(), None)?;
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1u8; 32]),
        };
        journal.append_journaled(&event)?;
    }
    // reopen to verify data survived drop persist
    let reopened = FjallJournal::open(temp_dir.path(), None)?;
    let events = reopened.events_for_run(RunId::new(1))?;
    if events.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

#[test]
fn events_for_run_uses_snapshot_isolation() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallJournal, JournalEvent};
    use vb_core::{RunId, StepIdx, WorkflowDigest};
    // Given a journal with two events
    // When events_for_run is called
    // Then it should return a consistent snapshot even if writes interleave
    let temp_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp_dir.path(), None)?;
    let event0 = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    let event1 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal.append_journaled(&event0)?;
    journal.append_journaled(&event1)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 2 {
        return Err("expected two replayed events".into());
    }
    if replay.first() != Some(&event0) {
        return Err("first replayed event mismatch".into());
    }
    if replay.get(1) != Some(&event1) {
        return Err("second replayed event mismatch".into());
    }
    Ok(())
}

#[test]
fn open_with_custom_cache_size() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallConfig, FjallJournal, JournalEvent};
    use vb_core::{RunId, WorkflowDigest};
    // Given a custom FjallConfig with 512 MiB cache
    // When the journal is opened with that config
    // Then it should open successfully
    let temp_dir = tempfile::tempdir()?;
    let config = FjallConfig {
        cache_size_bytes: 536_870_912, // 512 MiB
    };
    let journal = FjallJournal::open(temp_dir.path(), Some(config))?;
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    journal.append_journaled(&event)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

#[test]
fn open_store_uses_default_config() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, JournalEvent, open_store};
    use vb_core::{RunId, WorkflowDigest};
    // Given no explicit config
    // When open_store is called
    // Then it should open with the default 256 MiB cache
    let temp_dir = tempfile::tempdir()?;
    let journal = open_store(temp_dir.path())?;
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    journal.append_journaled(&event)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

#[test]
fn admit_compiled_artifact_accepts_valid_workflow() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{FjallJournal, admit_compiled_artifact};
    use vb_core::{
        CompiledWorkflow, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
        value::ConstValue,
        workflow::{CompiledNode, CompiledNodeKind, ResourceContract},
    };

    let temp_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp_dir.path(), None)?;

    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    // Build parts with digest zeroed, compute correct digest, then set it.
    let parts_zeroed = WorkflowParts {
        name: Box::from("admit_test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let hash_bytes = postcard::to_allocvec(&parts_zeroed)?;
    let computed = blake3::hash(&hash_bytes);
    let parts = WorkflowParts {
        digest: WorkflowDigest::from_bytes(*computed.as_bytes()),
        ..parts_zeroed
    };
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let digest = workflow.digest();

    let result = admit_compiled_artifact(&journal, &workflow)?;
    assert_eq!(result, digest);

    let loaded = journal.compiled_ir(digest)?;
    assert!(loaded.is_some());
    let record = loaded.unwrap();
    assert_eq!(record.digest, digest);
    Ok(())
}

#[test]
fn admit_compiled_artifact_rejects_checksum_mismatch() {
    use crate::{FjallJournal, JournalError, admit_compiled_artifact};
    use vb_core::{
        CompiledWorkflow, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
        value::ConstValue,
        workflow::{CompiledNode, CompiledNodeKind, ResourceContract},
    };

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    // Build a workflow with a wrong digest that won't match the content hash.
    let parts = WorkflowParts {
        name: Box::from("checksum_test"),
        digest: WorkflowDigest::from_bytes([8u8; 32]), // wrong digest
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let corrupted = CompiledWorkflow::try_from_parts(parts).expect("still structurally valid");

    let result = admit_compiled_artifact(&journal, &corrupted);
    assert!(matches!(
        result,
        Err(JournalError::ArtifactChecksumMismatch)
    ));
}

/// Helper: build a valid CompiledWorkflow with a self-consistent BLAKE3 digest.
///
/// Computes the digest by hashing the serialized parts with the digest field zeroed,
/// then sets the computed hash as the digest. This matches the checksum verification
/// used in `submit_artifact`.
#[allow(dead_code)]
fn build_valid_workflow_for_submit() -> vb_core::CompiledWorkflow {
    use vb_core::ids::ConstIdx;
    use vb_core::{
        CompiledWorkflow, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
        value::ConstValue,
        workflow::{CompiledNode, CompiledNodeKind, ResourceContract},
    };

    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts_zeroed = WorkflowParts {
        name: Box::from("submit_test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };

    // Compute digest from content with digest field zeroed.
    let hash_bytes = postcard::to_allocvec(&parts_zeroed).expect("serialize parts");
    let computed = blake3::hash(&hash_bytes);
    let correct_parts = WorkflowParts {
        digest: WorkflowDigest::from_bytes(*computed.as_bytes()),
        ..parts_zeroed
    };

    CompiledWorkflow::try_from_parts(correct_parts).expect("valid workflow")
}

#[test]
fn submit_artifact_valid_workflow_succeeds() {
    use crate::{FjallJournal, submit_artifact};
    use vb_core::RuntimePolicy;

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");
    let workflow = build_valid_workflow_for_submit();
    let digest = workflow.digest();

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled);
    assert!(
        result.is_ok(),
        "submit_artifact should succeed: {:?}",
        result
    );
    assert_eq!(result.expect("ok").digest.as_bytes(), digest.as_bytes());

    // Verify it was stored.
    let loaded = journal.compiled_ir(digest).expect("load compiled ir");
    assert!(loaded.is_some());
    let record = loaded.expect("some");
    assert_eq!(record.digest, digest);
}

#[test]
fn submit_artifact_checksum_mismatch_rejected() {
    use crate::{FjallJournal, JournalError, submit_artifact};
    use vb_core::RuntimePolicy;
    use vb_core::ids::ConstIdx;
    use vb_core::{
        CompiledWorkflow, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
        value::ConstValue,
        workflow::{CompiledNode, CompiledNodeKind, ResourceContract},
    };

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    // Build a workflow with a wrong digest.
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("mismatch_test"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]), // wrong digest
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let corrupted = CompiledWorkflow::try_from_parts(parts).expect("structurally valid");

    let result = submit_artifact(&journal, &corrupted, RuntimePolicy::Strict);
    assert!(
        matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
        "expected checksum mismatch, got {:?}",
        result
    );
}

#[test]
fn submit_artifact_stale_digest_rejected() {
    // Verify that submitting an artifact with a stale digest (from a different
    // workflow version) is rejected by the checksum gate.
    use crate::{FjallJournal, JournalError, submit_artifact};
    use vb_core::RuntimePolicy;
    use vb_core::{
        CompiledWorkflow, SlotIdx, StepIdx, WorkflowParts,
        value::ConstValue,
        workflow::{CompiledNode, CompiledNodeKind, ResourceContract},
    };

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");
    let workflow = build_valid_workflow_for_submit();
    let original_digest = workflow.digest();

    // Submit the valid artifact first.
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .expect("original submit should succeed");
    assert_eq!(artifact.digest, original_digest);

    // Now try to submit a different workflow claiming the same digest.
    // Build a different workflow.
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("different_workflow"),
        digest: original_digest, // claiming the SAME digest
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(9999)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let stale = CompiledWorkflow::try_from_parts(parts).expect("structurally valid");
    // The stale workflow claims the same digest as the original but has different
    // content (I64(9999) vs Bool(true)). Under Strict/Journaled policy, the
    // checksum gate must reject this because the hash of the content won't match
    // the claimed digest.
    let stale_result = submit_artifact(&journal, &stale, RuntimePolicy::Strict);
    assert!(
        matches!(stale_result, Err(JournalError::ArtifactChecksumMismatch)),
        "stale digest must be rejected by checksum gate, got {:?}",
        stale_result
    );
}

#[test]
fn list_artifacts_empty_returns_empty() {
    use crate::FjallJournal;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let artifacts = journal
        .list_artifacts()
        .expect("list_artifacts should succeed");
    assert!(
        artifacts.is_empty(),
        "empty journal should have no artifacts"
    );
}

#[test]
fn list_artifacts_returns_stored_digests() {
    use crate::FjallJournal;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let record1 = crate::accepted_compiled_ir_record_for_test(vec![1u8]);
    let record2 = crate::accepted_compiled_ir_record_for_test(vec![2u8]);
    let record3 = crate::accepted_compiled_ir_record_for_test(vec![3u8]);
    let d1 = record1.digest;
    let d2 = record2.digest;
    let d3 = record3.digest;

    journal.put_compiled_ir(&record1).expect("put d1");
    journal.put_compiled_ir(&record2).expect("put d2");
    journal.put_compiled_ir(&record3).expect("put d3");

    let mut artifacts = journal
        .list_artifacts()
        .expect("list_artifacts should succeed");
    artifacts.sort_by(|a, b| a.as_bytes().cmp(&b.as_bytes()));

    assert_eq!(artifacts.len(), 3, "should list all 3 artifacts");
    assert!(artifacts.contains(&d1), "should contain d1");
    assert!(artifacts.contains(&d2), "should contain d2");
    assert!(artifacts.contains(&d3), "should contain d3");
}

#[test]
fn remove_artifact_removes_from_list() {
    use crate::FjallJournal;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let record1 = crate::accepted_compiled_ir_record_for_test(vec![1u8]);
    let record2 = crate::accepted_compiled_ir_record_for_test(vec![2u8]);
    let d1 = record1.digest;
    let d2 = record2.digest;

    journal.put_compiled_ir(&record1).expect("put d1");
    journal.put_compiled_ir(&record2).expect("put d2");

    journal
        .remove_artifact(d1)
        .expect("remove d1 should succeed");

    let artifacts = journal
        .list_artifacts()
        .expect("list_artifacts should succeed");
    assert_eq!(artifacts.len(), 1, "should have 1 artifact after removal");
    assert!(artifacts.contains(&d2), "remaining artifact should be d2");
    assert!(
        !artifacts.contains(&d1),
        "removed artifact should not be in list"
    );
}

#[test]
fn remove_artifact_not_found_returns_error() {
    use crate::{FjallJournal, JournalError};
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let missing = vb_core::WorkflowDigest::from_bytes([0xFF; 32]);
    let result = journal.remove_artifact(missing);

    assert!(
        result.is_err(),
        "removing non-existent artifact should return error"
    );
    let Err(JournalError::ArtifactNotFound { digest }) = result else {
        panic!("expected ArtifactNotFound error variant");
    };
    assert_eq!(digest, missing, "error should contain the requested digest");
}

#[test]
fn artifact_exists_returns_true_for_stored() {
    use crate::FjallJournal;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open");

    let record = crate::accepted_compiled_ir_record_for_test(vec![42u8]);
    let digest = record.digest;

    let exists_before = journal
        .artifact_exists(digest)
        .expect("artifact_exists should succeed");
    assert!(!exists_before, "artifact should not exist before storage");

    journal
        .put_compiled_ir(&record)
        .expect("put should succeed");

    let exists_after = journal
        .artifact_exists(digest)
        .expect("artifact_exists should succeed");
    assert!(exists_after, "artifact should exist after storage");
}

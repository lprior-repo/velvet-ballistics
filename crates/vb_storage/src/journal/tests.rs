#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]
use super::*;
use crate::{
    BlobRecord, EventSeq, IndexStatusState, JournalError, JournalEvent, RunHeaderRecord,
    WorkflowSourceRecord, constants::*, recovery::RunSnapshot,
};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}

fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

#[test]
fn parse_event_rejects_trailing_bytes_with_exact_offsets() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = crate::codec::encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let declared_end = bytes.len();
    bytes.extend_from_slice(&[0xE7, 0x7E]);

    let result = parse_event(&bytes);

    let Err(JournalError::UnexpectedTrailingBytes {
        declared_end: found_declared_end,
        actual_len,
    }) = result
    else {
        panic!("parse_event must reject trailing bytes, got {result:?}");
    };
    assert_eq!(found_declared_end, declared_end);
    assert_eq!(actual_len, bytes.len());
    Ok(())
}

fn corrupt_magic_preserving_crc(value: &mut [u8]) {
    let magic_bytes = 0xDEAD_BEEFu32.to_le_bytes();
    if let Some(slice) = value.get_mut(0..4) {
        slice.copy_from_slice(&magic_bytes);
    }
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
}

// =========================================================================
// Write/read round-trip tests
// =========================================================================

#[test]
fn workflow_source_roundtrip() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow: hello_world".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };
    journal
        .put_workflow_source(&record)
        .expect("put should succeed");
    let loaded = journal.workflow_source(digest).expect("get should succeed");
    let found = loaded.expect("workflow source must be present after successful put");
    assert_eq!(found.source, source);
    assert_eq!(found.digest, digest);
}

#[test]
fn workflow_source_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let result = journal
        .workflow_source(missing)
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing digest should return None");
}

#[test]
fn compiled_ir_roundtrip() {
    let (_temp, journal) = temp_journal();
    let record =
        crate::try_accepted_compiled_ir_record_for_test(b"compiled-artifact-bytes".to_vec())
            .expect("test fixture should encode");
    let digest = record.digest;
    journal
        .put_compiled_ir(&record)
        .expect("put should succeed");
    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let found = loaded.expect("compiled IR must be present after successful put");
    assert_eq!(found, record);
}

#[test]
fn compiled_ir_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0x00; DIGEST_BYTES]);
    let result = journal.compiled_ir(missing).expect("lookup should succeed");
    assert_eq!(result, None, "missing digest should return None");
}

#[test]
fn run_header_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(12345);
    let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);
    let record = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(7),
        compiled_digest: digest,
        status: 2,
        accepted_at_ms: 1700000000000,
    };
    journal.put_run_header(&record).expect("put should succeed");
    let loaded = journal.run_header(run).expect("get should succeed");
    let found = loaded.expect("run header must be present after successful put");
    assert_eq!(found.run, run);
    assert_eq!(found.workflow_id, WorkflowId::new(7));
    assert_eq!(found.status, 2);
    assert_eq!(found.accepted_at_ms, 1700000000000);
}

#[test]
fn run_header_returns_none_for_missing_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .run_header(RunId::new(999))
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing run should return None");
}

#[test]
fn snapshot_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(55);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow,
        slots: vec![0u8, 1u8],
        taint: vec![0u8, 0u8],
    };
    journal.put_snapshot(&snapshot).expect("put should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(10))
        .expect("get should succeed");
    let found = loaded.expect("snapshot must be present after successful put");
    assert_eq!(found.run, run);
    assert_eq!(found.seq, EventSeq::new(10));
    assert_eq!(found.slots.len(), 2);
    assert_eq!(found.workflow, workflow);
}

#[test]
fn blob_roundtrip() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };
    journal.put_blob(&record).expect("put should succeed");
    let loaded = journal.blob(digest).expect("get should succeed");
    let found = loaded.expect("blob must be present after successful put");
    assert_eq!(found.bytes, payload);
}

#[test]
fn blob_returns_none_for_missing_digest() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .blob([0; DIGEST_BYTES])
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing blob should return None");
}

#[test]
fn blob_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3];
    let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
    let record = BlobRecord {
        digest: wrong_digest,
        bytes: payload,
    };
    let result = journal.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "digest mismatch must be rejected, got {:?}",
        result
    );
}

// =========================================================================
// Keyspace isolation — different run IDs don't interfere
// =========================================================================

#[test]
fn events_for_run_isolates_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);

    // Write events for run A
    let events_a = [make_event(run_a, 0), make_step_started(run_a, 1, 0)];
    journal
        .append_strict_batch(&events_a)
        .expect("batch A should succeed");

    // Write events for run B
    let events_b = [
        make_event(run_b, 0),
        make_step_started(run_b, 1, 0),
        make_step_started(run_b, 2, 1),
    ];
    journal
        .append_strict_batch(&events_b)
        .expect("batch B should succeed");

    // Replay run A: should only get run A events
    let replayed_a = journal
        .events_for_run(run_a)
        .expect("replay A should succeed");
    assert_eq!(replayed_a.len(), 2, "run A should have exactly 2 events");
    for event in &replayed_a {
        assert_eq!(event.run_id(), run_a, "replayed event must belong to run A");
    }

    // Replay run B: should only get run B events
    let replayed_b = journal
        .events_for_run(run_b)
        .expect("replay B should succeed");
    assert_eq!(replayed_b.len(), 3, "run B should have exactly 3 events");
    for event in &replayed_b {
        assert_eq!(event.run_id(), run_b, "replayed event must belong to run B");
    }
}

#[test]
fn run_headers_isolate_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(10);
    let run_b = RunId::new(20);
    let digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

    journal
        .put_run_header(&RunHeaderRecord {
            run: run_a,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 100,
        })
        .expect("put A should succeed");

    journal
        .put_run_header(&RunHeaderRecord {
            run: run_b,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: 200,
        })
        .expect("put B should succeed");

    let header_a = journal
        .run_header(run_a)
        .expect("get A should succeed")
        .expect("A present");
    let header_b = journal
        .run_header(run_b)
        .expect("get B should succeed")
        .expect("B present");

    assert_eq!(header_a.workflow_id, WorkflowId::new(1));
    assert_eq!(header_a.status, 1);
    assert_eq!(header_b.workflow_id, WorkflowId::new(2));
    assert_eq!(header_b.status, 2);
}

#[test]
fn snapshots_isolate_different_runs() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(50);
    let run_b = RunId::new(60);
    let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);

    journal
        .put_snapshot(&RunSnapshot {
            run: run_a,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![0u8],
            taint: vec![],
        })
        .expect("put A should succeed");

    journal
        .put_snapshot(&RunSnapshot {
            run: run_b,
            seq: EventSeq::new(1),
            workflow,
            slots: vec![1u8, 2u8, 3u8],
            taint: vec![0u8],
        })
        .expect("put B should succeed");

    let snap_a = journal
        .snapshot(run_a, EventSeq::new(1))
        .expect("get A")
        .expect("present");
    let snap_b = journal
        .snapshot(run_b, EventSeq::new(1))
        .expect("get B")
        .expect("present");

    assert_eq!(snap_a.run, run_a);
    assert_eq!(snap_a.slots.len(), 1);
    assert_eq!(snap_b.run, run_b);
    assert_eq!(snap_b.slots.len(), 3);
}

// =========================================================================
// Sequential event ordering
// =========================================================================

#[test]
fn events_for_run_returns_events_in_sequence_order() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);

    let events: Vec<JournalEvent> = (0..5)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new(i as u16),
            attempt: 1,
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 5);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            i as u64,
            "event at index {} should have seq {}",
            i,
            i
        );
    }
}

#[test]
fn events_for_run_rejects_sequence_gap() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);

    // Write seq 0 and seq 2 (gap at seq 1)
    let e0 = make_event(run, 0);
    let e2 = make_event(run, 2);
    journal.append_unpersisted(&e0).expect("append 0");
    journal.append_unpersisted(&e2).expect("append 2");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::SequenceGap { .. })),
        "sequence gap must be detected during replay, got {:?}",
        result
    );
}

#[test]
fn events_for_run_returns_empty_for_unknown_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .events_for_run(RunId::new(99999))
        .expect("replay should succeed");
    assert_eq!(result.len(), 0, "unknown run should have zero events");
}

// =========================================================================
// Duplicate event rejection
// =========================================================================

#[test]
fn append_strict_rejects_duplicate_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);
    let event = make_event(run, 0);

    journal
        .append_strict(&event)
        .expect("first append should succeed");
    let result = journal.append_strict(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate event must be rejected, got {:?}",
        result
    );
}

#[test]
fn append_journaled_succeeds_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(600);
    let event = make_event(run, 0);

    journal
        .append_journaled(&event)
        .expect("append_journaled should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

#[test]
fn append_strict_batch_writes_all_events() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let events = [
        make_event(run, 0),
        make_step_started(run, 1, 0),
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 3);
    assert_eq!(replayed[0], events[0]);
    assert_eq!(replayed[1], events[1]);
    assert_eq!(replayed[2], events[2]);
}

#[test]
fn append_strict_batch_on_empty_succeeds() {
    let (_temp, journal) = temp_journal();
    let result = journal.append_strict_batch(&[]);
    let Ok(()) = result else {
        panic!("empty batch must succeed, got {:?}", result);
    };
}

// =========================================================================
// Declared keyspaces
// =========================================================================

#[test]
fn declared_keyspaces_count_matches_opened_keyspaces() {
    let declared = FjallJournal::declared_keyspaces();
    assert_eq!(
        declared.len(),
        11,
        "there should be 11 declared keyspaces (added run_seq_gap in vb-1rqz7.1)"
    );
    let (_temp, _journal) = temp_journal();
    // If we got here, all keyspaces opened successfully
}

// =========================================================================
// Verify content digest
// =========================================================================

#[test]
fn verify_content_digest_accepts_valid() {
    let content = b"some bytes";
    let hash = blake3::hash(content);
    let result = verify_content_digest(content, hash.as_bytes());
    let Ok(()) = result else {
        panic!("valid content digest must pass, got {:?}", result);
    };
}

#[test]
fn verify_content_digest_rejects_mismatch() {
    let content = b"some bytes";
    let wrong = blake3::hash(b"other bytes");
    let result = verify_content_digest(content, wrong.as_bytes());
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "mismatched digest should fail"
    );
}

// =========================================================================
// Multiple event types round-trip through journal
// =========================================================================

#[test]
fn all_event_variant_roundtrip_through_journal() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(800);
    let digest = WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: vb_core::ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            action: vb_core::ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            attempt: 1,
            deadline_ms: 30000,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(2),
            attempt: 1,
            deadline_ms: 30000,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), events.len());
    for (i, (original, replayed_event)) in events.iter().zip(replayed.iter()).enumerate() {
        assert_eq!(original, replayed_event, "event at index {} mismatch", i);
    }
}

// =========================================================================
// put_workflow_source with valid and invalid digests
// =========================================================================

#[test]
fn workflow_source_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow: real content".to_vec();
    let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = WorkflowSourceRecord {
        digest: wrong_digest,
        source,
    };
    let result = journal.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "wrong digest must be rejected, got {:?}",
        result
    );
}

#[test]
fn workflow_source_accepts_large_valid_payload() {
    let (_temp, journal) = temp_journal();
    // Build a source that is near but under the max (use 64 KiB)
    let source = vec![0x41u8; 65536];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };
    journal
        .put_workflow_source(&record)
        .expect("put should succeed for large valid payload");
    let loaded = journal.workflow_source(digest).expect("get should succeed");
    let found = loaded.expect("large workflow source must be present after successful put");
    assert_eq!(found.source.len(), source.len());
    assert_eq!(found.source, source);
}

#[test]
fn workflow_source_stores_multiple_distinct_digests() {
    let (_temp, journal) = temp_journal();
    let source_a = b"workflow: a".to_vec();
    let source_b = b"workflow: b".to_vec();
    let digest_a = WorkflowDigest::from_bytes(blake3::hash(&source_a).into());
    let digest_b = WorkflowDigest::from_bytes(blake3::hash(&source_b).into());
    let record_a = WorkflowSourceRecord {
        digest: digest_a,
        source: source_a.clone(),
    };
    let record_b = WorkflowSourceRecord {
        digest: digest_b,
        source: source_b.clone(),
    };
    journal
        .put_workflow_source(&record_a)
        .expect("put A should succeed");
    journal
        .put_workflow_source(&record_b)
        .expect("put B should succeed");
    let loaded_a = journal
        .workflow_source(digest_a)
        .expect("get A should succeed")
        .expect("A present");
    let loaded_b = journal
        .workflow_source(digest_b)
        .expect("get B should succeed")
        .expect("B present");
    assert_eq!(loaded_a.source, source_a);
    assert_eq!(loaded_b.source, source_b);
}

// =========================================================================
// put_snapshot with various sequence numbers
// =========================================================================

#[test]
fn snapshot_multiple_sequences_same_run() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let workflow = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);
    for seq_val in 0u64..5 {
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(seq_val),
            workflow,
            slots: vec![seq_val as u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("put should succeed");
    }
    // Each snapshot should be retrievable independently
    for seq_val in 0u64..5 {
        let loaded = journal
            .snapshot(run, EventSeq::new(seq_val))
            .expect("get should succeed")
            .expect("should be present");
        assert_eq!(loaded.seq, EventSeq::new(seq_val));
        assert_eq!(loaded.slots, vec![seq_val as u8]);
    }
}

#[test]
fn snapshot_returns_none_for_missing_sequence() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(101);
    let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
    // Write seq 0 and seq 5
    let snap0 = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    let snap5 = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow,
        slots: vec![5u8],
        taint: vec![],
    };
    journal.put_snapshot(&snap0).expect("put 0");
    journal.put_snapshot(&snap5).expect("put 5");
    // seq 3 should be missing
    let result = journal
        .snapshot(run, EventSeq::new(3))
        .expect("get should succeed");
    assert_eq!(result, None, "missing snapshot seq should return None");
    // but seq 0 and 5 are present
    let snap0 = journal
        .snapshot(run, EventSeq::new(0))
        .expect("get 0")
        .expect("seq 0 snapshot must be present");
    assert_eq!(snap0.seq, EventSeq::new(0));
    let snap5 = journal
        .snapshot(run, EventSeq::new(5))
        .expect("get 5")
        .expect("seq 5 snapshot must be present");
    assert_eq!(snap5.seq, EventSeq::new(5));
}

#[test]
fn snapshot_preserves_large_slots_and_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(102);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let slots = vec![0xAB_u8; 4096];
    let taint = vec![0x01_u8; 4096];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: slots.clone(),
        taint: taint.clone(),
    };
    journal.put_snapshot(&snapshot).expect("put should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(0))
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded.slots, slots);
    assert_eq!(loaded.taint, taint);
}

// =========================================================================
// get_run_header / put_run_header extended round-trips
// =========================================================================

#[test]
fn run_header_overwrite_updates_status() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(999);
    let digest = WorkflowDigest::from_bytes([0x01; DIGEST_BYTES]);
    let original = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: digest,
        status: 1,
        accepted_at_ms: 100,
    };
    journal.put_run_header(&original).expect("put original");
    let updated = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: digest,
        status: 3,
        accepted_at_ms: 100,
    };
    journal.put_run_header(&updated).expect("put updated");
    let loaded = journal
        .run_header(run)
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded.status, 3, "status should be updated to 3");
    assert_eq!(loaded.run, run);
}

#[test]
fn run_headers_returns_all_headers_in_order() {
    let (_temp, journal) = temp_journal();
    let digest = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
    // Insert in non-sorted order (run IDs 30, 10, 20)
    for run_id in [30u64, 10u64, 20u64] {
        let record = RunHeaderRecord {
            run: RunId::new(run_id),
            workflow_id: WorkflowId::new(run_id as u32),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: run_id,
        };
        journal.put_run_header(&record).expect("put should succeed");
    }
    let headers = journal.run_headers().expect("run_headers should succeed");
    assert_eq!(headers.len(), 3, "should have 3 headers");
    // run_headers are returned in key order (big-endian run IDs)
    let run_ids: Vec<u64> = headers.iter().map(|h| h.run.get()).collect();
    let mut sorted = run_ids.clone();
    sorted.sort();
    assert_eq!(run_ids, sorted, "headers should be in key order");
}

// =========================================================================
// Event append and sequential read edge cases
// =========================================================================

#[test]
fn append_queued_unpersisted_allows_idempotent_duplicate() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1100);
    let event = make_event(run, 0);
    journal
        .append_queued_unpersisted(&event)
        .expect("first append should succeed");
    let result = journal.append_queued_unpersisted(&event);
    let Ok(()) = result else {
        panic!(
            "idempotent duplicate of same event should succeed, got {:?}",
            result
        );
    };
}

#[test]
fn append_queued_unpersisted_rejects_different_duplicate() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1101);
    let event_a = make_event(run, 0);
    let mut event_b = make_event(run, 0);
    // Change the workflow digest so event_b differs
    if let JournalEvent::RunAccepted {
        ref mut workflow, ..
    } = event_b
    {
        *workflow = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    }
    journal
        .append_queued_unpersisted(&event_a)
        .expect("first append should succeed");
    let result = journal.append_queued_unpersisted(&event_b);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "different event at same run/seq must be rejected, got {:?}",
        result
    );
}

#[test]
fn events_for_run_with_many_events() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1200);
    let count: u64 = 100;
    let events: Vec<JournalEvent> = (0..count)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new(i as u16),
            attempt: 1,
        })
        .collect();
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), count as usize);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.seq().get(), i as u64);
        assert_eq!(event.run_id(), run);
    }
}

// =========================================================================
// Status index construction
// =========================================================================

#[test]
fn status_index_stores_and_scans_markers() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(2000);
    let run_b = RunId::new(2001);
    // Insert status markers for different states
    journal
        .put_status_index(IndexStatusState::Active, 1000, run_a)
        .expect("status A");
    journal
        .put_status_index(IndexStatusState::Completed, 2000, run_a)
        .expect("status B");
    journal
        .put_status_index(IndexStatusState::Active, 3000, run_b)
        .expect("status C");
    // Scan the entire keyspace
    let mut count = 0usize;
    for item in journal.index_status.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_status key must be Ok for all entries");
        };
        count = count.saturating_add(1);
    }
    assert_eq!(count, 3, "should have 3 status index markers");
}

// =========================================================================
// Workflow and action index construction
// =========================================================================

#[test]
fn workflow_index_stores_markers() {
    let (_temp, journal) = temp_journal();
    let wf1 = WorkflowId::new(1);
    let wf2 = WorkflowId::new(2);
    let run_a = RunId::new(3000);
    let run_b = RunId::new(3001);
    journal.put_workflow_index(wf1, run_a).expect("wf idx A");
    journal.put_workflow_index(wf1, run_b).expect("wf idx B");
    journal.put_workflow_index(wf2, run_a).expect("wf idx C");
    let mut count = 0usize;
    for item in journal.index_workflow.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_workflow key must be Ok for all entries");
        };
        count = count.saturating_add(1);
    }
    assert_eq!(count, 3, "should have 3 workflow index markers");
}

#[test]
fn action_index_stores_markers() {
    let (_temp, journal) = temp_journal();
    let action1 = vb_core::ActionId::new(10);
    let action2 = vb_core::ActionId::new(20);
    let run = RunId::new(4000);
    let step_a = StepIdx::new(1);
    let step_b = StepIdx::new(2);
    journal
        .put_action_index(action1, run, step_a)
        .expect("action idx A");
    journal
        .put_action_index(action1, run, step_b)
        .expect("action idx B");
    journal
        .put_action_index(action2, run, step_a)
        .expect("action idx C");
    let mut count = 0usize;
    for item in journal.index_action.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_action key must be Ok for all entries");
        };
        count = count.saturating_add(1);
    }
    assert_eq!(count, 3, "should have 3 action index markers");
}

// =========================================================================
// Cross-keyspace batch operations
// =========================================================================

#[test]
fn batch_commits_across_multiple_keyspaces() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5000);
    let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

    let source = b"batch workflow".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let workflow_record = WorkflowSourceRecord {
        digest: source_digest,
        source,
    };

    let ir_record = crate::try_accepted_compiled_ir_record_for_test(b"batch ir".to_vec())
        .expect("test fixture should encode");

    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(42),
        compiled_digest: digest,
        status: 1,
        accepted_at_ms: 9000,
    };

    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: source_digest,
    };

    let payload = vec![0xBB];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let blob_record = BlobRecord {
        digest: blob_digest,
        bytes: payload,
    };

    {
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&workflow_record)
            .expect("batch workflow source");
        batch
            .put_compiled_ir(&ir_record)
            .expect("batch compiled ir");
        batch.put_run_header(&header).expect("batch run header");
        batch.append_event(&event).expect("batch event");
        batch.put_blob(&blob_record).expect("batch blob");
        batch
            .put_status_index(IndexStatusState::Submitted, 100, run)
            .expect("batch status idx");
        batch
            .put_workflow_index(WorkflowId::new(42), run)
            .expect("batch workflow idx");
        batch
            .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
            .expect("batch action idx");
        assert_eq!(batch.len(), 8, "batch should contain 8 operations");
        assert!(!batch.is_empty(), "batch should not be empty");
        batch.commit().expect("batch commit should succeed");
    }

    // Verify all keyspaces have the data
    let ws = journal
        .workflow_source(source_digest)
        .expect("get ws")
        .expect("workflow source must be present after batch commit");
    assert_eq!(ws.digest, source_digest);
    let ir = journal
        .compiled_ir(ir_record.digest)
        .expect("get ir")
        .expect("compiled IR must be present after batch commit");
    assert_eq!(ir.digest, ir_record.digest);
    let header = journal
        .run_header(run)
        .expect("get header")
        .expect("run header must be present after batch commit");
    assert_eq!(header.run, run);
    let replayed = journal.events_for_run(run).expect("get events");
    assert_eq!(replayed.len(), 1);
    let blob = journal
        .blob(blob_digest)
        .expect("get blob")
        .expect("blob must be present after batch commit");
    assert_eq!(blob.digest, blob_digest);
}

#[test]
fn batch_rejects_workflow_source_with_wrong_digest() {
    let (_temp, journal) = temp_journal();
    let source = b"real content".to_vec();
    let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = WorkflowSourceRecord {
        digest: wrong_digest,
        source,
    };
    let mut batch = journal.batch();
    let result = batch.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch should reject digest mismatch, got {:?}",
        result
    );
}

#[test]
fn batch_rejects_blob_with_wrong_digest() {
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3];
    let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
    let record = BlobRecord {
        digest: wrong_digest,
        bytes: payload,
    };
    let mut batch = journal.batch();
    let result = batch.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch should reject blob digest mismatch, got {:?}",
        result
    );
}

#[test]
fn batch_empty_commit_succeeds() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert!(batch.is_empty(), "new batch should be empty");
    assert_eq!(batch.len(), 0, "new batch length should be 0");
    batch.commit().expect("empty batch commit should succeed");
}

#[test]
fn batch_with_strict_durability() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5001);
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        status: 0,
        accepted_at_ms: 1,
    };
    let batch = journal.batch();
    let mut batch = batch.strict();
    batch
        .put_run_header(&header)
        .expect("batch put should succeed");
    batch.commit().expect("strict batch commit should succeed");
    let header = journal
        .run_header(run)
        .expect("get")
        .expect("run header must be present after strict batch commit");
    assert_eq!(header.run, run);
}

// =========================================================================
// Missing key lookups return correct error / None
// =========================================================================

#[test]
fn compiled_ir_returns_none_for_unwritten_digest() {
    let (_temp, journal) = temp_journal();
    let missing = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);
    let result = journal.compiled_ir(missing).expect("lookup should succeed");
    assert_eq!(result, None, "missing compiled IR should return None");
}

#[test]
fn snapshot_returns_none_for_missing_run() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .snapshot(RunId::new(99999), EventSeq::new(0))
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing snapshot should return None");
}

#[test]
fn blob_returns_none_for_unwritten_digest() {
    let (_temp, journal) = temp_journal();
    let result = journal
        .blob([0x99; DIGEST_BYTES])
        .expect("lookup should succeed");
    assert_eq!(result, None, "missing blob should return None");
}

// =========================================================================
// compiled_ir with digest verification
// =========================================================================

#[test]
fn compiled_ir_roundtrip_large_payload() {
    let (_temp, journal) = temp_journal();
    let record = crate::try_accepted_compiled_ir_record_for_test(vec![0x42u8; 65536])
        .expect("test fixture should encode");
    let digest = record.digest;
    journal
        .put_compiled_ir(&record)
        .expect("put should succeed");
    let loaded = journal
        .compiled_ir(digest)
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded, record);
}

// =========================================================================
// persist_strict succeeds on idle journal
// =========================================================================

#[test]
fn persist_strict_succeeds_without_prior_writes() {
    let (_temp, journal) = temp_journal();
    journal
        .persist_strict()
        .expect("persist_strict on idle journal should succeed");
}

// =========================================================================
// Edge case: write and read single event
// =========================================================================

#[test]
fn write_and_read_single_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7777);
    let digest = WorkflowDigest::from_bytes([0xDD; DIGEST_BYTES]);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal
        .append_strict(&event)
        .expect("append should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1, "should have exactly one event");
    assert_eq!(replayed[0], event, "replayed event should match original");
}

// =========================================================================
// Edge case: write batch of 100 events, read all back in order
// =========================================================================

#[test]
fn write_batch_100_events_read_all_in_order() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(8888);
    let count: u64 = 100;

    let events: Vec<JournalEvent> = (0..count)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new((i % 256) as u16),
            attempt: 1,
        })
        .collect();

    journal
        .append_strict_batch(&events)
        .expect("batch append should succeed");

    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(
        replayed.len(),
        count as usize,
        "should have exactly 100 events"
    );

    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            i as u64,
            "event at index {} should have seq {}",
            i,
            i
        );
        assert_eq!(event.run_id(), run, "all events should belong to same run");
        assert_eq!(
            *event, events[i],
            "event at index {} should match original",
            i
        );
    }
}

// =========================================================================
// Edge case: read from empty journal
// =========================================================================

#[test]
fn read_from_empty_journal_returns_nothing() {
    let (_temp, journal) = temp_journal();

    // All read operations on an empty journal should return None or empty
    let events = journal
        .events_for_run(RunId::new(1))
        .expect("events_for_run on empty journal should succeed");
    assert_eq!(events.len(), 0, "empty journal should have no events");

    let source = journal
        .workflow_source(WorkflowDigest::from_bytes([0; DIGEST_BYTES]))
        .expect("workflow_source on empty journal should succeed");
    assert_eq!(
        source, None,
        "empty journal should return None for workflow source"
    );

    let ir = journal
        .compiled_ir(WorkflowDigest::from_bytes([0; DIGEST_BYTES]))
        .expect("compiled_ir on empty journal should succeed");
    assert_eq!(ir, None, "empty journal should return None for compiled IR");

    let header = journal
        .run_header(RunId::new(1))
        .expect("run_header on empty journal should succeed");
    assert_eq!(
        header, None,
        "empty journal should return None for run header"
    );

    let snap = journal
        .snapshot(RunId::new(1), EventSeq::new(0))
        .expect("snapshot on empty journal should succeed");
    assert_eq!(snap, None, "empty journal should return None for snapshot");

    let blob = journal
        .blob([0; DIGEST_BYTES])
        .expect("blob on empty journal should succeed");
    assert_eq!(blob, None, "empty journal should return None for blob");

    let headers = journal
        .run_headers()
        .expect("run_headers on empty journal should succeed");
    assert_eq!(headers.len(), 0, "empty journal should have no run headers");
}

// =========================================================================
// Edge case: header validation - corrupted magic in stored record
// =========================================================================

#[test]
fn header_validation_rejects_corrupted_magic_in_stored_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9001);
    let event = make_event(run, 0);
    journal
        .append_strict(&event)
        .expect("append should succeed");

    // Directly corrupt the magic bytes in the underlying keyspace
    let key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("key construction should succeed");
    let mut value = journal
        .events
        .get(key.as_slice())
        .expect("fjall get should succeed")
        .expect("event should exist")
        .to_vec();

    // Overwrite the first 4 bytes (magic) with garbage
    let garbage_magic: u32 = 0xDEAD_BEEF;
    let magic_bytes = garbage_magic.to_le_bytes();
    if let Some(slice) = value.get_mut(0..4) {
        slice.copy_from_slice(&magic_bytes);
    }
    // Recompute CRC so the header passes CRC check but has wrong magic
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    // Write back corrupted value
    journal
        .events
        .insert(key.to_vec(), value)
        .expect("insert should succeed");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "corrupted magic in stored event must yield BadMagic, got {:?}",
        result
    );
}

// =========================================================================
// Edge case: header validation - wrong schema version in stored record
// =========================================================================

#[test]
fn header_validation_rejects_wrong_schema_version_in_stored_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9002);
    let event = make_event(run, 0);
    journal
        .append_strict(&event)
        .expect("append should succeed");

    let key =
        crate::keys::run_event_key(run, EventSeq::new(0)).expect("key construction should succeed");
    let mut value = journal
        .events
        .get(key.as_slice())
        .expect("fjall get should succeed")
        .expect("event should exist")
        .to_vec();

    // Overwrite schema version (offset 4, u16 LE) with a future version
    let future_version: u16 = 99;
    let version_bytes = future_version.to_le_bytes();
    if let Some(slice) = value.get_mut(4..6) {
        slice.copy_from_slice(&version_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&value[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = value.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    journal
        .events
        .insert(key.to_vec(), value)
        .expect("insert should succeed");

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
        "wrong schema version in stored event must yield UnsupportedSchemaVersion, got {:?}",
        result
    );
}

// =========================================================================
// Edge case: snapshot round-trip (write snapshot, read back)
// =========================================================================

#[test]
fn snapshot_roundtrip_preserves_all_fields() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7777);
    let workflow = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
    let slots = vec![0x01_u8, 0x02, 0x03, 0x04];
    let taint = vec![0xFF_u8, 0xFE, 0xFD];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(42),
        workflow,
        slots: slots.clone(),
        taint: taint.clone(),
    };

    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(42))
        .expect("snapshot get should succeed")
        .expect("snapshot should exist");

    assert_eq!(loaded.run, run, "run must match");
    assert_eq!(loaded.seq, EventSeq::new(42), "seq must match");
    assert_eq!(loaded.workflow, workflow, "workflow must match");
    assert_eq!(loaded.slots, slots, "slots must match byte-for-byte");
    assert_eq!(loaded.taint, taint, "taint must match byte-for-byte");
}

#[test]
fn snapshot_roundtrip_with_empty_slots_and_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(7778);
    let workflow = WorkflowDigest::from_bytes([0; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: vec![],
        taint: vec![],
    };

    journal
        .put_snapshot(&snapshot)
        .expect("put_snapshot should succeed");
    let loaded = journal
        .snapshot(run, EventSeq::new(0))
        .expect("snapshot get should succeed")
        .expect("snapshot should exist");

    assert_eq!(loaded.slots.len(), 0, "slots should be empty");
    assert_eq!(loaded.taint.len(), 0, "taint should be empty");
    assert_eq!(loaded, snapshot, "full snapshot must round-trip");
}

// =========================================================================
// Edge case: source marker round-trip (workflow source)
// =========================================================================

#[test]
fn source_marker_roundtrip_preserves_content_and_digest() {
    let (_temp, journal) = temp_journal();
    let source = b"workflow:\n  name: edge_test\n  steps: []".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    journal
        .put_workflow_source(&record)
        .expect("put_workflow_source should succeed");
    let loaded = journal
        .workflow_source(digest)
        .expect("workflow_source get should succeed")
        .expect("workflow source should exist");

    assert_eq!(
        loaded.source, source,
        "source bytes must round-trip exactly"
    );
    assert_eq!(loaded.digest, digest, "digest must round-trip exactly");
}

#[test]
fn source_marker_roundtrip_with_minimal_source() {
    let (_temp, journal) = temp_journal();
    // Minimal valid source: empty bytes
    let source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };

    journal
        .put_workflow_source(&record)
        .expect("put_workflow_source should succeed for empty source");
    let loaded = journal
        .workflow_source(digest)
        .expect("workflow_source get should succeed")
        .expect("workflow source should exist");

    assert_eq!(loaded.source.len(), 0, "empty source should round-trip");
    assert_eq!(loaded.source, source);
}

// =========================================================================
// Edge case: multiple persist_strict calls in sequence
// =========================================================================

#[test]
fn multiple_persist_strict_calls_succeed() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(9900);
    let event = make_event(run, 0);
    journal.append_strict(&event).expect("first append");
    journal.persist_strict().expect("first persist");
    journal.persist_strict().expect("second persist");
    journal.persist_strict().expect("third persist");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), 1);
}

// =========================================================================
// Edge case: interleaved writes to different runs
// =========================================================================

#[test]
fn interleaved_writes_to_different_runs_preserves_ordering() {
    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(10001);
    let run_b = RunId::new(10002);
    let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");

    let replayed_a = journal.events_for_run(run_a).expect("replay A");
    let replayed_b = journal.events_for_run(run_b).expect("replay B");

    assert_eq!(replayed_a.len(), 3, "run A should have 3 events");
    assert_eq!(replayed_b.len(), 2, "run B should have 2 events");
    assert_eq!(replayed_a[0].seq().get(), 0);
    assert_eq!(replayed_a[1].seq().get(), 1);
    assert_eq!(replayed_a[2].seq().get(), 2);
    assert_eq!(replayed_b[0].seq().get(), 0);
    assert_eq!(replayed_b[1].seq().get(), 1);
}

// =========================================================================
// Edge case: batch with single element
// =========================================================================

#[test]
fn append_strict_batch_single_element_roundtrips() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10100);
    let event = make_event(run, 0);
    journal
        .append_strict_batch(&[event.clone()])
        .expect("single-element batch");
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

// =========================================================================
// Edge case: duplicate detection after batch commit
// =========================================================================

#[test]
fn duplicate_event_after_batch_commit_is_rejected() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10200);
    let event = make_event(run, 0);
    journal
        .append_strict_batch(&[event.clone()])
        .expect("batch commit");
    let result = journal.append_strict(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate after batch should be rejected, got {:?}",
        result
    );
}

// =========================================================================
// Edge case: snapshot with post-snapshot events starts replay after snapshot
// =========================================================================

#[test]
fn events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10300);
    let workflow = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);

    // Write a snapshot at seq 2
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("put snapshot");

    // Only write events at seq 3, 4, 5 (as if snapshot-covered events were trimmed)
    for seq in 3u64..6 {
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(seq as u16),
            attempt: 1,
        };
        journal
            .append_unpersisted(&event)
            .expect("append should succeed");
    }

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(
        replayed.len(),
        3,
        "should replay 3 events starting after snapshot seq"
    );
    assert_eq!(replayed[0].seq().get(), 3);
    assert_eq!(replayed[1].seq().get(), 4);
    assert_eq!(replayed[2].seq().get(), 5);
}

#[test]
fn events_for_run_bounded_rejects_over_limit() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10301);
    for seq in 0u16..2 {
        journal
            .append_unpersisted(&make_step_started(run, u64::from(seq), seq))
            .map_err(|err| err.to_string())?;
    }

    let Some(limit) = EventReplayLimit::new(1) else {
        return Err("non-zero replay limit was rejected".to_owned());
    };
    let result = journal.events_for_run_bounded(run, limit);
    assert!(
        matches!(
            result,
            Err(JournalError::TooManyEvents {
                run: found_run,
                limit: 1,
                observed: 2,
            }) if found_run == run
        ),
        "bounded replay must fail closed when more events are present"
    );
    Ok(())
}

#[test]
fn events_for_run_detects_missing_first_tail_event_after_snapshot() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10302);
    let workflow = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unpersisted(&make_step_started(run, 4, 4))
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap {
                expected,
                actual,
            }) if expected == EventSeq::new(3) && actual == EventSeq::new(4)
        ),
        "missing first event after the durable snapshot must not be laundered"
    );
    Ok(())
}

#[test]
fn events_for_run_without_snapshot_rejects_missing_initial_sequence() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10303);
    journal
        .append_unpersisted(&make_step_started(run, 4, 4))
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap { expected, actual })
                if expected == EventSeq::new(0) && actual == EventSeq::new(4)
        ),
        "runs without snapshots must start at seq 0, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_corrupt_latest_snapshot_before_skipping_events() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10304);
    let workflow = WorkflowDigest::from_bytes([0x45; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow,
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unpersisted(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let mut value = journal
        .run_snapshot
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "snapshot record should exist".to_owned())?
        .to_vec();
    corrupt_magic_preserving_crc(&mut value);
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "corrupt latest snapshot must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay()
-> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10306);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: WorkflowDigest::from_bytes([0x47; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;
    journal
        .append_unpersisted(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let mut value = journal
        .run_snapshot
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "snapshot record should exist".to_owned())?
        .to_vec();
    let last = value
        .last_mut()
        .ok_or_else(|| "snapshot record must be non-empty".to_owned())?;
    *last ^= 0x01;
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupt latest snapshot payload digest must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_rejects_latest_snapshot_postcard_decode_failure_before_tail_replay()
-> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10307);
    journal
        .append_unpersisted(&make_step_started(run, 3, 3))
        .map_err(|err| err.to_string())?;

    let key =
        crate::keys::run_snapshot_key(run, EventSeq::new(2)).map_err(|err| err.to_string())?;
    let invalid_payload = vec![0xFF_u8];
    let value = crate::codec::encode_record(
        MAGIC_SNAPSHOT,
        crate::records::RecordKind::Snapshot,
        2,
        &invalid_payload,
        MAX_SNAPSHOT_BYTES,
    )
    .map_err(|err| err.to_string())?;
    journal
        .run_snapshot
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let result = journal.events_for_run(run);
    assert!(
        matches!(result, Err(JournalError::PostcardDecodeFailed)),
        "invalid latest snapshot payload must fail before tail replay, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn events_for_run_skips_corrupt_pre_snapshot_event_by_key_range() -> Result<(), String> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10305);
    for seq in 0_u64..3 {
        journal
            .append_unpersisted(&make_step_started(run, seq, seq as u16))
            .map_err(|err| err.to_string())?;
    }
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([0x46; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    };
    journal
        .put_snapshot(&snapshot)
        .map_err(|err| err.to_string())?;

    let key = crate::keys::run_event_key(run, EventSeq::new(0)).map_err(|err| err.to_string())?;
    let mut value = journal
        .events
        .get(key.as_slice())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "event record should exist".to_owned())?
        .to_vec();
    corrupt_magic_preserving_crc(&mut value);
    journal
        .events
        .insert(key.to_vec(), value)
        .map_err(|err| err.to_string())?;

    let replayed = journal.events_for_run(run).map_err(|err| err.to_string())?;
    assert_eq!(replayed, vec![make_step_started(run, 2, 2)]);
    Ok(())
}

// =========================================================================
// Edge case: compiled_ir stores multiple distinct digests
// =========================================================================

#[test]
fn compiled_ir_stores_multiple_distinct_digests() {
    let (_temp, journal) = temp_journal();
    let record_v1 = crate::try_accepted_compiled_ir_record_for_test(b"compiled-v1".to_vec())
        .expect("test fixture should encode");
    let digest_v1 = record_v1.digest;
    journal.put_compiled_ir(&record_v1).expect("put v1");

    let record_v2 = crate::try_accepted_compiled_ir_record_for_test(b"compiled-v2".to_vec())
        .expect("test fixture should encode");
    let digest_v2 = record_v2.digest;
    journal.put_compiled_ir(&record_v2).expect("put v2");

    let loaded_v1 = journal
        .compiled_ir(digest_v1)
        .expect("get v1")
        .expect("present");
    let loaded_v2 = journal
        .compiled_ir(digest_v2)
        .expect("get v2")
        .expect("present");
    assert_eq!(loaded_v1, record_v1);
    assert_eq!(loaded_v2, record_v2);
}

// =========================================================================
// Edge case: many distinct runs with one event each
// =========================================================================

#[test]
fn many_runs_one_event_each_are_isolated() {
    let (_temp, journal) = temp_journal();
    let count: u64 = 20;
    for i in 0u64..count {
        let run = RunId::new(11000_u64.saturating_add(i));
        let event = make_event(run, 0);
        journal
            .append_unpersisted(&event)
            .expect("append should succeed");
    }

    for i in 0u64..count {
        let run = RunId::new(11000_u64.saturating_add(i));
        let replayed = journal.events_for_run(run).expect("replay");
        assert_eq!(replayed.len(), 1, "run {} should have 1 event", i);
        assert_eq!(replayed[0].run_id(), run);
    }
}

// =========================================================================
// Edge case: large batch write of 500 events
// =========================================================================

#[test]
fn large_batch_500_events_roundtrips() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(12000);
    let count: u64 = 500;

    let events: Vec<JournalEvent> = (0..count)
        .map(|i| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(i),
            step: StepIdx::new((i % 256) as u16),
            attempt: 1,
        })
        .collect();

    journal
        .append_strict_batch(&events)
        .expect("batch should succeed");
    let replayed = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(replayed.len(), count as usize);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.seq().get(), i as u64);
    }
}

// =========================================================================
// Edge case: boundary sequence numbers (0 and 1)
// =========================================================================

#[test]
fn boundary_sequence_numbers_roundtrip() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(13000);
    let digest = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal.append_unpersisted(&e0).expect("append seq 0");

    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal.append_unpersisted(&e1).expect("append seq 1");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].seq().get(), 0);
    assert_eq!(replayed[1].seq().get(), 1);
}

// =========================================================================
// Edge case: blob with 16 KiB payload roundtrips
// =========================================================================

#[test]
fn blob_large_payload_roundtrips() {
    let (_temp, journal) = temp_journal();
    let payload = vec![0xFE_u8; 16384];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload.clone(),
    };
    journal.put_blob(&record).expect("put should succeed");
    let loaded = journal
        .blob(digest)
        .expect("get should succeed")
        .expect("present");
    assert_eq!(loaded.bytes, payload);
}

// =========================================================================
// Edge case: batch append_event allows duplicate key (last write wins)
// =========================================================================

#[test]
fn batch_append_event_rejects_duplicate_key_within_batch() {
    // SA-003: two append_event calls with the same (run, seq) inside a
    // single batch must be rejected so the second cannot silently
    // overwrite the first via Fjall last-write-wins.
    let (_temp, journal) = temp_journal();
    let run = RunId::new(14000);
    let event = make_event(run, 0);

    let mut batch = journal.batch();
    batch.append_event(&event).expect("first batch append");
    let result = batch.append_event(&event);
    assert!(
        matches!(result, Err(crate::error::JournalError::DuplicateEvent { .. })),
        "second batch append with same key must yield DuplicateEvent, got {:?}",
        result
    );
    batch.commit().expect("commit should succeed");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(
        replayed.len(),
        0,
        "aborted batch must not persist any event, got {}",
        replayed.len()
    );
}

// =========================================================================
// Edge case: batch with snapshot and event for same run
// =========================================================================

#[test]
fn batch_with_snapshot_and_event_for_same_run() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(15000);
    let workflow = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow,
        slots: vec![1, 2, 3],
        taint: vec![],
    };
    let event = make_event(run, 1);

    let mut batch = journal.batch();
    batch.put_snapshot(&snapshot).expect("batch snapshot");
    batch.append_event(&event).expect("batch event");
    batch.commit().expect("commit");

    let loaded_snap = journal
        .snapshot(run, EventSeq::new(0))
        .expect("get snapshot")
        .expect("present");
    assert_eq!(loaded_snap.slots, vec![1, 2, 3]);
    // Snapshot at seq 0 covers events 0..0; event at seq 1 is after snapshot
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
}

// =========================================================================
// Edge case: workflow source with 128 KiB payload
// =========================================================================

#[test]
fn workflow_source_near_max_bytes_is_accepted() {
    let (_temp, journal) = temp_journal();
    let source = vec![0x42u8; 131072];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: source.clone(),
    };
    journal
        .put_workflow_source(&record)
        .expect("put should succeed for 128 KiB");
    let loaded = journal
        .workflow_source(digest)
        .expect("get")
        .expect("present");
    assert_eq!(loaded.source.len(), source.len());
}

// =========================================================================
// Edge case: run header with max field values
// =========================================================================

#[test]
fn run_header_with_max_field_values() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(u64::MAX);
    let digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(u32::MAX),
        compiled_digest: digest,
        status: u8::MAX,
        accepted_at_ms: u64::MAX,
    };
    journal.put_run_header(&record).expect("put should succeed");
    let loaded = journal.run_header(run).expect("get").expect("present");
    assert_eq!(loaded.run, run);
    assert_eq!(loaded.workflow_id, vb_core::WorkflowId::new(u32::MAX));
    assert_eq!(loaded.status, u8::MAX);
    assert_eq!(loaded.accepted_at_ms, u64::MAX);
}

// =========================================================================
// Edge case: empty run headers list
// =========================================================================

#[test]
fn run_headers_empty_journal_returns_empty_vec() {
    let (_temp, journal) = temp_journal();
    let headers = journal.run_headers().expect("run_headers on empty journal");
    assert_eq!(headers.len(), 0, "empty journal should have no headers");
}

// =========================================================================
// Edge case: snapshot with populated slots and empty taint
// =========================================================================

#[test]
fn snapshot_with_populated_slots_empty_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(16000);
    let workflow = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);
    let slots = vec![0x01_u8; 256];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(10),
        workflow,
        slots: slots.clone(),
        taint: vec![],
    };
    journal.put_snapshot(&snapshot).expect("put");
    let loaded = journal
        .snapshot(run, EventSeq::new(10))
        .expect("get")
        .expect("present");
    assert_eq!(loaded.slots, slots);
    assert!(loaded.taint.is_empty());
}

// =========================================================================
// Edge case: snapshot with empty slots and populated taint
// =========================================================================

#[test]
fn snapshot_with_empty_slots_populated_taint() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(16001);
    let workflow = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
    let taint = vec![0xFF_u8; 128];
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(5),
        workflow,
        slots: vec![],
        taint: taint.clone(),
    };
    journal.put_snapshot(&snapshot).expect("put");
    let loaded = journal
        .snapshot(run, EventSeq::new(5))
        .expect("get")
        .expect("present");
    assert!(loaded.slots.is_empty());
    assert_eq!(loaded.taint, taint);
}

// =========================================================================
// Edge case: append_queued_unpersisted then read back
// =========================================================================

#[test]
fn append_queued_unpersisted_then_read_back() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(17000);
    let event = make_event(run, 0);
    journal
        .append_queued_unpersisted(&event)
        .expect("append should succeed");
    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0], event);
}

// =========================================================================
// Edge case: consecutive append_journaled calls
// =========================================================================

#[test]
fn consecutive_append_journaled_calls() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(18000);

    for seq in 0u64..10 {
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(seq as u16),
            attempt: 1,
        };
        journal
            .append_journaled(&event)
            .expect("append_journaled should succeed");
    }

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 10);
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.seq().get(), i as u64);
    }
}

// =========================================================================
// Edge case: mixed durability modes on same run
// =========================================================================

#[test]
fn mixed_durability_modes_on_same_run() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(19000);
    let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal.append_strict(&e0).expect("strict append");

    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    journal.append_journaled(&e1).expect("journaled append");

    let e2 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        attempt: 1,
    };
    journal.append_strict(&e2).expect("strict append 2");

    let replayed = journal.events_for_run(run).expect("replay");
    assert_eq!(replayed.len(), 3);
    assert_eq!(replayed[0], e0);
    assert_eq!(replayed[1], e1);
    assert_eq!(replayed[2], e2);
}

// =========================================================================
// Edge case: index keyspaces remain empty until explicitly written
// =========================================================================

#[test]
fn index_keyspaces_empty_after_regular_writes() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20000);
    let event = make_event(run, 0);
    journal.append_strict(&event).expect("append");

    let mut status_count = 0usize;
    for item in journal.index_status.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_status key must be Ok for all entries");
        };
        status_count = status_count.saturating_add(1);
    }
    assert_eq!(status_count, 0, "status index should be empty");

    let mut workflow_count = 0usize;
    for item in journal.index_workflow.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_workflow key must be Ok for all entries");
        };
        workflow_count = workflow_count.saturating_add(1);
    }
    assert_eq!(workflow_count, 0, "workflow index should be empty");

    let mut action_count = 0usize;
    for item in journal.index_action.iter() {
        let Ok(_key) = item.key() else {
            panic!("index_action key must be Ok for all entries");
        };
        action_count = action_count.saturating_add(1);
    }
    assert_eq!(action_count, 0, "action index should be empty");
}

// =========================================================================
// Edge case: all declared keyspaces are queryable after open
// =========================================================================

#[test]
fn all_declared_keyspaces_are_iterable_after_open() {
    let (_temp, journal) = temp_journal();
    assert!(journal.workflow_source.iter().next().is_none());
    assert!(journal.compiled_ir.iter().next().is_none());
    assert!(journal.run_header.iter().next().is_none());
    assert!(journal.events.iter().next().is_none());
    assert!(journal.run_snapshot.iter().next().is_none());
    assert!(journal.blob.iter().next().is_none());
    assert!(journal.index_status.iter().next().is_none());
    assert!(journal.index_workflow.iter().next().is_none());
    assert!(journal.index_action.iter().next().is_none());
    assert!(journal.recovery_stamp.iter().next().is_none());
}

// =========================================================================
// close() method tests
// =========================================================================

/// Test that close() propagates errors from persist_strict on a clean journal.
/// A clean (empty) journal should not produce persist errors.
#[test]
fn close_succeeds_on_clean_journal() {
    let (_temp, mut journal) = temp_journal();
    // close() should succeed on idle journal
    journal
        .close()
        .expect("close on clean journal should succeed");
}

/// Test that close() returns unit on success.
#[test]
fn close_returns_unit_on_success() {
    let (_temp, mut journal) = temp_journal();
    let result = journal.close();
    let Ok(()) = result else {
        panic!("close should return Ok(()) on success, got {:?}", result);
    };
}

/// Test that drop does NOT call close() by verifying the process lock is released
/// even when close() is not called. The journal can be reopened after drop.
#[test]
fn drop_releases_process_lock_even_without_explicit_close() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();

    // First, open and hold the journal
    {
        let journal = FjallJournal::open(&path, None).expect("journal open should succeed");
        let run = RunId::new(9999);
        let digest = WorkflowDigest::from_bytes([0xEE; DIGEST_BYTES]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal
            .append_strict(&event)
            .expect("append should succeed");
        // Explicitly do NOT call close() here
    } // journal drops here without explicit close

    // The process lock should be released when journal drops, allowing reopen
    // If drop called close() and close() succeeded, this would also work.
    // If drop called close() and close() failed, the process lock might not be released.
    let journal2 =
        FjallJournal::open(&path, None).expect("journal should be reopenable after drop");
    let replayed = journal2
        .events_for_run(RunId::new(9999))
        .expect("replay should succeed");
    assert_eq!(
        replayed.len(),
        1,
        "should have exactly one event after reopen"
    );
}

/// Test that close() propagates persist errors when underlying storage fails.
/// This uses a test-only fault hook to force the strict durability boundary to fail.
#[test]
fn close_propagates_persist_errors() {
    let (_temp, mut journal) = temp_journal();
    journal.fail_next_persist_for_test();

    let result = journal.close();
    assert!(
        matches!(result, Err(JournalError::StrictDurabilityFailed)),
        "close must propagate strict durability failures, got {:?}",
        result
    );
}

// =========================================================================
// vb-k6iwh: BDD Fjall round-trip for RecoveryStampRecord
// (mirrors the codec round-trip test in tests.rs:recovery_stamp_record_round_trips_through_decoder
//  but exercises the actual FjallJournal::put_recovery_stamp / get_recovery_stamp methods).
// =========================================================================

/// Given a typed `RecoveryStampRecord` and a real `FjallJournal`
/// When the record is persisted via `FjallJournal::put_recovery_stamp`
///      and read back via `FjallJournal::get_recovery_stamp`
/// Then the deserialized record is byte-equal to the original, the prefix
///      is `PREFIX_RECOVERY_STAMP` (`0x40`), and the key matches
///      `[0x40][run_id_be][seq_be]`.
#[test]
fn recovery_stamp_persists_and_reads_back_via_fjall() {
    use crate::RecoveryStampRecord;
    use crate::constants::PREFIX_RECOVERY_STAMP;
    use crate::keys::recovery_stamp_key;

    let (_temp, journal) = temp_journal();
    let run = RunId::new(7);
    let seq = EventSeq::new(13);
    let stamp = RecoveryStampRecord {
        run,
        last_seq: seq,
        written_at_ms: 1_700_000_000_000,
    };

    // First read must be None (no record has been written for this (run, seq)).
    let pre = journal
        .get_recovery_stamp(run, seq)
        .expect("get_recovery_stamp pre-read must succeed");
    assert!(
        pre.is_none(),
        "no recovery_stamp has been written for (run={}, seq={}) yet",
        run.get(),
        seq.get()
    );

    journal
        .put_recovery_stamp(run, seq, stamp)
        .expect("put_recovery_stamp must succeed");

    let loaded = journal
        .get_recovery_stamp(run, seq)
        .expect("get_recovery_stamp post-write must succeed")
        .expect("get_recovery_stamp must return Some after put");
    assert_eq!(loaded, stamp, "loaded record must equal the persisted one");

    // The decoded key must round-trip and the prefix must be 0x40.
    let key = recovery_stamp_key(run, seq).expect("recovery_stamp_key must succeed");
    assert_eq!(key.len(), crate::constants::RECOVERY_STAMP_KEY_BYTES);
    assert_eq!(
        key[0], PREFIX_RECOVERY_STAMP,
        "recovery_stamp key prefix must be 0x40"
    );
}

/// vb-1rqz7.33 / SR-014 — `put_recovery_stamp` must durably persist the stamp
/// so it survives a process reopen.
#[test]
fn recovery_stamp_survives_journal_reopen() {
    use crate::RecoveryStampRecord;

    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(0xD014);
    let seq = EventSeq::new(42);
    let stamp = RecoveryStampRecord {
        run,
        last_seq: seq,
        written_at_ms: 1_700_000_000_000,
    };

    {
        let journal = FjallJournal::open(&path, None).expect("open should succeed");
        journal
            .put_recovery_stamp(run, seq, stamp)
            .expect("put_recovery_stamp should succeed");
    }

    // Reopen the journal and confirm the stamp is still present.
    let reopened = FjallJournal::open(&path, None).expect("reopen should succeed");
    let loaded = reopened
        .get_recovery_stamp(run, seq)
        .expect("get_recovery_stamp should succeed")
        .expect("stamp must survive reopen");
    assert_eq!(
        loaded, stamp,
        "recovery_stamp must be durably persisted across reopen"
    );
}

/// Given two distinct `(run, seq)` keys
/// When two `RecoveryStampRecord` values are written via Fjall
/// Then each lookup returns only its own record (no cross-key aliasing).
#[test]
fn recovery_stamp_keys_do_not_alias_in_fjall() {
    use crate::RecoveryStampRecord;

    let (_temp, journal) = temp_journal();
    let run_a = RunId::new(101);
    let run_b = RunId::new(102);
    let seq_a = EventSeq::new(1);
    let seq_b = EventSeq::new(2);

    let stamp_a = RecoveryStampRecord {
        run: run_a,
        last_seq: seq_a,
        written_at_ms: 100,
    };
    let stamp_b = RecoveryStampRecord {
        run: run_b,
        last_seq: seq_b,
        written_at_ms: 200,
    };

    journal
        .put_recovery_stamp(run_a, seq_a, stamp_a)
        .expect("put A must succeed");
    journal
        .put_recovery_stamp(run_b, seq_b, stamp_b)
        .expect("put B must succeed");

    let got_a = journal
        .get_recovery_stamp(run_a, seq_a)
        .expect("get A must succeed")
        .expect("get A must be Some");
    let got_b = journal
        .get_recovery_stamp(run_b, seq_b)
        .expect("get B must succeed")
        .expect("get B must be Some");
    assert_eq!(got_a, stamp_a, "A lookup must return stamp A");
    assert_eq!(got_b, stamp_b, "B lookup must return stamp B");
    assert_ne!(got_a, got_b, "stamps A and B must be distinct records");
}

/// Given a closed and reopened `FjallJournal`
/// When a `RecoveryStampRecord` is written before close and read after reopen
/// Then the record is durable across the reopen boundary.
#[test]
fn recovery_stamp_survives_close_and_reopen() {
    use crate::RecoveryStampRecord;
    use crate::types::FjallConfig;

    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(555);
    let seq = EventSeq::new(99);
    let stamp = RecoveryStampRecord {
        run,
        last_seq: seq,
        written_at_ms: 1_234_567_890,
    };

    // Phase 1: write the stamp, then close the journal.
    {
        let mut journal =
            FjallJournal::open(&path, Some(FjallConfig::default())).expect("open should succeed");
        journal
            .put_recovery_stamp(run, seq, stamp)
            .expect("put_recovery_stamp must succeed");
        journal.close().expect("close must succeed");
    }

    // Phase 2: reopen the journal and read the stamp back.
    {
        let journal =
            FjallJournal::open(&path, Some(FjallConfig::default())).expect("reopen should succeed");
        let loaded = journal
            .get_recovery_stamp(run, seq)
            .expect("get_recovery_stamp must succeed")
            .expect("recovery_stamp must survive close/reopen");
        assert_eq!(loaded, stamp, "reopened stamp must equal the persisted one");
    }
}

// =========================================================================
// vb-k6iwh-r: TRUE end-to-end BDD — recovery path writes the stamp, and
// the stamp persists across a journal close/reopen boundary.
//
// This test exercises the full production recovery code path
// (crate::recovery::recover_full_journal), not just the put/get helpers in
// isolation. It is the proof that the recovery_stamp keyspace is wired into
// the recovery path (not a phantom keyspace).
// =========================================================================

/// Given a fresh `FjallJournal` with a small accepted/admission/finished event
///      sequence for one run,
/// When the production recovery path is invoked via
///      `crate::recovery::recover_full_journal`,
/// Then a `RecoveryStampRecord` is persisted at `(run, last_seq)` by the
///      recovery path itself,
/// And the persisted stamp survives a journal close/reopen cycle.
#[test]
fn recover_full_journal_writes_recovery_stamp_and_persists_across_reopen() {
    use crate::recovery::{ActionReplayTracker, recover_full_journal};

    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(0x5C0);
    let workflow = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);
    let last_seq = EventSeq::new(2);

    // Phase 1: open the journal, write a small event sequence, run the
    //          production recovery path, and verify the stamp is written.
    {
        let mut journal = FjallJournal::open(&path, None).expect("open should succeed");

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("append RunAccepted should succeed");
        journal
            .append_journaled(&JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: workflow,
                granted_capabilities: vb_core::CapabilitySet::empty(),
                policy: vb_core::RuntimePolicy::Relaxed,
            })
            .expect("append RunAdmission should succeed");
        journal
            .append_journaled(&JournalEvent::RunFinished {
                run,
                seq: last_seq,
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("append RunFinished should succeed");

        // No stamp should exist before recovery runs.
        let pre = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp pre-read must succeed");
        assert!(
            pre.is_none(),
            "no recovery_stamp may exist before recover_full_journal runs"
        );

        // Run the production recovery path. This is the call site that
        // previous beads wired the stamp writes into.
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .expect("recover_full_journal must succeed");
        assert_eq!(
            replayed.len(),
            3,
            "all three events (RunAccepted, RunAdmission, RunFinished) must be replayed"
        );

        // The recovery path must have written a stamp at (run, last_seq).
        let stamp = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp post-recovery must succeed")
            .expect("recover_full_journal must persist a RecoveryStampRecord at (run, last_seq)");
        assert_eq!(stamp.run, run, "stamp must reference the recovered run");
        assert_eq!(
            stamp.last_seq, last_seq,
            "stamp must reference the last replayed sequence number"
        );
        // written_at_ms is derived from SystemTime::now(); assert it is a
        // recent (post-2023) timestamp to prove it is not a sentinel.
        assert!(
            stamp.written_at_ms > 1_700_000_000_000,
            "stamp.written_at_ms must be a recent unix-millis timestamp, got {}",
            stamp.written_at_ms
        );

        // Simulate a crash: close the journal explicitly so the durability
        // barrier is enforced before the process exits.
        journal.close().expect("close must succeed");
    }

    // Phase 2: reopen the journal, read the stamp back, and confirm it
    //          matches what the recovery path wrote. This proves the stamp
    //          is durable across a process restart boundary.
    {
        let journal = FjallJournal::open(&path, None).expect("reopen should succeed");
        let loaded = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp after reopen must succeed")
            .expect("recovery_stamp must survive close/reopen");
        assert_eq!(
            loaded.run, run,
            "reopened stamp must reference the recovered run"
        );
        assert_eq!(
            loaded.last_seq, last_seq,
            "reopened stamp must reference the last replayed sequence number"
        );
        assert!(
            loaded.written_at_ms > 1_700_000_000_000,
            "reopened stamp must carry the original recent timestamp"
        );
    }
}

/// Given a `FjallJournal` that contains a small run sequence and NO prior
///      `RecoveryStampRecord` for that run,
/// When `recover_full_journal` is invoked for the first time,
/// Then the production recovery path itself writes a stamp at
///      `(run, last_seq)`, and the stamp survives a close/reopen cycle.
/// And when `recover_full_journal` is invoked a SECOND time on the same
///      journal, the existing stamp is preserved (its timestamp is
///      unchanged), proving the `is_none()` branch in the production
///      recovery path skips re-writing the stamp.
///
/// This test deliberately does NOT use `FjallJournal::put_recovery_stamp`
/// or `FjallJournal::get_recovery_stamp` as setup helpers, because doing so
/// would let a future mutation that simply removes the `put_recovery_stamp`
/// call from `recover_full_journal` go undetected: a planted stamp would
/// still appear "preserved" even if production code never wrote it. By
/// requiring the FIRST call to `recover_full_journal` to be the one that
/// creates the stamp, the test fails the moment the production wire-up is
/// removed (Phase 1 would observe `None` from `get_recovery_stamp`).
#[test]
fn recover_full_journal_preserves_existing_recovery_stamp_on_replay() {
    use crate::recovery::{ActionReplayTracker, recover_full_journal};

    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let path = temp.path().to_path_buf();
    let run = RunId::new(0xD03);
    let workflow = WorkflowDigest::from_bytes([0x7E; DIGEST_BYTES]);
    let last_seq = EventSeq::new(2);

    let first_stamp_written_at_ms: u64;

    // Phase 1: open the journal, append the run's events, then call the
    //          production `recover_full_journal` for the FIRST time. The
    //          recovery path must write a stamp at `(run, last_seq)`.
    //          Capture the stamp's `written_at_ms` so Phase 2 can prove the
    //          second call did not overwrite it.
    {
        let mut journal = FjallJournal::open(&path, None).expect("open should succeed");

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("append RunAccepted should succeed");
        journal
            .append_journaled(&JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: workflow,
                granted_capabilities: vb_core::CapabilitySet::empty(),
                policy: vb_core::RuntimePolicy::Relaxed,
            })
            .expect("append RunAdmission should succeed");
        journal
            .append_journaled(&JournalEvent::RunFinished {
                run,
                seq: last_seq,
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("append RunFinished should succeed");

        // Pre-condition: no stamp may exist before the first recovery call.
        // If a prior run left a stamp at this (run, last_seq) it would
        // short-circuit Phase 1's write path; assert absence so Phase 1 is
        // forced to depend on the production wire-up.
        let pre = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp pre-read must succeed");
        assert!(
            pre.is_none(),
            "no recovery_stamp may exist before the first recover_full_journal call"
        );

        // First production recovery call. This MUST persist a stamp at
        // (run, last_seq). If the production `put_recovery_stamp` call is
        // removed, the post-recovery assertion below will fail.
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .expect("first recover_full_journal must succeed");
        assert_eq!(
            replayed.len(),
            3,
            "all three events must be replayed on the first recovery call"
        );

        let stamp = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp post-recovery must succeed")
            .expect(
                "recover_full_journal must persist a RecoveryStampRecord at (run, last_seq); \
                 if this fails, the production put_recovery_stamp call has been removed",
            );
        assert_eq!(
            stamp.run, run,
            "first-call stamp must reference the recovered run"
        );
        assert_eq!(
            stamp.last_seq, last_seq,
            "first-call stamp must reference the last replayed sequence number"
        );
        // The timestamp must be a real wall-clock value, not a sentinel.
        assert!(
            stamp.written_at_ms > 1_700_000_000_000,
            "first-call stamp.written_at_ms must be a recent unix-millis timestamp, got {}",
            stamp.written_at_ms
        );
        first_stamp_written_at_ms = stamp.written_at_ms;

        // Explicit close so the durability barrier is enforced before the
        // process exits the Phase 1 block, matching the close-then-reopen
        // pattern used by sibling tests (e.g. Test 1 above and the prior
        // recovery_stamp_survives_close_and_reopen BDD at line 2551).
        journal.close().expect("close must succeed");
    }

    // Phase 2: reopen the journal, then call `recover_full_journal` a
    //          SECOND time. The stamp written by Phase 1 must be preserved
    //          (its `written_at_ms` must still equal `first_stamp_written_at_ms`).
    {
        let journal = FjallJournal::open(&path, None).expect("reopen should succeed");

        // Pre-condition: the stamp written in Phase 1 must have survived
        // the close/reopen cycle. This pins durability as well as the
        // production wire-up.
        let reopened_stamp = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp after reopen must succeed")
            .expect("stamp written by the first recovery call must survive close/reopen");
        assert_eq!(
            reopened_stamp.written_at_ms, first_stamp_written_at_ms,
            "stamp timestamp must be unchanged across close/reopen"
        );

        // Second production recovery call. The existing stamp must NOT be
        // overwritten: the `is_none()` branch in the production code must
        // skip the `put_recovery_stamp` call. If a regression made the
        // production code always call `put_recovery_stamp`, the timestamp
        // would be bumped to a fresh wall-clock value and this assertion
        // would catch it (we captured the exact T1 above).
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .expect("second recover_full_journal must succeed");
        assert_eq!(
            replayed.len(),
            3,
            "all three events must still be replayed on the second recovery call"
        );

        let preserved_stamp = journal
            .get_recovery_stamp(run, last_seq)
            .expect("get_recovery_stamp after second recovery must succeed")
            .expect("stamp must still exist after the second recovery call");
        assert_eq!(
            preserved_stamp.written_at_ms, first_stamp_written_at_ms,
            "second recovery call must NOT overwrite the existing stamp's timestamp \
             (the is_none branch must skip the put_recovery_stamp call)"
        );
        assert_eq!(
            preserved_stamp.run, run,
            "preserved stamp must still reference the recovered run"
        );
        assert_eq!(
            preserved_stamp.last_seq, last_seq,
            "preserved stamp must still reference the last replayed sequence number"
        );
    }
}

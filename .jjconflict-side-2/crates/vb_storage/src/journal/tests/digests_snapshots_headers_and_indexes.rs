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
    let Some(found) = loaded else {
        panic!("large workflow source should be found");
    };
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
    assert!(
        journal
            .snapshot(run, EventSeq::new(0))
            .expect("get 0")
            .is_some()
    );
    assert!(
        journal
            .snapshot(run, EventSeq::new(5))
            .expect("get 5")
            .is_some()
    );
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
fn append_queued_unfsynced_allows_idempotent_duplicate() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1100);
    let event = make_event(run, 0);
    journal
        .append_queued_unfsynced(&event)
        .expect("first append should succeed");
    let result = journal.append_queued_unfsynced(&event);
    assert!(
        result.is_ok(),
        "idempotent duplicate of same event should succeed, got {:?}",
        result
    );
}

#[test]
fn append_queued_unfsynced_rejects_different_duplicate() {
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
        .append_queued_unfsynced(&event_a)
        .expect("first append should succeed");
    let result = journal.append_queued_unfsynced(&event_b);
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
        assert!(item.key().is_ok());
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
        assert!(item.key().is_ok());
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
        assert!(item.key().is_ok());
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

    let ir = b"batch ir".to_vec();
    let ir_digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let ir_record = CompiledIrRecord {
        digest: ir_digest,
        ir,
    };

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
    assert!(
        journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some()
    );
    assert!(journal.compiled_ir(ir_digest).expect("get ir").is_some());
    assert!(journal.run_header(run).expect("get header").is_some());
    let replayed = journal.events_for_run(run).expect("get events");
    assert_eq!(replayed.len(), 1);
    assert!(journal.blob(blob_digest).expect("get blob").is_some());
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
    assert!(journal.run_header(run).expect("get").is_some());
}


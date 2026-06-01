#![forbid(unsafe_code)]
//! Integration tests for Fjall table contract-pack runner.
//!
//! These tests verify the contract behaviors of Fjall-backed storage operations
//! including run_event append/read patterns, blob storage contracts, and
//! keyspace range operations.
//!
//! Tests are written in failing-first style to establish baseline behavior
//! before implementing the contract-pack runner infrastructure.

use vb_core::{RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};
use vb_storage::{
    BlobRecord, EventSeq, FjallJournal, JournalEvent, RunHeaderRecord, WorkflowSourceRecord,
};

/// Minimum testable run id.
const TEST_RUN: RunId = RunId::new(1);

/// Digest for test artifacts.
const TEST_DIGEST: [u8; 32] = [0xAB; 32];

/// Helper: creates a temporary journal for testing.
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let journal = FjallJournal::open(temp.path(), None).expect("journal should open");
    (temp, journal)
}

fn storage_admissible_workflow() -> vb_core::CompiledWorkflow {
    let yaml = br#"version: velvet-ballistics/v1
name: storage_contract_pack
when:
  manual: {}
steps:
  - id: make
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#;
    let workflow = vb_compile::compile_workflow(yaml).expect("workflow should compile");
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts).expect("workflow parts should encode");
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    vb_core::CompiledWorkflow::try_from_parts(parts).expect("workflow parts should validate")
}

fn submit_storage_artifact(journal: &FjallJournal) -> vb_storage::AcceptedArtifact {
    let workflow = storage_admissible_workflow();
    vb_storage::admission::submit_artifact(journal, &workflow, RuntimePolicy::Journaled)
        .expect("compiled artifact submit should succeed")
}

// ============================================================================
// Happy path: run_event contract
// ============================================================================

/// Contract: run_event append + range read + point read + reopen.
#[test]
fn run_event_contract_append_range_point_reopen() {
    let (temp, journal) = temp_journal();
    let path = temp.path().to_path_buf();

    // Append multiple events for a run.
    let events = vec![
        JournalEvent::RunAccepted {
            run: TEST_RUN,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes(TEST_DIGEST),
        },
        JournalEvent::StepStarted {
            run: TEST_RUN,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: TEST_RUN,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
    ];

    for event in &events {
        journal
            .append_journaled(event)
            .expect("append should succeed");
    }

    // Range read: fetch all events for the run.
    let all_events = journal
        .events_for_run(TEST_RUN)
        .expect("events_for_run should succeed");
    assert_eq!(all_events.len(), 3, "should recover all 3 events");

    // Point read: fetch a specific event by sequence.
    let event_seq_1 = journal
        .events_for_run(TEST_RUN)
        .expect("events_for_run should succeed");
    assert_eq!(
        event_seq_1.get(1).map(|e| e.seq()),
        Some(EventSeq::new(1)),
        "seq 1 event should be at index 1"
    );

    // Reopen the journal and verify persistence.
    drop(journal);
    let reopened = FjallJournal::open(&path, None).expect("reopen should succeed");
    let recovered_events = reopened
        .events_for_run(TEST_RUN)
        .expect("recovered events should succeed");
    assert_eq!(
        recovered_events.len(),
        3,
        "all events should survive reopen"
    );
}

/// Contract: blob write + read + digest check + reopen.
#[test]
fn blob_contract_write_read_digest_reopen() {
    let (temp, journal) = temp_journal();
    let path = temp.path().to_path_buf();

    let blob_bytes = b"test blob payload for contract verification".to_vec();
    let blob_digest: [u8; 32] = blake3::hash(&blob_bytes).into();

    // Write blob.
    let record = BlobRecord {
        digest: blob_digest,
        bytes: blob_bytes.clone(),
    };
    journal.put_blob(&record).expect("blob put should succeed");

    // Read blob back.
    let loaded = journal
        .blob(blob_digest)
        .expect("blob read should succeed")
        .expect("blob should exist after put");
    assert_eq!(loaded.bytes, blob_bytes, "blob content should match");

    // Digest check: reading with wrong digest should return None.
    let wrong_digest: [u8; 32] = [0xFF; 32];
    let result = journal.blob(wrong_digest);
    assert!(
        result.expect("blob lookup should succeed").is_none(),
        "wrong digest should return None"
    );

    // Reopen and verify blob persists.
    drop(journal);
    let reopened = FjallJournal::open(&path, None).expect("reopen should succeed");
    let recovered = reopened
        .blob(blob_digest)
        .expect("blob read after reopen should succeed")
        .expect("blob should exist after reopen");
    assert_eq!(
        recovered.bytes, blob_bytes,
        "recovered blob should match original"
    );
}

// ============================================================================
// Error path: invalid key prefix
// ============================================================================

/// Contract: wrong key prefix returns typed key error.
///
/// This test verifies the contract-pack runner's ability to detect and reject
/// malformed keys. Since FjallJournal doesn't expose raw keyspace access via
/// public API, we test the expected contract behavior through the existing
/// typed APIs: inserting a run header with run_id=0 should be rejected
/// with a typed error since run_id=0 is invalid per the contract schema.
#[test]
fn invalid_key_prefix_returns_typed_error() {
    let (_temp, journal) = temp_journal();

    // The contract-pack runner should validate that run_id=0 is invalid
    // and return a typed error (e.g., InvalidRunId or similar).
    // Currently, run_id=0 returns Ok(None) - this is the failing-first
    // assertion that documents the expected contract behavior.
    let result = journal.run_header(RunId::new(0));

    // This assertion SHOULD fail after contract-pack runner implementation:
    // the contract specifies run_id=0 is invalid and must return a typed error.
    // Currently, this passes (returns Ok(None)) but should return Err.
    assert!(
        result.is_err(),
        "run_id=0 should return a typed error (contract violation), \
         but currently returns Ok(None) - contract-pack runner must enforce run_id validity"
    );
}

/// Contract: missing compiled IR digest returns Ok(None).
///
/// This test verifies that querying a non-existent compiled IR digest
/// returns Ok(None), not an error. The storage layer does not enforce
/// artifact verification - that is a higher-level concern.
#[test]
fn missing_compiled_ir_digest_returns_none() {
    let (_temp, journal) = temp_journal();

    // Store a valid accepted artifact under a different digest first.
    let artifact = submit_storage_artifact(&journal);
    assert_ne!(artifact.digest, WorkflowDigest::from_bytes([0xCC; 32]));

    // Query with a different (non-existent) digest.
    let missing_digest = WorkflowDigest::from_bytes([0xCC; 32]);
    let result = journal.compiled_ir(missing_digest);

    // Missing artifacts return Ok(None) - this is the correct behavior
    // for the storage layer. Artifact verification is a higher-level concern.
    assert!(
        result.expect("compiled_ir lookup should succeed").is_none(),
        "missing digest should return Ok(None), not an error"
    );
}

// ============================================================================
// Edge case: empty keyspace range
// ============================================================================

/// Contract: empty keyspace range returns no rows.
#[test]
fn empty_keyspace_range_returns_no_rows() {
    let (_temp, journal) = temp_journal();

    // Query events for a run that has no events.
    let empty_run = RunId::new(99999);
    let events = journal
        .events_for_run(empty_run)
        .expect("events_for_run should succeed for empty run");
    assert!(events.is_empty(), "empty run should return no events");

    // Query blobs with a digest that doesn't exist.
    let missing_blob_digest: [u8; 32] = [0xAA; 32];
    let blob_result = journal.blob(missing_blob_digest);
    assert!(
        blob_result.expect("blob lookup should succeed").is_none(),
        "missing blob should return None"
    );

    // Query compiled_ir with non-existent digest.
    let missing_ir_digest = WorkflowDigest::from_bytes([0xDD; 32]);
    let ir_result = journal.compiled_ir(missing_ir_digest);
    assert!(
        ir_result
            .expect("compiled_ir lookup should succeed")
            .is_none(),
        "missing compiled_ir should return None"
    );

    // Query run_header for non-existent run.
    let missing_run = RunId::new(88888);
    let header_result = journal.run_header(missing_run);
    assert!(
        header_result
            .expect("run_header lookup should succeed")
            .is_none(),
        "missing run_header should return None"
    );
}

/// Contract: delete or retention boundary removes expected keys only.
/// Contract: delete/trim boundary removes expected keys only.
///
/// Verifies that deleting blob2 via `trim_blob` only affects blob2
/// while preserving blob1 and blob3.
#[test]
fn delete_retention_boundary_removes_expected_keys_only() {
    let (temp, journal) = temp_journal();

    // Write multiple blobs with different digests.
    let blob1_bytes = b"blob one".to_vec();
    let blob1_digest: [u8; 32] = blake3::hash(&blob1_bytes).into();
    let blob2_bytes = b"blob two".to_vec();
    let blob2_digest: [u8; 32] = blake3::hash(&blob2_bytes).into();
    let blob3_bytes = b"blob three".to_vec();
    let blob3_digest: [u8; 32] = blake3::hash(&blob3_bytes).into();

    journal
        .put_blob(&BlobRecord {
            digest: blob1_digest,
            bytes: blob1_bytes.clone(),
        })
        .expect("blob1 put should succeed");
    journal
        .put_blob(&BlobRecord {
            digest: blob2_digest,
            bytes: blob2_bytes.clone(),
        })
        .expect("blob2 put should succeed");
    journal
        .put_blob(&BlobRecord {
            digest: blob3_digest,
            bytes: blob3_bytes.clone(),
        })
        .expect("blob3 put should succeed");

    // Verify all three exist.
    assert!(
        journal
            .blob(blob1_digest)
            .expect("blob1 lookup should succeed")
            .is_some(),
        "blob1 should exist"
    );
    assert!(
        journal
            .blob(blob2_digest)
            .expect("blob2 lookup should succeed")
            .is_some(),
        "blob2 should exist"
    );
    assert!(
        journal
            .blob(blob3_digest)
            .expect("blob3 lookup should succeed")
            .is_some(),
        "blob3 should exist"
    );

    // Delete blob2 using trim_blob - deletion should only affect blob2.
    let trim_result = journal.trim_blob(blob2_digest);
    assert!(
        trim_result.is_ok(),
        "trim_blob should succeed for existing blob"
    );

    // Verify blob2 is deleted but blob1 and blob3 are unaffected.
    let blob2_lookup = journal.blob(blob2_digest);
    assert!(
        blob2_lookup.expect("blob2 lookup should succeed").is_none(),
        "blob2 should be deleted after trim_blob"
    );
    assert!(
        journal
            .blob(blob1_digest)
            .expect("blob1 lookup should succeed")
            .is_some(),
        "blob1 should be unaffected by blob2 deletion"
    );
    assert!(
        journal
            .blob(blob3_digest)
            .expect("blob3 lookup should succeed")
            .is_some(),
        "blob3 should be unaffected by blob2 deletion"
    );

    // Reopen and verify deletion is durable.
    drop(journal);
    let reopened = FjallJournal::open(temp.path(), None).expect("reopen should succeed");
    assert!(
        reopened
            .blob(blob1_digest)
            .expect("blob1 lookup after reopen should succeed")
            .is_some(),
        "blob1 should persist after reopen"
    );
    assert!(
        reopened
            .blob(blob2_digest)
            .expect("blob2 lookup after reopen should succeed")
            .is_none(),
        "blob2 should remain deleted after reopen"
    );
    assert!(
        reopened
            .blob(blob3_digest)
            .expect("blob3 lookup after reopen should succeed")
            .is_some(),
        "blob3 should persist after reopen"
    );
}

// ============================================================================
// Contract-pack runner specific tests
// ============================================================================

/// Contract-pack runner: range scan over events with mixed prefixes.
#[test]
fn range_scan_detects_mixed_prefix_keys() {
    let (_temp, journal) = temp_journal();

    // Write events for two different runs.
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        })
        .expect("run_a accepted should succeed");
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        })
        .expect("run_b accepted should succeed");

    // Query events for run_a only.
    let events_a = journal
        .events_for_run(run_a)
        .expect("events_for_run run_a should succeed");
    assert_eq!(events_a.len(), 1, "run_a should have exactly 1 event");
    assert_eq!(events_a[0].run_id(), run_a, "event should belong to run_a");

    // Query events for run_b only.
    let events_b = journal
        .events_for_run(run_b)
        .expect("events_for_run run_b should succeed");
    assert_eq!(events_b.len(), 1, "run_b should have exactly 1 event");
    assert_eq!(events_b[0].run_id(), run_b, "event should belong to run_b");
}

/// Contract-pack runner: compiled IR artifact verification.
#[test]
fn compiled_ir_artifact_verification_contract() {
    let (_temp, journal) = temp_journal();

    // Write a compiled IR artifact through public accepted-artifact admission.
    let artifact = submit_storage_artifact(&journal);
    let ir_digest = artifact.digest;
    let accepted_envelope = postcard::to_allocvec(&artifact).expect("artifact should encode");

    // Retrieve and verify.
    let retrieved = journal
        .compiled_ir(ir_digest)
        .expect("compiled_ir lookup should succeed")
        .expect("compiled_ir should exist after put");
    assert_eq!(
        retrieved.ir, accepted_envelope,
        "retrieved IR should match accepted-artifact envelope"
    );
    assert_eq!(
        retrieved.digest, ir_digest,
        "retrieved digest should match original"
    );

    // Verify with wrong digest should not find the artifact.
    let tampered_digest = WorkflowDigest::from_bytes([0x11; 32]);
    let tampered_result = journal.compiled_ir(tampered_digest);
    assert!(
        tampered_result.expect("lookup should succeed").is_none(),
        "tampered digest should not find artifact"
    );
}

/// Contract-pack runner: workflow source artifact verification.
#[test]
fn workflow_source_artifact_verification_contract() {
    let (_temp, journal) = temp_journal();

    // Write a workflow source artifact.
    let source_content = b"version: velvet-ballistics/v1\nname: test".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source_content).into());

    let source_record = WorkflowSourceRecord {
        digest: source_digest,
        source: source_content.clone(),
    };
    journal
        .put_workflow_source(&source_record)
        .expect("workflow source put should succeed");

    // Retrieve and verify.
    let retrieved = journal
        .workflow_source(source_digest)
        .expect("workflow_source lookup should succeed")
        .expect("workflow_source should exist after put");
    assert_eq!(
        retrieved.source, source_content,
        "retrieved source should match original"
    );
    assert_eq!(
        retrieved.digest, source_digest,
        "retrieved digest should match original"
    );
}

/// Contract-pack runner: run header persistence contract.
#[test]
fn run_header_persistence_contract() {
    let (temp, journal) = temp_journal();

    // Write a run header.
    let run_id = RunId::new(42);
    let workflow_id = WorkflowId::new(7);
    let compiled_digest = WorkflowDigest::from_bytes([0x99; 32]);

    let header = RunHeaderRecord {
        run: run_id,
        workflow_id,
        compiled_digest,
        status: 1,
        accepted_at_ms: 1234567890,
    };
    journal
        .put_run_header(&header)
        .expect("run_header put should succeed");

    // Retrieve and verify.
    let retrieved = journal
        .run_header(run_id)
        .expect("run_header lookup should succeed")
        .expect("run_header should exist after put");
    assert_eq!(retrieved.run, run_id, "run id should match");
    assert_eq!(
        retrieved.workflow_id, workflow_id,
        "workflow_id should match"
    );
    assert_eq!(
        retrieved.compiled_digest, compiled_digest,
        "compiled_digest should match"
    );

    // Reopen and verify persistence.
    drop(journal);
    let reopened = FjallJournal::open(temp.path(), None).expect("reopen should succeed");
    let recovered = reopened
        .run_header(run_id)
        .expect("run_header lookup after reopen should succeed")
        .expect("run_header should exist after reopen");
    assert_eq!(recovered.run, run_id, "recovered run id should match");
    assert_eq!(
        recovered.accepted_at_ms, 1234567890,
        "recovered timestamp should match"
    );
}

/// Contract-pack runner: multiple runs event isolation.
///
/// This test verifies that events for different runs are properly isolated
/// and do not contaminate each other. Each run's events must be independent
/// and retrievable without interference from other runs.
#[test]
fn multiple_runs_event_isolation_contract() {
    let (_temp, journal) = temp_journal();

    let run_1 = RunId::new(1001);
    let run_2 = RunId::new(1002);
    let run_3 = RunId::new(1003);

    // Append events to each run - each run starts at seq 0 per journal contract.
    // Run 1: seq 0, 1
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: run_1,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x01; 32]),
        })
        .expect("run_1 accepted should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run: run_1,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("run_1 step started should succeed");

    // Run 2: seq 0, 1
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: run_2,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x02; 32]),
        })
        .expect("run_2 accepted should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run: run_2,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("run_2 step started should succeed");

    // Run 3: seq 0, 1
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: run_3,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x03; 32]),
        })
        .expect("run_3 accepted should succeed");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run: run_3,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        })
        .expect("run_3 step started should succeed");

    // Verify each run has exactly 2 events, isolated from others.
    for run in [run_1, run_2, run_3] {
        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2, "each run should have exactly 2 events");
    }

    // Verify cross-run isolation: run_1 should not see run_2's events.
    let run_1_events = journal
        .events_for_run(run_1)
        .expect("events_for_run run_1 should succeed");
    let run_2_events = journal
        .events_for_run(run_2)
        .expect("events_for_run run_2 should succeed");
    let run_3_events = journal
        .events_for_run(run_3)
        .expect("events_for_run run_3 should succeed");

    // Each run's first event should be at seq 0
    assert_eq!(
        run_1_events.get(0).map(|e| e.seq()),
        Some(EventSeq::new(0)),
        "run_1 first event should be at seq 0"
    );
    assert_eq!(
        run_2_events.get(0).map(|e| e.seq()),
        Some(EventSeq::new(0)),
        "run_2 first event should be at seq 0"
    );
    assert_eq!(
        run_3_events.get(0).map(|e| e.seq()),
        Some(EventSeq::new(0)),
        "run_3 first event should be at seq 0"
    );

    // Verify runs are isolated - different runs have different workflow digests.
    // Extract workflow digest via pattern matching on RunAccepted variant.
    let run_1_digest = match &run_1_events[0] {
        JournalEvent::RunAccepted { workflow, .. } => *workflow,
        other => panic!(
            "first event of run_1 should be RunAccepted, got {:?}",
            other
        ),
    };
    let run_2_digest = match &run_2_events[0] {
        JournalEvent::RunAccepted { workflow, .. } => *workflow,
        other => panic!(
            "first event of run_2 should be RunAccepted, got {:?}",
            other
        ),
    };
    let run_3_digest = match &run_3_events[0] {
        JournalEvent::RunAccepted { workflow, .. } => *workflow,
        other => panic!(
            "first event of run_3 should be RunAccepted, got {:?}",
            other
        ),
    };

    assert_ne!(
        run_1_digest, run_2_digest,
        "run_1 and run_2 should have different workflow digests"
    );
    assert_ne!(
        run_1_digest, run_3_digest,
        "run_1 and run_3 should have different workflow digests"
    );
    assert_ne!(
        run_2_digest, run_3_digest,
        "run_2 and run_3 should have different workflow digests"
    );
}

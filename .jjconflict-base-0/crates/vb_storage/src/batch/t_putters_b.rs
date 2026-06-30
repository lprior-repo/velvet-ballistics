#![forbid(unsafe_code)]
use super::*;

#[test]
fn batch_mixed_operations_across_keyspaces_commit_atomically() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(600);
    let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

    let source = b"batch mixed ops source".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let workflow_record = WorkflowSourceRecord {
        digest: source_digest,
        source,
    };

    let ir = b"batch mixed ops ir".to_vec();
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
        accepted_at_ms: 9999,
    };

    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_source(&workflow_record)
        .expect("workflow source");
    batch.put_compiled_ir(&ir_record).expect("compiled ir");
    batch.put_run_header(&header).expect("run header");
    batch.append_event(&event).expect("event");
    batch
        .put_status_index(IndexStatusState::Active, 100, run)
        .expect("status index");
    batch
        .put_workflow_index(WorkflowId::new(42), run)
        .expect("workflow index");
    batch
        .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
        .expect("action index");

    assert_eq!(batch.len(), 7, "batch should track 7 operations");
    batch.commit().expect("mixed batch commit should succeed");

    assert!(
        journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some(),
        "workflow source should exist"
    );
    assert!(
        journal.compiled_ir(ir_digest).expect("get ir").is_some(),
        "compiled IR should exist"
    );
    assert!(
        journal.run_header(run).expect("get header").is_some(),
        "run header should exist"
    );
    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(events.len(), 1, "should have 1 event");
}

#[test]
fn batch_put_workflow_source_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let source = b"real content".to_vec();
    let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
    let record = WorkflowSourceRecord {
        digest: wrong_digest,
        source,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch must reject digest mismatch, got {:?}",
        result
    );
    assert_eq!(batch.len(), 0);
}

#[test]
fn batch_put_blob_rejects_digest_mismatch() {
    let (_temp, journal) = temp_journal();
    let payload = vec![1, 2, 3];
    let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; DIGEST_BYTES];
    let record = BlobRecord {
        digest: wrong_digest,
        bytes: payload,
    };

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch must reject blob digest mismatch, got {:?}",
        result
    );
    assert_eq!(batch.len(), 0);
}

#[test]
fn batch_index_operations_increment_len_without_payloads() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let wf = WorkflowId::new(10);
    let action = vb_core::ActionId::new(20);
    let step = StepIdx::new(1);

    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_status_index(IndexStatusState::Completed, 5000, run)
        .expect("status idx");
    batch.put_workflow_index(wf, run).expect("workflow idx");
    batch
        .put_action_index(action, run, step)
        .expect("action idx");
    assert_eq!(batch.len(), 3, "three index operations should yield len 3");
    assert!(!batch.is_empty());
    batch.commit().expect("index batch commit should succeed");

    let mut status_count = 0usize;
    for item in journal.index_status.iter() {
        let _ = item.key();
        status_count = status_count.saturating_add(1);
    }
    assert_eq!(status_count, 1, "should have 1 status index marker");

    let mut wf_count = 0usize;
    for item in journal.index_workflow.iter() {
        let _ = item.key();
        wf_count = wf_count.saturating_add(1);
    }
    assert_eq!(wf_count, 1, "should have 1 workflow index marker");

    let mut action_count = 0usize;
    for item in journal.index_action.iter() {
        let _ = item.key();
        action_count = action_count.saturating_add(1);
    }
    assert_eq!(action_count, 1, "should have 1 action index marker");
}

#[test]
fn batch_put_compiled_ir_commits_and_is_readable() {
    let (_temp, journal) = temp_journal();
    let ir = b"compiled-artifact-bytes".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: ir.clone(),
    };

    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_compiled_ir(&record).expect("batch compiled ir");
    batch.commit().expect("commit should succeed");

    let loaded = journal.compiled_ir(digest).expect("get should succeed");
    let Some(found) = loaded else {
        panic!("compiled IR should be found after batch commit");
    };
    assert_eq!(found.ir, ir);
}

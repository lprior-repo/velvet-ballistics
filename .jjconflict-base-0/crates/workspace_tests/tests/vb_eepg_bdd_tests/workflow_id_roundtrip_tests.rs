use super::*;

#[test]
fn index_workflow_key_preserves_workflow_id_bytes() -> Result<(), JournalError> {
    // Given: a WorkflowId constructed from any u32 value and a RunId
    let workflow_id = WorkflowId::new(0x1234_5678);
    let run_id = RunId::new(0xAABB_CCDD_EEFF_0011);

    // When: index_workflow_key is called
    let key = keys::index_workflow_key(workflow_id, run_id)?;

    // Then: the resulting key bytes are [0x31][workflow_u32_be][run_u64_be]
    assert_eq!(
        key[0],
        vb_storage::constants::PREFIX_INDEX_WORKFLOW,
        "prefix must be 0x31"
    );
    assert_eq!(
        &key[1..5],
        workflow_id.get().to_be_bytes(),
        "workflow_id bytes must be preserved in big-endian"
    );
    assert_eq!(
        &key[5..13],
        run_id.get().to_be_bytes(),
        "run_id bytes must be preserved in big-endian"
    );

    // And: each field decodes to its original value
    let recovered_workflow = WorkflowId::new(u32::from_be_bytes(key[1..5].try_into().unwrap()));
    let recovered_run_id = RunId::new(u64::from_be_bytes(key[5..13].try_into().unwrap()));
    assert_eq!(recovered_workflow, workflow_id);
    assert_eq!(recovered_run_id, run_id);
    Ok(())
}

#[test]
fn index_workflow_key_lexicographic_ordering() -> Result<(), JournalError> {
    // Given: two WorkflowIds w1 < w2 with the same RunId
    let run_id = RunId::new(42);
    let w1 = WorkflowId::new(100);
    let w2 = WorkflowId::new(200);

    // When: index_workflow_key is called for both
    let key1 = keys::index_workflow_key(w1, run_id)?;
    let key2 = keys::index_workflow_key(w2, run_id)?;

    // Then: the resulting keys maintain the same ordering lexicographically
    assert!(
        key1 < key2,
        "key(w1) must be less than key(w2) when w1 < w2"
    );
    Ok(())
}

#[test]
fn index_workflow_key_zero_values() -> Result<(), JournalError> {
    // Given: WorkflowId::new(0) and RunId::new(0)
    let workflow_id = WorkflowId::new(0);
    let run_id = RunId::new(0);

    // When: index_workflow_key is called
    let result = keys::index_workflow_key(workflow_id, run_id);

    // Then: the encoder rejects RunId::new(0) with
    // `JournalError::InvalidRunId { run: RunId::new(0) }` because
    // zero RunId is reserved (PO-cn2v4-001). The previous
    // expectation that zero bytes round-trip is no longer valid;
    // see `index_workflow_key_zero_workflow_id` for the
    // zero-workflow-but-non-zero-run coverage.
    match result {
        Err(JournalError::InvalidRunId { run }) => {
            assert_eq!(run, RunId::new(0), "the run must round-trip back as zero");
        }
        other => panic!(
            "expected JournalError::InvalidRunId for zero RunId, got {:?}",
            other
        ),
    }
    Ok(())
}

#[test]
fn index_workflow_key_zero_workflow_id() -> Result<(), JournalError> {
    // Companion that pins the (workflow=0, run=nonzero) byte
    // layout the previous test used to cover.
    let workflow_id = WorkflowId::new(0);
    let run_id = RunId::new(1);
    let key = keys::index_workflow_key(workflow_id, run_id)?;
    assert_eq!(
        &key[1..5],
        0u32.to_be_bytes(),
        "zero workflow_id must encode as all-zero bytes"
    );
    assert_eq!(
        &key[5..13],
        1u64.to_be_bytes(),
        "non-zero run_id bytes must encode as big-endian u64"
    );
    assert_eq!(key.len(), 13, "index_workflow_key must be 13 bytes");
    Ok(())
}

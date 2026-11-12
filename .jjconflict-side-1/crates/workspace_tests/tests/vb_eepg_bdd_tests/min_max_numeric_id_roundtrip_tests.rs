use super::*;

#[test]
fn run_id_zero_roundtrip() -> Result<(), JournalError> {
    // Given: RunId::new(0)
    let run_id = RunId::new(0);

    // When: run_header_key is called
    let result = keys::run_header_key(run_id);

    // Then: the encoder rejects RunId::new(0) per the
    // `InvalidRunId` contract (PO-cn2v4-001). The original
    // roundtrip expectation is no longer valid because zero is
    // a reserved sentinel on both the encode and decode paths.
    match result {
        Err(JournalError::InvalidRunId { run }) => {
            assert_eq!(run, RunId::new(0));
        }
        other => panic!(
            "expected JournalError::InvalidRunId for zero RunId, got {:?}",
            other
        ),
    }
    Ok(())
}

#[test]
fn run_id_nonzero_roundtrip() -> Result<(), JournalError> {
    // Companion to `run_id_zero_roundtrip` that covers the
    // big-endian 8-byte layout for a non-zero RunId.
    let run_id = RunId::new(42);
    let key = keys::run_header_key(run_id)?;
    let decoded = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));
    assert_eq!(decoded, run_id, "non-zero RunId must roundtrip correctly");
    Ok(())
}

#[test]
fn run_id_max_roundtrip() -> Result<(), JournalError> {
    // Given: RunId::new(u64::MAX)
    let run_id = RunId::new(u64::MAX);

    // When: run_header_key is called then decoded
    let key = keys::run_header_key(run_id)?;

    // Then: the decoded RunId equals the original
    let decoded = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));
    assert_eq!(decoded, run_id, "max RunId must roundtrip correctly");
    Ok(())
}

#[test]
fn workflow_id_extents_roundtrip() -> Result<(), JournalError> {
    // Given: WorkflowId::new(0) and WorkflowId::new(u32::MAX)
    let run_id = RunId::new(42);
    let min_workflow = WorkflowId::new(0);
    let max_workflow = WorkflowId::new(u32::MAX);

    // When: index_workflow_key is called for each then decoded
    let key_min = keys::index_workflow_key(min_workflow, run_id)?;
    let key_max = keys::index_workflow_key(max_workflow, run_id)?;

    // Then: each decoded WorkflowId equals its original
    let decoded_min = WorkflowId::new(u32::from_be_bytes(key_min[1..5].try_into().unwrap()));
    let decoded_max = WorkflowId::new(u32::from_be_bytes(key_max[1..5].try_into().unwrap()));

    assert_eq!(
        decoded_min, min_workflow,
        "min WorkflowId must roundtrip correctly"
    );
    assert_eq!(
        decoded_max, max_workflow,
        "max WorkflowId must roundtrip correctly"
    );
    Ok(())
}

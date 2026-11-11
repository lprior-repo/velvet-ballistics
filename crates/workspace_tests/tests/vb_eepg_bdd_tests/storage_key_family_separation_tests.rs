use super::*;

#[test]
fn all_key_prefixes_are_distinct() -> Result<(), JournalError> {
    // Given: all key constructor functions with fixed inputs
    let run_id = RunId::new(1);
    let seq = EventSeq::new(1);
    let workflow_id = WorkflowId::new(1);
    let action_id = ActionId::new(1);
    let step_idx = StepIdx::new(1);
    let digest = [0u8; 32];

    // When: each is called and the first byte (prefix) is extracted
    let prefixes = [
        keys::run_header_key(run_id)?[0],
        keys::run_event_key(run_id, seq)?[0],
        keys::run_snapshot_key(run_id, seq)?[0],
        keys::workflow_source_key(digest)?[0],
        keys::compiled_ir_key(digest)?[0],
        keys::blob_key(digest)?[0],
        keys::index_status_key(vb_storage::types::IndexStatusState::Submitted, 0, run_id)?[0],
        keys::index_workflow_key(workflow_id, run_id)?[0],
        keys::index_action_key(action_id, run_id, step_idx)?[0],
    ];

    // Then: all prefixes are pairwise distinct
    for i in 0..prefixes.len() {
        for j in (i + 1)..prefixes.len() {
            assert_ne!(
                prefixes[i], prefixes[j],
                "prefix at index {} ({:#04x}) must differ from prefix at index {} ({:#04x})",
                i, prefixes[i], j, prefixes[j]
            );
        }
    }
    Ok(())
}

#[test]
fn run_family_prefixes_distinct() -> Result<(), JournalError> {
    // Given: run_header_key, run_event_key, run_snapshot_key
    let run_id = RunId::new(1);
    let seq = EventSeq::new(1);

    // When: each is called with valid inputs
    let header_prefix = keys::run_header_key(run_id)?[0];
    let event_prefix = keys::run_event_key(run_id, seq)?[0];
    let snapshot_prefix = keys::run_snapshot_key(run_id, seq)?[0];

    // Then: the three prefixes are 0x10, 0x11, 0x12 respectively
    // And: no two are equal
    assert_eq!(header_prefix, 0x10, "run_header prefix must be 0x10");
    assert_eq!(event_prefix, 0x11, "run_event prefix must be 0x11");
    assert_eq!(snapshot_prefix, 0x12, "run_snapshot prefix must be 0x12");
    assert_ne!(
        header_prefix, event_prefix,
        "header and event prefixes must differ"
    );
    assert_ne!(
        header_prefix, snapshot_prefix,
        "header and snapshot prefixes must differ"
    );
    assert_ne!(
        event_prefix, snapshot_prefix,
        "event and snapshot prefixes must differ"
    );
    Ok(())
}

use super::*;

#[test]
fn record_kind_id_is_stable() {
    // Given: RecordKind enum variants
    let variants = [
        RecordKind::WorkflowSource,
        RecordKind::CompiledIr,
        RecordKind::RunHeader,
        RecordKind::RunAccepted,
        RecordKind::StepStarted,
        RecordKind::SlotWritten,
        RecordKind::ActionScheduled,
        RecordKind::ActionCompleted,
        RecordKind::ActionFailed,
        RecordKind::WaitScheduled,
        RecordKind::AskScheduled,
        RecordKind::AskAnswered,
        RecordKind::AskTimedOut,
        RecordKind::RetryScheduled,
        RecordKind::StepFailed,
        RecordKind::RunCancelled,
        RecordKind::RunKilled,
        RecordKind::RunFinished,
        RecordKind::RunFailed,
        RecordKind::RunAdmission,
        RecordKind::RunResumed,
        RecordKind::RunRetried,
        RecordKind::RunAnswered,
        RecordKind::WaitResolved,
        RecordKind::ActionAbandoned,
        RecordKind::Snapshot,
        RecordKind::Blob,
        RecordKind::IndexUpdate,
    ];

    // When: .id() is called on each variant multiple times
    let ids: Vec<u16> = variants.iter().map(|v| v.id()).collect();
    let ids_again: Vec<u16> = variants.iter().map(|v| v.id()).collect();

    // Then: each returns the same u16 value across all invocations
    // And: no two variants share the same ID
    assert_eq!(
        ids, ids_again,
        "RecordKind.id() must be stable across invocations"
    );

    // Check uniqueness using a sorted copy
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    sorted_ids.dedup();
    assert_eq!(
        sorted_ids.len(),
        ids.len(),
        "all RecordKind IDs must be unique"
    );
}

#[test]
fn record_kind_id_values_match_constants() {
    // Given: RecordKind::WorkflowSource
    let kind = RecordKind::WorkflowSource;

    // When: .id() is called
    let id = kind.id();

    // Then: the result equals the wire protocol constant for WorkflowSource
    assert_eq!(id, 1, "WorkflowSource.id() must equal 1");
    assert_eq!(
        RecordKind::CompiledIr.id(),
        2,
        "CompiledIr.id() must equal 2"
    );
    assert_eq!(RecordKind::RunHeader.id(), 3, "RunHeader.id() must equal 3");
    assert_eq!(
        RecordKind::RunAccepted.id(),
        10,
        "RunAccepted.id() must equal 10"
    );
    assert_eq!(
        RecordKind::RunAnswered.id(),
        27,
        "RunAnswered.id() must equal 27"
    );
    assert_eq!(
        RecordKind::RunKilled.id(),
        28,
        "RunKilled.id() must equal 28"
    );
    assert_eq!(
        RecordKind::AskTimedOut.id(),
        29,
        "AskTimedOut.id() must equal 29"
    );
    assert_eq!(RecordKind::Snapshot.id(), 30, "Snapshot.id() must equal 30");
    assert_eq!(
        RecordKind::WaitResolved.id(),
        31,
        "WaitResolved.id() must equal 31"
    );
    assert_eq!(
        RecordKind::ActionAbandoned.id(),
        32,
        "ActionAbandoned.id() must equal 32"
    );
    assert_eq!(RecordKind::Blob.id(), 40, "Blob.id() must equal 40");
    assert_eq!(
        RecordKind::IndexUpdate.id(),
        50,
        "IndexUpdate.id() must equal 50"
    );
}

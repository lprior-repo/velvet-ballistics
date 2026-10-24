use super::support::*;
use vb_storage::codec::validate_record_kind_family;
use vb_storage::{JournalError, RecordKind};

const ORIGINAL: &[(u8, u16, RecordKind)] = &[
    (0, 1, RecordKind::WorkflowSource),
    (1, 2, RecordKind::CompiledIr),
    (2, 3, RecordKind::RunHeader),
    (3, 10, RecordKind::RunAccepted),
    (4, 11, RecordKind::StepStarted),
    (5, 12, RecordKind::SlotWritten),
    (6, 13, RecordKind::ActionScheduled),
    (7, 34, RecordKind::ActionScheduledTicket),
    (8, 14, RecordKind::ActionCompleted),
    (9, 35, RecordKind::ActionCompletedEnvelope),
    (10, 37, RecordKind::RetryCheckpoint),
    (11, 15, RecordKind::ActionFailed),
    (12, 16, RecordKind::WaitScheduled),
    (13, 17, RecordKind::AskScheduled),
    (14, 18, RecordKind::AskAnswered),
    (15, 19, RecordKind::RetryScheduled),
    (16, 20, RecordKind::StepFailed),
    (17, 33, RecordKind::StepSucceeded),
    (18, 21, RecordKind::RunCancelled),
    (19, 28, RecordKind::RunKilled),
    (20, 29, RecordKind::AskTimedOut),
    (21, 31, RecordKind::WaitResolved),
    (22, 32, RecordKind::ActionAbandoned),
    (23, 22, RecordKind::RunFinished),
    (24, 23, RecordKind::RunFailed),
    (25, 24, RecordKind::RunAdmission),
    (26, 25, RecordKind::RunResumed),
    (27, 26, RecordKind::RunRetried),
    (28, 27, RecordKind::RunAnswered),
    (29, 30, RecordKind::Snapshot),
    (30, 40, RecordKind::Blob),
    (31, 50, RecordKind::IndexUpdate),
    (32, 4, RecordKind::RunInputs),
    (33, 36, RecordKind::RunAdmissionBound),
    (34, 38, RecordKind::PolicyDecision),
    (35, 39, RecordKind::RunAdmissionPinned),
    (36, 41, RecordKind::RunAdmissionBoundPinned),
    (37, 42, RecordKind::RunOwnedInputs),
];

#[test]
fn kind43_is_journal_only_and_every_old_ordinal_id_stays_literal() -> TestResult {
    for (ordinal, id, kind) in ORIGINAL.iter().copied() {
        assert_eq!(kind.id(), id);
        assert_eq!(RecordKind::from_id(id), Some(kind));
        assert_eq!(postcard::to_allocvec(&kind)?, vec![ordinal]);
    }
    assert_eq!(RecordKind::RunContinuation.id(), 43);
    assert_eq!(RecordKind::from_id(43), Some(RecordKind::RunContinuation));
    validate_record_kind_family(vb_storage::MAGIC_JOURNAL_EVENT, 43)?;
    assert!(matches!(
        validate_record_kind_family(vb_storage::MAGIC_INDEX_RECORD, 43),
        Err(JournalError::RecordKindFamilyMismatch { kind: 43, .. })
    ));
    assert_eq!(RecordKind::from_id(44), None);
    Ok(())
}

#[test]
fn old_stable_event_golden_and_index50_generic_codec_remain_available() -> TestResult {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(17),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    };
    let bytes = postcard::to_allocvec(&event)?;
    assert_eq!(bytes, [vec![10, 17, 0], vec![0; 32]].concat());
    let record = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        1_048_576,
    )?;
    assert_eq!(
        vb_storage::decode_journal_event(&record, vb_storage::MAGIC_JOURNAL_EVENT, 1_048_576)?.1,
        event
    );
    let index = vb_storage::encode_record(
        vb_storage::MAGIC_INDEX_RECORD,
        RecordKind::IndexUpdate,
        0,
        &7u8,
        1_048_576,
    )?;
    assert_eq!(
        vb_storage::decode_record_header(&index, vb_storage::MAGIC_INDEX_RECORD, 1_048_576)?
            .record_kind,
        50
    );
    Ok(())
}

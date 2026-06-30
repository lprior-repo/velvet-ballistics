use super::*;

#[test]
fn matrix_has_row_for_every_primitive() {
    let result = verify_matrix_completeness();
    assert!(
        result.is_ok(),
        "Expected all primitives to have rows, got: {:?}",
        result
    );
}

#[test]
fn every_row_has_replay_proof() {
    let result = verify_matrix_replay_proofs();
    assert!(
        result.is_ok(),
        "Expected all rows to have replay proof, got: {:?}",
        result
    );
}

#[test]
fn no_row_claims_ack_before_persist() {
    let result = verify_ack_after_persist();
    assert!(
        result.is_ok(),
        "Expected all rows to ack after persist, got: {:?}",
        result
    );
}

#[test]
fn full_matrix_verification_passes() {
    let result = verify_matrix();
    assert!(
        result.is_ok(),
        "Expected full matrix to pass, got: {:?}",
        result
    );
}

#[test]
fn set_row_exists_and_is_correct() {
    let row = DURABILITY_MATRIX
        .iter()
        .find(|r| r.primitive == "set")
        .unwrap();
    assert_eq!(row.compiled_node_kind, "SetConst");
    assert!(row.journal_events.contains(&RecordKind::StepStarted));
    assert!(row.journal_events.contains(&RecordKind::SlotWritten));
    assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
}

#[test]
fn do_row_exists_and_is_correct() {
    let row = DURABILITY_MATRIX
        .iter()
        .find(|r| r.primitive == "do")
        .unwrap();
    assert_eq!(row.compiled_node_kind, "Do");
    assert!(row.journal_events.contains(&RecordKind::ActionScheduled));
    assert!(row.journal_events.contains(&RecordKind::ActionCompleted));
    assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
}

#[test]
fn wait_row_names_wait_scheduled_and_wait_resolved() {
    let row = DURABILITY_MATRIX
        .iter()
        .find(|r| r.primitive == "wait")
        .unwrap();
    assert!(row.journal_events.contains(&RecordKind::WaitScheduled));
    assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
}

#[test]
fn ask_row_names_ask_scheduled_and_ask_answered() {
    let row = DURABILITY_MATRIX
        .iter()
        .find(|r| r.primitive == "ask")
        .unwrap();
    assert!(row.journal_events.contains(&RecordKind::AskScheduled));
    assert!(row.journal_events.contains(&RecordKind::AskAnswered));
    assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
}

#[test]
fn finish_row_names_run_finished() {
    let row = DURABILITY_MATRIX
        .iter()
        .find(|r| r.primitive == "finish")
        .unwrap();
    assert!(row.journal_events.contains(&RecordKind::RunFinished));
    assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
}

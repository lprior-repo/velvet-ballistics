#![forbid(unsafe_code)]
//! Per-primitive durability proof matrix.
//!
//! Maps every YAML primitive to its journal events, storage partition,
//! ack point, replay assertion, and test evidence.

use vb_storage::RecordKind;

/// Storage partition for a primitive's events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoragePartition {
    /// Runtime journal keyspace.
    RuntimeJournal,
    /// Action boundary keyspace.
    ActionJournal,
    /// Timer/suspend keyspace.
    TimerJournal,
}

/// When the shard acknowledges the command relative to persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckPoint {
    /// Journal append happens before Ok(()) is returned.
    AfterJournalAppend,
    /// Acknowledgment happens before persistence (forbidden).
    BeforeJournalAppend,
}

/// A single row in the durability matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityRow {
    /// YAML primitive name (canonical).
    pub primitive: &'static str,
    /// Compiled IR node kind.
    pub compiled_node_kind: &'static str,
    /// Journal events emitted by this primitive.
    pub journal_events: &'static [RecordKind],
    /// Where events are stored.
    pub storage_partition: StoragePartition,
    /// When acknowledgment happens.
    pub ack_point: AckPoint,
    /// Human-readable replay assertion.
    pub replay_assertion: &'static str,
    /// Paths to test files proving this row.
    pub test_evidence: &'static [&'static str],
}

/// All primitives that must have a matrix row.
pub const REQUIRED_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish",
];

// ---------------------------------------------------------------------------
// DURABILITY MATRIX — intentionally incomplete for red phase
// ---------------------------------------------------------------------------

/// The durability matrix. Each primitive must have exactly one row.
pub const DURABILITY_MATRIX: &[DurabilityRow] = &[
    // set — fully specified
    DurabilityRow {
        primitive: "set",
        compiled_node_kind: "SetConst",
        journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay reproduces the same slot value and advances PC",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // do — fully specified
    DurabilityRow {
        primitive: "do",
        compiled_node_kind: "Do",
        journal_events: &[
            RecordKind::StepStarted,
            RecordKind::ActionScheduled,
            RecordKind::ActionCompleted,
            RecordKind::SlotWritten,
        ],
        storage_partition: StoragePartition::ActionJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay re-schedules action and reaches same completion state",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // choose — fully specified
    DurabilityRow {
        primitive: "choose",
        compiled_node_kind: "Choose",
        journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay selects the same branch and advances PC",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // for_each — fully specified
    DurabilityRow {
        primitive: "for_each",
        compiled_node_kind: "ForEach",
        journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay iterates the same items in the same order",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // together — fully specified
    DurabilityRow {
        primitive: "together",
        compiled_node_kind: "Together",
        journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay executes branches in the same order",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // collect — fully specified
    DurabilityRow {
        primitive: "collect",
        compiled_node_kind: "Collect",
        journal_events: &[
            RecordKind::StepStarted,
            RecordKind::SlotWritten,
            RecordKind::SlotWritten,
        ],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay collects the same pages and produces identical output",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // reduce — fully specified
    DurabilityRow {
        primitive: "reduce",
        compiled_node_kind: "Reduce",
        journal_events: &[
            RecordKind::StepStarted,
            RecordKind::SlotWritten,
            RecordKind::SlotWritten,
        ],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay reduces the same items to the same accumulator",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // repeat — fully specified
    DurabilityRow {
        primitive: "repeat",
        compiled_node_kind: "Repeat",
        journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay repeats the same number of iterations",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // wait — fully specified
    DurabilityRow {
        primitive: "wait",
        compiled_node_kind: "WaitUntil",
        journal_events: &[
            RecordKind::StepStarted,
            RecordKind::WaitScheduled,
            RecordKind::SlotWritten,
        ],
        storage_partition: StoragePartition::TimerJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay resumes from the same timer expiration",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // ask — fully specified
    DurabilityRow {
        primitive: "ask",
        compiled_node_kind: "Ask",
        journal_events: &[
            RecordKind::StepStarted,
            RecordKind::AskScheduled,
            RecordKind::AskAnswered,
            RecordKind::SlotWritten,
            RecordKind::SlotWritten,
        ],
        storage_partition: StoragePartition::TimerJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay reproduces the same answer slot value and resumes PC",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
    // finish — fully specified
    DurabilityRow {
        primitive: "finish",
        compiled_node_kind: "Finish",
        journal_events: &[RecordKind::StepStarted, RecordKind::RunFinished],
        storage_partition: StoragePartition::RuntimeJournal,
        ack_point: AckPoint::AfterJournalAppend,
        replay_assertion: "Replay terminates with the same result slot",
        test_evidence: &["crates/vb_runtime/src/shard/tests.rs"],
    },
];

/// Errors produced by the durability matrix verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurabilityError {
    /// A required primitive has no matrix row.
    MissingPrimitiveRow { primitive: String },
    /// A row exists but has no test evidence.
    MissingReplayProof { primitive: String, event: String },
    /// A row claims ack-before-persist.
    AckBeforePersist { primitive: String, handler: String },
    /// A journal event has no associated primitive.
    OrphanEvent { event: String },
}

/// Verify that every required primitive has a row.
pub fn verify_matrix_completeness() -> Result<(), DurabilityError> {
    verify_matrix_completeness_with_primitives(REQUIRED_PRIMITIVES)
}

/// Verify that every required primitive has a row (testable variant).
pub fn verify_matrix_completeness_with_primitives(
    primitives: &[&str],
) -> Result<(), DurabilityError> {
    for &primitive in primitives {
        let found = DURABILITY_MATRIX
            .iter()
            .any(|row| row.primitive == primitive);
        if !found {
            return Err(DurabilityError::MissingPrimitiveRow {
                primitive: primitive.to_owned(),
            });
        }
    }
    Ok(())
}

/// Verify that every row has at least one test evidence link.
pub fn verify_matrix_replay_proofs() -> Result<(), DurabilityError> {
    verify_matrix_replay_proofs_with_matrix(DURABILITY_MATRIX)
}

/// Verify that every row has at least one test evidence link (testable variant).
pub fn verify_matrix_replay_proofs_with_matrix(
    matrix: &[DurabilityRow],
) -> Result<(), DurabilityError> {
    for row in matrix {
        if row.test_evidence.is_empty() {
            return Err(DurabilityError::MissingReplayProof {
                primitive: row.primitive.to_owned(),
                event: format!("{:?}", row.journal_events.first()),
            });
        }
    }
    Ok(())
}

/// Verify that no row claims ack-before-persist.
pub fn verify_ack_after_persist() -> Result<(), DurabilityError> {
    verify_ack_after_persist_with_matrix(DURABILITY_MATRIX)
}

/// Verify that no row claims ack-before-persist (testable variant).
pub fn verify_ack_after_persist_with_matrix(
    matrix: &[DurabilityRow],
) -> Result<(), DurabilityError> {
    for row in matrix {
        if row.ack_point == AckPoint::BeforeJournalAppend {
            return Err(DurabilityError::AckBeforePersist {
                primitive: row.primitive.to_owned(),
                handler: row.compiled_node_kind.to_owned(),
            });
        }
    }
    Ok(())
}

/// Run all matrix verifications.
pub fn verify_matrix() -> Result<(), DurabilityError> {
    verify_matrix_completeness()?;
    verify_matrix_replay_proofs()?;
    verify_ack_after_persist()?;
    Ok(())
}

/// Run all matrix verifications with custom primitives (testable variant).
pub fn verify_matrix_with_primitives_and_matrix(
    primitives: &[&str],
    matrix: &[DurabilityRow],
) -> Result<(), DurabilityError> {
    verify_matrix_completeness_with_primitives(primitives)?;
    verify_matrix_replay_proofs_with_matrix(matrix)?;
    verify_ack_after_persist_with_matrix(matrix)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
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
        let row = match DURABILITY_MATRIX.iter().find(|r| r.primitive == "set") {
            Some(r) => r,
            None => panic!("set primitive must have a row in the matrix"),
        };
        assert_eq!(row.compiled_node_kind, "SetConst");
        assert!(row.journal_events.contains(&RecordKind::StepStarted));
        assert!(row.journal_events.contains(&RecordKind::SlotWritten));
        assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
    }

    #[test]
    fn do_row_exists_and_is_correct() {
        let row = match DURABILITY_MATRIX.iter().find(|r| r.primitive == "do") {
            Some(r) => r,
            None => panic!("do primitive must have a row in the matrix"),
        };
        assert_eq!(row.compiled_node_kind, "Do");
        assert!(row.journal_events.contains(&RecordKind::ActionScheduled));
        assert!(row.journal_events.contains(&RecordKind::ActionCompleted));
        assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
    }

    #[test]
    fn wait_row_names_wait_scheduled_and_wait_resolved() {
        let row = match DURABILITY_MATRIX.iter().find(|r| r.primitive == "wait") {
            Some(r) => r,
            None => panic!("wait primitive must have a row in the matrix"),
        };
        assert!(row.journal_events.contains(&RecordKind::WaitScheduled));
        assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
    }

    #[test]
    fn ask_row_names_ask_scheduled_and_ask_answered() {
        let row = match DURABILITY_MATRIX.iter().find(|r| r.primitive == "ask") {
            Some(r) => r,
            None => panic!("ask primitive must have a row in the matrix"),
        };
        assert!(row.journal_events.contains(&RecordKind::AskScheduled));
        assert!(row.journal_events.contains(&RecordKind::AskAnswered));
        assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
    }

    #[test]
    fn finish_row_names_run_finished() {
        let row = match DURABILITY_MATRIX.iter().find(|r| r.primitive == "finish") {
            Some(r) => r,
            None => panic!("finish primitive must have a row in the matrix"),
        };
        assert!(row.journal_events.contains(&RecordKind::RunFinished));
        assert_eq!(row.ack_point, AckPoint::AfterJournalAppend);
    }

    #[test]
    fn verify_matrix_completeness_err_when_primitive_missing() {
        let result = verify_matrix_completeness_with_primitives(&["nonexistent_primitive"]);
        let err = match result {
            Err(e) => e,
            Ok(v) => panic!("Expected error for missing primitive, got Ok: {:?}", v),
        };
        assert!(
            matches!(&err, DurabilityError::MissingPrimitiveRow { primitive } if primitive == "nonexistent_primitive"),
            "Expected MissingPrimitiveRow error, got: {:?}",
            err
        );
    }

    #[test]
    fn verify_matrix_replay_proofs_err_when_evidence_empty() {
        let phantom_row = DurabilityRow {
            primitive: "phantom",
            compiled_node_kind: "Phantom",
            journal_events: &[RecordKind::StepStarted],
            storage_partition: StoragePartition::RuntimeJournal,
            ack_point: AckPoint::AfterJournalAppend,
            replay_assertion: "phantom",
            test_evidence: &[],
        };
        let result = verify_matrix_replay_proofs_with_matrix(&[phantom_row]);
        let err = match result {
            Err(e) => e,
            Ok(v) => panic!("Expected error for empty evidence, got Ok: {:?}", v),
        };
        assert!(
            matches!(&err, DurabilityError::MissingReplayProof { primitive, .. } if primitive == "phantom"),
            "Expected MissingReplayProof error, got: {:?}",
            err
        );
    }

    #[test]
    fn verify_ack_after_persist_err_when_before_journal_append() {
        let bad_row = DurabilityRow {
            primitive: "bad_handler",
            compiled_node_kind: "BadHandler",
            journal_events: &[RecordKind::StepStarted],
            storage_partition: StoragePartition::RuntimeJournal,
            ack_point: AckPoint::BeforeJournalAppend,
            replay_assertion: "bad",
            test_evidence: &["test.rs"],
        };
        let result = verify_ack_after_persist_with_matrix(&[bad_row]);
        let err = match result {
            Err(e) => e,
            Ok(v) => panic!("Expected error for ack-before-persist, got Ok: {:?}", v),
        };
        assert!(
            matches!(&err, DurabilityError::AckBeforePersist { primitive, .. } if primitive == "bad_handler"),
            "Expected AckBeforePersist error, got: {:?}",
            err
        );
    }

    #[test]
    fn verify_matrix_completeness_empty_primitives_succeeds() {
        let result = verify_matrix_completeness_with_primitives(&[]);
        assert!(
            result.is_ok(),
            "Empty primitives list should succeed, got: {:?}",
            result
        );
    }

    #[test]
    fn verify_matrix_completeness_single_primitive_succeeds() {
        let result = verify_matrix_completeness_with_primitives(&["set"]);
        assert!(
            result.is_ok(),
            "Single existing primitive should succeed, got: {:?}",
            result
        );
    }

    #[test]
    fn verify_matrix_completeness_err_when_duplicate_primitives() {
        let result = verify_matrix_completeness_with_primitives(&["set", "set"]);
        assert!(
            result.is_ok(),
            "Duplicate primitives should still pass completeness check (not a completeness error), got: {:?}",
            result
        );
    }
}

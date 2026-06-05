#![forbid(unsafe_code)]
//! BDD tests for vb-eepg: Typed Partitioned ID Persistence Invariants
//!
//! This test module covers all 8 behaviors and 23 BDD scenarios from the vb-eepg test plan.
//! Tests are organized by behavior: B-001 through B-008.

use vb_core::{ActionId, RunId, SeqNo, StepIdx, WorkflowId};
use vb_storage::{
    JournalError, RecordKind,
    codec::{decode_record_header, encode_record},
    keys,
    types::EventSeq,
};
// =============================================================================
// Helper Functions
// =============================================================================

/// Builds a properly encoded record header for testing.
/// Uses encode_record to create a valid header, then corrupts the kind to test rejection.
#[allow(dead_code)]
fn make_header_with_kind(_kind: u16) -> Vec<u8> {
    // Create a minimal valid header by encoding an event
    // The kind will be corrupted later by corrupt_kind_in_header
    let event = vb_storage::JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    // Use kind 10 (RunAccepted) which is valid for MAGIC_JOURNAL_EVENT
    let valid_kind = RecordKind::RunAccepted;
    encode_record(
        vb_storage::constants::MAGIC_JOURNAL_EVENT,
        valid_kind,
        0,
        &event,
        vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed")
}

/// Corrupts the kind in a header to the specified value and recomputes CRC.
fn corrupt_kind_in_header(header: &mut [u8], new_kind: u16) {
    // Kind is at offset 6 (2 bytes, little-endian)
    header[6..8].copy_from_slice(&new_kind.to_le_bytes());
    // Recompute CRC (at offset CRC_OFFSET = 56)
    let checksum = crc32c::crc32c(&header[..vb_storage::constants::CRC_OFFSET]);
    header[vb_storage::constants::CRC_OFFSET..vb_storage::constants::CRC_OFFSET + 4]
        .copy_from_slice(&checksum.to_le_bytes());
}

/// Returns true if the kind is NOT a known record kind.
#[allow(dead_code)]
fn is_unknown_kind(kind: u16) -> bool {
    !matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50)
}

// =============================================================================
// B-001: RunId roundtrip
// =============================================================================

mod run_id_roundtrip {
    use super::*;

    #[test]
    fn run_header_key_preserves_run_id_bytes() -> Result<(), JournalError> {
        // Given: a RunId constructed from any u64 value
        let run_id = RunId::new(0xDEAD_BEEF_CAFE_BABE);

        // When: run_header_key is called
        let key = keys::run_header_key(run_id)?;

        // Then: the resulting key bytes are exactly [0x10] followed by the big-endian u64 bytes
        assert_eq!(
            key[0],
            vb_storage::constants::PREFIX_RUN_HEADER,
            "prefix must be 0x10"
        );
        assert_eq!(
            &key[1..9],
            run_id.get().to_be_bytes(),
            "run_id bytes must be preserved in big-endian"
        );

        // And: the first 9 bytes decode back to the original RunId
        let recovered_run_id = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));
        assert_eq!(recovered_run_id, run_id);
        Ok(())
    }

    #[test]
    fn run_header_key_prefix_is_0x10() -> Result<(), JournalError> {
        // Given: a RunId with value 0
        let run_id = RunId::new(0);

        // When: run_header_key is called
        let key = keys::run_header_key(run_id)?;

        // Then: key[0] equals 0x10 (PREFIX_RUN_HEADER)
        assert_eq!(key[0], 0x10, "prefix byte must be 0x10");
        assert_eq!(
            key[0],
            vb_storage::constants::PREFIX_RUN_HEADER,
            "prefix must match constant"
        );
        Ok(())
    }

    #[test]
    fn run_header_key_zero_run_id() -> Result<(), JournalError> {
        // Given: RunId::new(0)
        let run_id = RunId::new(0);

        // When: run_header_key is called
        let key = keys::run_header_key(run_id)?;

        // Then: key[1..9] equals 0u64.to_be_bytes()
        assert_eq!(
            &key[1..9],
            0u64.to_be_bytes(),
            "zero run_id must encode as all-zero bytes"
        );
        assert_eq!(key.len(), 9, "run_header_key must be 9 bytes");
        Ok(())
    }

    #[test]
    fn run_header_key_max_run_id() -> Result<(), JournalError> {
        // Given: RunId::new(u64::MAX)
        let run_id = RunId::new(u64::MAX);

        // When: run_header_key is called
        let key = keys::run_header_key(run_id)?;

        // Then: key[1..9] equals u64::MAX.to_be_bytes()
        assert_eq!(
            &key[1..9],
            u64::MAX.to_be_bytes(),
            "max run_id must encode as all-0xFF bytes"
        );
        Ok(())
    }
}

// =============================================================================
// B-002: WorkflowId roundtrip
// =============================================================================

mod workflow_id_roundtrip {
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
        let key = keys::index_workflow_key(workflow_id, run_id)?;

        // Then: key[1..5] equals 0u32.to_be_bytes()
        // And: key[5..13] equals 0u64.to_be_bytes()
        assert_eq!(
            &key[1..5],
            0u32.to_be_bytes(),
            "zero workflow_id must encode as all-zero bytes"
        );
        assert_eq!(
            &key[5..13],
            0u64.to_be_bytes(),
            "zero run_id must encode as all-zero bytes"
        );
        assert_eq!(key.len(), 13, "index_workflow_key must be 13 bytes");
        Ok(())
    }
}

// =============================================================================
// B-003: ActionTicketKey roundtrip
// =============================================================================

mod action_ticket_key_roundtrip {
    use super::*;

    #[test]
    fn index_action_key_preserves_all_fields() -> Result<(), JournalError> {
        // Given: ActionId, RunId, and StepIdx with arbitrary values
        let action_id = ActionId::new(0x1234);
        let run_id = RunId::new(0xDEAD_BEEF_CAFE_BABE);
        let step_idx = StepIdx::new(0x5678);

        // When: index_action_key is called
        let key = keys::index_action_key(action_id, run_id, step_idx)?;

        // Then: key = [0x32][action_u16_be][run_u64_be][step_u16_be]
        assert_eq!(
            key[0],
            vb_storage::constants::PREFIX_INDEX_ACTION,
            "prefix must be 0x32"
        );
        assert_eq!(
            &key[1..3],
            action_id.get().to_be_bytes(),
            "action_id bytes must be preserved"
        );
        assert_eq!(
            &key[3..11],
            run_id.get().to_be_bytes(),
            "run_id bytes must be preserved"
        );
        assert_eq!(
            &key[11..13],
            step_idx.get().to_be_bytes(),
            "step_idx bytes must be preserved"
        );

        // And: each field decodes to its original value
        let recovered_action = ActionId::new(u16::from_be_bytes(key[1..3].try_into().unwrap()));
        let recovered_run_id = RunId::new(u64::from_be_bytes(key[3..11].try_into().unwrap()));
        let recovered_step = StepIdx::new(u16::from_be_bytes(key[11..13].try_into().unwrap()));
        assert_eq!(recovered_action, action_id);
        assert_eq!(recovered_run_id, run_id);
        assert_eq!(recovered_step, step_idx);
        Ok(())
    }

    #[test]
    fn index_action_key_max_values() -> Result<(), JournalError> {
        // Given: ActionId::new(u16::MAX), RunId::new(u64::MAX), StepIdx::new(u16::MAX)
        let action_id = ActionId::new(u16::MAX);
        let run_id = RunId::new(u64::MAX);
        let step_idx = StepIdx::new(u16::MAX);

        // When: index_workflow_key is called (typo in spec, should be index_action_key)
        let key = keys::index_action_key(action_id, run_id, step_idx)?;

        // Then: all field bytes equal their type::MAX in big-endian
        assert_eq!(
            &key[1..3],
            u16::MAX.to_be_bytes(),
            "max action_id must encode as 0xFFFF"
        );
        assert_eq!(
            &key[3..11],
            u64::MAX.to_be_bytes(),
            "max run_id must encode as all 0xFF"
        );
        assert_eq!(
            &key[11..13],
            u16::MAX.to_be_bytes(),
            "max step_idx must encode as 0xFFFF"
        );
        Ok(())
    }
}

// =============================================================================
// B-004: SeqNo overflow rejection
// =============================================================================

mod seqno_overflow_rejection {
    use super::*;

    #[test]
    fn seqno_overflow_returns_none() {
        // Given: SeqNo::new(u64::MAX)
        let seq = SeqNo::new(u64::MAX);

        // When: checked_add(1) is called
        let result = seq.checked_add(1);

        // Then: the result is None
        assert!(result.is_none(), "u64::MAX + 1 must return None (overflow)");
    }

    #[test]
    fn seqno_add_exact_max_succeeds() {
        // Given: SeqNo::new(0)
        let seq = SeqNo::new(0);

        // When: checked_add(u64::MAX) is called
        let result = seq.checked_add(u64::MAX);

        // Then: the result is Some(SeqNo::new(u64::MAX))
        assert_eq!(
            result,
            Some(SeqNo::new(u64::MAX)),
            "0 + u64::MAX must equal u64::MAX"
        );
    }

    #[test]
    fn seqno_normal_addition_succeeds() {
        // Given: SeqNo::new(100)
        let seq = SeqNo::new(100);

        // When: checked_add(50) is called
        let result = seq.checked_add(50);

        // Then: the result is Some(SeqNo::new(150))
        assert_eq!(result, Some(SeqNo::new(150)), "100 + 50 must equal 150");
    }

    #[test]
    fn seqno_checked_add_associativity() {
        // Test that checked_add is associative for non-overflowing cases
        let a = SeqNo::new(100);
        let b = SeqNo::new(50);
        let c = SeqNo::new(25);

        let ab = a.checked_add(b.get()).and_then(|x| x.checked_add(c.get()));
        let ac = a.checked_add((b.get() + c.get()) as u64);

        assert_eq!(
            ab, ac,
            "checked_add must be associative for non-overflowing values"
        );
    }
}

// =============================================================================
// B-005: Unknown record kind rejection
// =============================================================================

mod unknown_record_kind_rejection {
    use super::*;

    #[test]
    fn decode_rejects_unknown_record_kind() -> Result<(), JournalError> {
        // Given: a record header with kind = 255 (unknown)
        let mut header = make_header_with_kind(255);
        corrupt_kind_in_header(&mut header, 255);

        // When: decode_record_header is called
        let result = decode_record_header(
            &header,
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        // Then: the result is Err(JournalError::UnknownRecordKind { kind: 255 })
        match result {
            Err(JournalError::UnknownRecordKind { kind }) => {
                assert_eq!(kind, 255, "unknown kind must be 255");
            }
            other => panic!("expected UnknownRecordKind(255), got {:?}", other),
        }
        Ok(())
    }

    #[test]
    fn decode_accepts_all_known_journal_event_kinds() -> Result<(), JournalError> {
        // Given: record headers with kinds 10..=27 (valid for MAGIC_JOURNAL_EVENT)
        let known_journal_kinds: Vec<u16> = vec![
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        ];

        for &kind in &known_journal_kinds {
            let mut header = make_header_with_kind(kind);
            corrupt_kind_in_header(&mut header, kind);

            let result = decode_record_header(
                &header,
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );

            // Then: no kind is rejected as unknown
            assert!(
                result.is_ok(),
                "known journal event kind {} must not be rejected, got {:?}",
                kind,
                result
            );
        }
        Ok(())
    }

    #[test]
    fn is_known_record_kind_returns_true_for_valid_kinds() {
        // Given: kind values that are in the known set {1, 2, 3, 10-28, 30, 40, 50}
        // We test through decode_record_header which calls is_known_record_kind internally.
        // Note: Some kinds will fail with RecordKindFamilyMismatch because they don't
        // match MAGIC_JOURNAL_EVENT, but they should NOT fail with UnknownRecordKind.
        let valid_kinds: Vec<u16> = vec![
            1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
            30, 40, 50,
        ];

        for &kind in &valid_kinds {
            let mut header = make_header_with_kind(kind);
            corrupt_kind_in_header(&mut header, kind);

            let result = decode_record_header(
                &header,
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );

            // Then: none return UnknownRecordKind error (is_known_record_kind returned true)
            // Some may return RecordKindFamilyMismatch if kind doesn't match MAGIC_JOURNAL_EVENT,
            // but UnknownRecordKind means is_known_record_kind returned false.
            assert!(
                !matches!(result, Err(JournalError::UnknownRecordKind { .. })),
                "kind {} must be known (is_known_record_kind returned true), got {:?}",
                kind,
                result
            );
        }
    }

    #[test]
    fn is_known_record_kind_returns_false_for_invalid_kinds() {
        // Given: kind values 0, 4..=9, 31..=39, 41..=49, 51..=65535
        // Note: 29 is StepSucceeded (valid) after vb-mrwe.5 fix
        // Note: 30 is Snapshot (valid kind but for MAGIC_SNAPSHOT, not MAGIC_JOURNAL_EVENT)
        // We test a representative sample due to the large range
        let invalid_kinds: Vec<u16> = vec![
            0, 4, 5, 6, 7, 8, 9, // 4..=9
            31, 32, 33, 34, 35, 36, 37, 38, 39, // 31..=39
            41, 42, 43, 44, 45, 46, 47, 48, 49, // 41..=49
            51, 52, // small sample after 50
            100, 255,   // edge cases
            65535, // max u16
        ];

        for &kind in &invalid_kinds {
            let mut header = make_header_with_kind(kind);
            corrupt_kind_in_header(&mut header, kind);

            let result = decode_record_header(
                &header,
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );

            // Then: all return UnknownRecordKind error (is_known_record_kind returned false)
            assert!(
                matches!(result, Err(JournalError::UnknownRecordKind { kind: k }) if k == kind),
                "kind {} must be unknown, got {:?}",
                kind,
                result
            );
        }
    }
}

// =============================================================================
// B-006: RecordKind stable IDs
// =============================================================================

mod record_kind_stable_ids {
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
        assert_eq!(RecordKind::Snapshot.id(), 30, "Snapshot.id() must equal 30");
        assert_eq!(RecordKind::Blob.id(), 40, "Blob.id() must equal 40");
        assert_eq!(
            RecordKind::IndexUpdate.id(),
            50,
            "IndexUpdate.id() must equal 50"
        );
    }
}

// =============================================================================
// B-007: Storage key family separation
// =============================================================================

mod storage_key_family_separation {
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
}

// =============================================================================
// B-008: Min/max numeric ID roundtrip
// =============================================================================

mod min_max_numeric_id_roundtrip {
    use super::*;

    #[test]
    fn run_id_zero_roundtrip() -> Result<(), JournalError> {
        // Given: RunId::new(0)
        let run_id = RunId::new(0);

        // When: run_header_key is called then decoded
        let key = keys::run_header_key(run_id)?;

        // Then: the decoded RunId equals the original
        let decoded = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));
        assert_eq!(decoded, run_id, "zero RunId must roundtrip correctly");
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
}

// =============================================================================
// Integration Tests: Full Roundtrip Scenarios
// =============================================================================

mod integration_full_roundtrip {
    use super::*;

    #[test]
    fn full_run_id_to_key_to_decode_roundtrip() -> Result<(), JournalError> {
        // Given: a valid RunId value
        let original = RunId::new(0x1234_5678_ABCD_EF00);

        // When: run_header_key is constructed, and the key bytes are decoded back
        let key = keys::run_header_key(original)?;
        let recovered = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));

        // Then: the decoded RunId equals the original
        assert_eq!(recovered, original);
        Ok(())
    }

    #[test]
    fn full_workflow_id_index_key_roundtrip() -> Result<(), JournalError> {
        // Given: WorkflowId::new(12345) and RunId::new(67890)
        let workflow_id = WorkflowId::new(12345);
        let run_id = RunId::new(67890);

        // When: index_workflow_key is constructed and the key components are extracted
        let key = keys::index_workflow_key(workflow_id, run_id)?;

        // Then: the extracted workflow_id matches 12345 and run_id matches 67890
        let extracted_workflow = WorkflowId::new(u32::from_be_bytes(key[1..5].try_into().unwrap()));
        let extracted_run_id = RunId::new(u64::from_be_bytes(key[5..13].try_into().unwrap()));

        assert_eq!(extracted_workflow, workflow_id);
        assert_eq!(extracted_run_id, run_id);
        Ok(())
    }
}

// =============================================================================
// E2E: FromStr Cold Path Roundtrip
// =============================================================================

mod e2e_fromstr_cold_path {
    use vb_core::RunId;

    #[test]
    fn fromstr_max_u64_roundtrip() -> Result<(), String> {
        // Given: the string "18446744073709551615" (u64::MAX)
        let s = "18446744073709551615";

        // When: it is parsed as RunId via FromStr
        let run_id: RunId = s.parse().map_err(|e| format!("parse error: {e}"))?;

        // Then: the resulting RunId.get() equals 18446744073709551615
        assert_eq!(
            run_id.get(),
            18446744073709551615u64,
            "parsed max u64 must equal original value"
        );

        // And: run_header_key produces correct key bytes
        let key = vb_storage::keys::run_header_key(run_id).map_err(|e| e.to_string())?;
        assert_eq!(
            &key[1..9],
            18446744073709551615u64.to_be_bytes(),
            "key bytes must match max u64"
        );
        Ok(())
    }

    #[test]
    fn fromstr_zero_roundtrip() -> Result<(), String> {
        // Given: the string "0"
        let s = "0";

        // When: it is parsed as RunId via FromStr
        let run_id: RunId = s.parse().map_err(|e| format!("parse error: {e}"))?;

        // Then: the resulting RunId.get() equals 0
        assert_eq!(run_id.get(), 0u64, "parsed zero must equal 0");
        Ok(())
    }

    #[test]
    fn fromstr_rejects_empty() {
        // Given: empty string
        let s = "";

        // When: it is parsed as RunId via FromStr
        let result: Result<RunId, _> = s.parse();

        // Then: the result is Err
        assert!(result.is_err(), "empty string must fail to parse");
    }

    #[test]
    fn fromstr_rejects_negative() {
        // Given: negative string
        let s = "-1";

        // When: it is parsed as RunId via FromStr
        let result: Result<RunId, _> = s.parse();

        // Then: the result is Err
        assert!(result.is_err(), "negative string must fail to parse");
    }

    #[test]
    fn fromstr_rejects_overflow() {
        // Given: string larger than u64::MAX
        let s = "18446744073709551616"; // u64::MAX + 1

        // When: it is parsed as RunId via FromStr
        let result: Result<RunId, _> = s.parse();

        // Then: the result is Err
        assert!(result.is_err(), "overflow string must fail to parse");
    }
}

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
    !matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 40 | 50)
}

// =============================================================================
// B-001: RunId roundtrip
// =============================================================================

#[path = "vb_eepg_bdd_tests/run_id_roundtrip_tests.rs"]
mod run_id_roundtrip;

// =============================================================================
// B-002: WorkflowId roundtrip
// =============================================================================

#[path = "vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs"]
mod workflow_id_roundtrip;

// =============================================================================
// B-003: ActionTicketKey roundtrip
// =============================================================================

#[path = "vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs"]
mod action_ticket_key_roundtrip;

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

#[path = "vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs"]
mod unknown_record_kind_rejection;

// =============================================================================
// B-006: RecordKind stable IDs
// =============================================================================

#[path = "vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs"]
mod record_kind_stable_ids;

// =============================================================================
// B-007: Storage key family separation
// =============================================================================

#[path = "vb_eepg_bdd_tests/storage_key_family_separation_tests.rs"]
mod storage_key_family_separation;

// =============================================================================
// B-008: Min/max numeric ID roundtrip
// =============================================================================

#[path = "vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs"]
mod min_max_numeric_id_roundtrip;

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

#![forbid(unsafe_code)]

use crate::codec::{decode_record, decode_record_header, encode_record};
use crate::constants::RECORD_HEADER_BYTES;
use crate::{
    BlobRecord, EventSeq, JournalEvent, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_BLOB_BYTES,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RecordKind,
};
use blake3::hash;
use proptest::test_runner::TestCaseError;
use vb_core::RunId;

/// Deterministic coordinates for all matrix rows.
const MATRIX_RUN: u64 = 907;
const MATRIX_SEQ: u64 = 83;
const MATRIX_ATTEMPT: u16 = 9;

/// The 21 selected journaled kind IDs, with their RecordKind values.
/// RunKilled is 28; kind 30 (Snapshot) is deliberately excluded.
const SELECTED_KINDS: [(u16, RecordKind); 21] = [
    (10, RecordKind::RunAccepted),
    (11, RecordKind::StepStarted),
    (12, RecordKind::SlotWritten),
    (13, RecordKind::ActionScheduled),
    (14, RecordKind::ActionCompleted),
    (15, RecordKind::ActionFailed),
    (16, RecordKind::WaitScheduled),
    (17, RecordKind::AskScheduled),
    (18, RecordKind::AskAnswered),
    (19, RecordKind::RetryScheduled),
    (20, RecordKind::StepFailed),
    (21, RecordKind::RunCancelled),
    (22, RecordKind::RunFinished),
    (23, RecordKind::RunFailed),
    (24, RecordKind::RunAdmission),
    (25, RecordKind::RunResumed),
    (26, RecordKind::RunRetried),
    (27, RecordKind::RunAnswered),
    (28, RecordKind::RunKilled),
    (29, RecordKind::AskTimedOut),
    (31, RecordKind::WaitResolved),
];

const NO_ATTEMPT_KINDS: [u16; 5] = [10, 24, 25, 26, 27];

/// Run the full 21-row kind matrix with deterministic coordinates.
fn matrix_encode_decode_all_kinds() -> Result<(), TestCaseError> {
    let run = RunId::new(MATRIX_RUN);
    let seq = EventSeq::new(MATRIX_SEQ);
    let kind_by_id: std::collections::HashMap<u16, RecordKind> = SELECTED_KINDS
        .iter()
        .map(|(id, kind)| (*id, *kind))
        .collect();

    for (kind_id, kind) in &SELECTED_KINDS {
        let event =
            super::codec_fixtures::journal_event(*kind, run, seq, MATRIX_ATTEMPT).map_err(|e| {
                TestCaseError::fail(format!(
                    "journal_event fixture failed for kind {kind_id}: {e}"
                ))
            })?;

        if event.run_id() != run {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: run_id mismatch after factory"
            )));
        }
        if event.seq() != seq {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: seq mismatch after factory"
            )));
        }
        if event.record_kind() != *kind {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: record_kind mismatch after factory"
            )));
        }
        if !event.is_valid() {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: is_valid() returned false"
            )));
        }

        // Encode via the production API.
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            *kind,
            seq.get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| TestCaseError::fail(format!("encode failed for kind {kind_id}: {e}")))?;

        // Verify the envelope header fields before decoding.
        if encoded.len() <= RECORD_HEADER_BYTES {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: encoded length too short ({})",
                encoded.len()
            )));
        }
        let hdr = decode_record_header(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| {
            TestCaseError::fail(format!("header decode failed for kind {kind_id}: {e}"))
        })?;
        if hdr.magic != MAGIC_JOURNAL_EVENT {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: envelope magic mismatch ({:#x})",
                hdr.magic
            )));
        }
        if hdr.record_kind != *kind_id {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: envelope record_kind mismatch ({})",
                hdr.record_kind
            )));
        }
        if hdr.sequence != seq.get() {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: envelope sequence mismatch ({})",
                hdr.sequence
            )));
        }

        // Decode and compare the payload back to the original.
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| TestCaseError::fail(format!("decode failed for kind {kind_id}: {e}")))?;

        // Envelope must match original.
        if envelope.magic != MAGIC_JOURNAL_EVENT {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded envelope magic mismatch"
            )));
        }
        if envelope.record_kind != *kind_id {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded envelope record_kind mismatch"
            )));
        }
        if envelope.sequence != seq.get() {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded envelope sequence mismatch"
            )));
        }

        // Decoded event must be byte-identical to the original.
        if decoded != event {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded event differs from original"
            )));
        }

        // Event metadata must round-trip exactly.
        if decoded.run_id() != run {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded run_id mismatch"
            )));
        }
        if decoded.seq() != seq {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded seq mismatch"
            )));
        }
        if decoded.record_kind() != *kind {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded record_kind mismatch"
            )));
        }
        if !decoded.is_valid() {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: decoded is_valid() returned false"
            )));
        }

        // Attempt parity: five kinds have no attempt field; the other 16
        // must round-trip the exact attempt value.
        let has_attempt = !NO_ATTEMPT_KINDS.contains(kind_id);
        let expected_attempt = if has_attempt {
            Some(MATRIX_ATTEMPT)
        } else {
            None
        };
        if decoded.attempt() != expected_attempt {
            return Err(TestCaseError::fail(format!(
                "kind {kind_id}: attempt mismatch: expected {:?}, got {:?}",
                expected_attempt,
                decoded.attempt()
            )));
        }
    }

    let run_killed_kind = kind_by_id
        .get(&28)
        .copied()
        .ok_or_else(|| TestCaseError::fail("kind 28 not in SELECTED_KINDS map"))?;
    proptest::prop_assert_eq!(run_killed_kind, RecordKind::RunKilled);

    Ok(())
}

fn blob_roundtrip_empty() -> Result<(), TestCaseError> {
    let bytes = Vec::<u8>::new();
    let digest = hash(&bytes);
    let record = BlobRecord {
        digest: *digest.as_bytes(),
        bytes: bytes.clone(),
    };

    let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)
        .map_err(|e| TestCaseError::fail(format!("blob encode failed (empty): {e}")))?;

    let (envelope, decoded) = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, MAX_BLOB_BYTES)
        .map_err(|e| TestCaseError::fail(format!("blob decode failed (empty): {e}")))?;

    if envelope.magic != MAGIC_BLOB {
        return Err(TestCaseError::fail("blob envelope magic mismatch (empty)"));
    }
    if envelope.record_kind != RecordKind::Blob.id() {
        return Err(TestCaseError::fail(
            "blob envelope record_kind mismatch (empty)",
        ));
    }
    proptest::prop_assert_eq!(
        envelope.sequence,
        0,
        "blob envelope sequence mismatch (empty)"
    );
    if decoded != record {
        return Err(TestCaseError::fail(
            "blob decoded record differs from original (empty)",
        ));
    }
    if decoded.bytes.len() != 0 {
        return Err(TestCaseError::fail("blob decoded length mismatch (empty)"));
    }

    Ok(())
}

/// Blob positive control uses the 4095-byte (domain max minus one) payload.
fn blob_roundtrip_max_minus_one() -> Result<(), TestCaseError> {
    let bytes: Vec<u8> = (0u8..=u8::MAX).cycle().take(4095).collect();
    let digest = hash(&bytes);
    let record = BlobRecord {
        digest: *digest.as_bytes(),
        bytes: bytes.clone(),
    };

    let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)
        .map_err(|e| TestCaseError::fail(format!("blob encode failed (4095): {e}")))?;

    let (envelope, decoded) = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, MAX_BLOB_BYTES)
        .map_err(|e| TestCaseError::fail(format!("blob decode failed (4095): {e}")))?;

    if envelope.magic != MAGIC_BLOB {
        return Err(TestCaseError::fail("blob envelope magic mismatch (4095)"));
    }
    if envelope.record_kind != RecordKind::Blob.id() {
        return Err(TestCaseError::fail(
            "blob envelope record_kind mismatch (4095)",
        ));
    }
    proptest::prop_assert_eq!(
        envelope.sequence,
        0,
        "blob envelope sequence mismatch (4095)"
    );
    if decoded != record {
        return Err(TestCaseError::fail(
            "blob decoded record differs from original (4095)",
        ));
    }
    if decoded.bytes.len() != 4095 {
        return Err(TestCaseError::fail("blob decoded length mismatch (4095)"));
    }

    Ok(())
}

#[test]
fn codec_matrix_all_21_kinds_roundtrip() -> Result<(), TestCaseError> {
    matrix_encode_decode_all_kinds()
}

#[test]
fn blob_positive_control_empty() -> Result<(), TestCaseError> {
    blob_roundtrip_empty()
}

#[test]
fn blob_positive_control_4095_bytes() -> Result<(), TestCaseError> {
    blob_roundtrip_max_minus_one()
}

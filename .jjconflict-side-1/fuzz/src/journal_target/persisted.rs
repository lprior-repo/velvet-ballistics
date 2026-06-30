//! Persisted journal payload boundary fuzz target bodies.

use super::errors::assert_typed_journal_error;

pub fn fuzz_vb_qi37_12_persisted_payload_decode(data: &[u8]) {
    let max_payload_len = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    match decoded {
        Ok((_envelope, _event)) => {}
        Err(error) => assert_typed_journal_error(error),
    }
    exercise_truncated_persisted_payload(max_payload_len);
    exercise_corrupted_persisted_payload(max_payload_len);
}

fn exercise_truncated_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x37; 32]),
    };
    let Ok(encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(truncated_len) = encoded.len().checked_sub(1) else {
        return;
    };
    let Some(truncated) = encoded.get(..truncated_len) else {
        return;
    };
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        truncated,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(matches!(result, Err(vb_storage::JournalError::UnexpectedEof)));
}

fn exercise_corrupted_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(2),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x12; 32]),
    };
    let Ok(mut encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(last) = encoded.last_mut() else {
        return;
    };
    *last ^= 0xA5;
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        &encoded,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(matches!(
        result,
        Err(vb_storage::JournalError::PayloadDigestMismatch)
    ));
}

pub fn fuzz_storage_envelope_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };
    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(
            data,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
        return;
    }
    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    if let Err(e) = result {
        assert_typed_journal_error(e);
    }
    if data.len() < 60 {
        let result = decode_record::<vb_storage::JournalEvent>(
            data,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(
            result,
            Err(JournalError::UnexpectedEof) | Err(JournalError::HeaderLengthMismatch { .. })
        ));
    }
}

pub fn fuzz_binary_payload_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };
    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, 1024);
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
        return;
    }
    let small_max = 64u32;
    match decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, small_max) {
        Ok((_envelope, _event)) => {}
        Err(JournalError::PayloadTooLarge { .. }) => {}
        Err(e) => assert_typed_journal_error(e),
    }
    let tiny_max = 1u32;
    if let Err(e) = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, tiny_max) {
        assert_typed_journal_error(e);
    }
    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT.wrapping_add(1),
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    match result {
        Ok(_) | Err(JournalError::BadMagic { .. }) | Err(JournalError::RecordKindFamilyMismatch { .. }) => {}
        Err(e) => assert_typed_journal_error(e),
    }
}

pub fn fuzz_accepted_artifact_envelope_qi37_4_2(data: &[u8]) {
    let Ok(artifact) = postcard::from_bytes::<vb_storage::AcceptedArtifact>(data) else {
        return;
    };
    assert!(artifact.verification.gate_count > 0);
    assert!(artifact.accepted_at_seq.get() >= 1);
    let _ = artifact.verification.durable;
    let _ = artifact.digest;
    assert!(artifact.required_capabilities.len() <= 256);
}

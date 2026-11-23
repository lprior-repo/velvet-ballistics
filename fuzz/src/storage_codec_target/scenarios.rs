use super::*;

pub(super) fn fuzz_codec_payload_corruption(data: &[u8]) {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

    let Some(encoded) = encode_seeded_event(data) else {
        return;
    };

    assert_fresh_decode_ok(&encoded, magic, max_payload);
    assert_future_schema_rejected(&encoded, magic, max_payload);
    assert_old_schema_rejected(&encoded, magic, max_payload);
    assert_unknown_kind_rejected(&encoded, magic, max_payload);
    assert_kind_family_rejected(&encoded, magic, max_payload);
    assert_header_len_rejected(&encoded, magic, max_payload);
    assert_header_crc_rejected(&encoded, magic, max_payload);
    assert_header_digest_rejected(&encoded, magic, max_payload);
    assert_payload_byte_rejected(&encoded, magic, max_payload);
    assert_header_truncations_rejected(&encoded, magic, max_payload);
}

pub(super) fn fuzz_codec_kind_family_boundary(data: &[u8]) {
    let record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source: data.to_vec().leak_or_truncate(100),
    };

    assert_encode_ok(vb_storage::encode_record(
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &record,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    ));
    assert_all_family_mismatches(&record);
}

fn assert_all_family_mismatches(record: &vb_storage::WorkflowSourceRecord) {
    assert_encode_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_JOURNAL_EVENT,
            vb_storage::RecordKind::WorkflowSource,
            0,
            record,
            128,
        ),
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::WorkflowSource.id(),
    );
    assert_encode_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_BLOB,
            vb_storage::RecordKind::Snapshot,
            0,
            record,
            128,
        ),
        vb_storage::MAGIC_BLOB,
        vb_storage::RecordKind::Snapshot.id(),
    );
    assert_encode_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_SNAPSHOT,
            vb_storage::RecordKind::IndexUpdate,
            0,
            record,
            128,
        ),
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::RecordKind::IndexUpdate.id(),
    );
    assert_encode_family_mismatch(
        vb_storage::encode_record(
            vb_storage::MAGIC_WORKFLOW_SOURCE,
            vb_storage::RecordKind::CompiledIr,
            0,
            record,
            128,
        ),
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::CompiledIr.id(),
    );
}

pub(super) fn fuzz_codec_schema_version_boundary(data: &[u8]) {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

    let Some(encoded) = encode_seeded_event(data) else {
        return;
    };

    assert_future_schema_rejected(&encoded, magic, max_payload);
    assert_old_schema_rejected(&encoded, magic, max_payload);
    assert_unknown_kind_rejected(&encoded, magic, max_payload);
    assert_kind_family_rejected(&encoded, magic, max_payload);
    assert_header_len_rejected(&encoded, magic, max_payload);
}

pub(super) fn fuzz_codec_digest_verification(data: &[u8]) {
    assert_digest_mismatch(vb_storage::verify_digest_match(data, [0u8; 32]));
    assert_digest_mismatch(vb_storage::verify_digest_match(&[], [0u8; 32]));

    let hash: [u8; 32] = blake3::hash(data).into();
    let result = vb_storage::verify_digest_match(data, hash);
    assert!(
        matches!(result, Ok(())),
        "verify_digest_match(data, blake3::hash(data)) must succeed, got {result:?}"
    );

    let mut corrupted_hash = hash;
    if let Some(first) = corrupted_hash.first_mut() {
        *first ^= 1;
        assert_digest_mismatch(vb_storage::verify_digest_match(data, corrupted_hash));
    }
}

fn encode_seeded_event(data: &[u8]) -> Option<Vec<u8>> {
    let event = seeded_event(data);
    vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .ok()
}

fn seeded_event(data: &[u8]) -> vb_storage::JournalEvent {
    let digest_bytes: [u8; 32] = blake3::hash(data).into();
    vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes(digest_bytes),
    }
}

trait LeakOrTruncate {
    fn leak_or_truncate(self, max: usize) -> Self;
}

impl LeakOrTruncate for Vec<u8> {
    fn leak_or_truncate(mut self, max: usize) -> Self {
        self.truncate(max);
        self
    }
}

#![forbid(unsafe_code)]

use crate::storage_codec_assertions::{
    assert_encode_family_mismatch, assert_encode_ok, assert_fresh_decode_ok,
};
use crate::storage_codec_errors::assert_digest_mismatch;
use crate::storage_codec_mutation::{
    assert_future_schema_rejected, assert_header_crc_rejected, assert_header_digest_rejected,
    assert_header_len_rejected, assert_header_truncations_rejected, assert_kind_family_rejected,
    assert_old_schema_rejected, assert_payload_byte_rejected, assert_unknown_kind_rejected,
};

pub fn fuzz_vb_storage_codec(data: &[u8]) {
    fuzz_codec_encode_decode_path(data);
    fuzz_codec_header_decode_path(data);
    fuzz_codec_payload_corruption(data);
    fuzz_codec_kind_family_boundary(data);
    fuzz_codec_schema_version_boundary(data);
    fuzz_codec_digest_verification(data);
}

fn fuzz_codec_encode_decode_path(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;

    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload);
    decode_with_alternate_magics(data);

    if let Ok((envelope, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    {
        assert_reencodable_event(&event, magic, max_payload);
        assert_encode_family_mismatch(
            vb_storage::encode_record(
                0xFFFF_FFFF,
                event.record_kind(),
                event.seq().get(),
                &event,
                max_payload,
            ),
            0xFFFF_FFFF,
            event.record_kind().id(),
        );
        drop(vb_storage::encode_record(
            magic,
            event.record_kind(),
            event.seq().get(),
            &event,
            u32::MAX,
        ));
        let _ = envelope;
    }
}

fn decode_with_alternate_magics(data: &[u8]) {
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAX_BLOB_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAX_COMPILED_IR_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAX_SNAPSHOT_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAX_WORKFLOW_SOURCE_BYTES,
    );
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_INDEX_RECORD,
        vb_storage::MAX_RUN_HEADER_BYTES,
    );
}

fn assert_reencodable_event(event: &vb_storage::JournalEvent, magic: u32, max_payload: u32) {
    if event.is_valid()
        && let Ok(encoded) = vb_storage::encode_record(
            magic,
            event.record_kind(),
            event.seq().get(),
            event,
            max_payload,
        )
    {
        assert_fresh_decode_ok(&encoded, magic, max_payload);
    }
}

fn fuzz_codec_header_decode_path(data: &[u8]) {
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, 1024);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_BLOB, 65536);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_COMPILED_ARTIFACT, 16_777_216);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_SNAPSHOT, 67_108_864);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_WORKFLOW_SOURCE, 1_048_576);
    let _ = vb_storage::decode_record_header(data, 0xDEAD_BEEF, 1024);
    let _ = vb_storage::decode_record_header(data, 0x0000_0000, 65536);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, 1);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, 0);
}

fn fuzz_codec_payload_corruption(data: &[u8]) {
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

fn fuzz_codec_kind_family_boundary(data: &[u8]) {
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
        vb_storage::encode_record(vb_storage::MAGIC_BLOB, vb_storage::RecordKind::Snapshot, 0, record, 128),
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

fn fuzz_codec_schema_version_boundary(data: &[u8]) {
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

fn fuzz_codec_digest_verification(data: &[u8]) {
    drop(vb_storage::verify_digest_match(data, [0u8; 32]));
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

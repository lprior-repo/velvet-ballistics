#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_codec_encode_decode_path(data);
    fuzz_codec_header_decode_path(data);
    fuzz_codec_payload_corruption(data);
    fuzz_codec_kind_family_boundary(data);
    fuzz_codec_schema_version_boundary(data);
    fuzz_codec_digest_verification(data);
});

fn fuzz_codec_encode_decode_path(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;

    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload);

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

    if let Ok((envelope, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    {
        if event.is_valid() {
            if let Ok(encoded) = vb_storage::encode_record(
                magic,
                event.record_kind(),
                event.seq().get(),
                &event,
                max_payload,
            ) {
                let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
                    &encoded, magic, max_payload,
                );
            }
        }

        let _ = vb_storage::encode_record(
            0xFFFF_FFFF,
            event.record_kind(),
            event.seq().get(),
            &event,
            max_payload,
        );
        let _ = vb_storage::encode_record(
            magic,
            event.record_kind(),
            event.seq().get(),
            &event,
            u32::MAX,
        );
        let _ = envelope;
    }
}

fn fuzz_codec_header_decode_path(data: &[u8]) {
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, 1024);
    let _ = vb_storage::decode_record_header(data, vb_storage::MAGIC_BLOB, 65536);
    let _ =
        vb_storage::decode_record_header(data, vb_storage::MAGIC_COMPILED_ARTIFACT, 16_777_216);
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

    let Ok((envelope, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    else {
        return;
    };
    if !event.is_valid() {
        return;
    }

    let Ok(encoded) = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    ) else {
        return;
    };

    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

    for corruption_offset in [0u32, 1u32, 2u32, 3u32, 56u32, 60u32] {
        let mut corrupted = encoded.clone();
        let off = corruption_offset as usize;
        if off < corrupted.len() {
            corrupted[off] = corrupted[off].wrapping_add(1);
        }
        let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &corrupted,
            magic,
            max_payload,
        );
    }

    for truncation in 0usize..encoded.len().min(64) {
        let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &encoded[..truncation],
            magic,
            max_payload,
        );
    }

    let _ = envelope;
}

fn fuzz_codec_kind_family_boundary(data: &[u8]) {
    let record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        source: data.to_vec().leak_or_truncate(100),
    };

    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_BLOB,
        vb_storage::RecordKind::Snapshot,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::RecordKind::IndexUpdate,
        0,
        &record,
        128,
    );
    let _ = vb_storage::encode_record(
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::RecordKind::CompiledIr,
        0,
        &record,
        128,
    );
}

fn fuzz_codec_schema_version_boundary(data: &[u8]) {
    let magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let schema_version = vb_storage::CURRENT_SCHEMA_VERSION;

    let Ok((_, event)) =
        vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, max_payload)
    else {
        return;
    };
    if !event.is_valid() {
        return;
    }

    let Ok(mut encoded) = vb_storage::encode_record(
        magic,
        event.record_kind(),
        event.seq().get(),
        &event,
        max_payload,
    ) else {
        return;
    };

    if encoded.len() >= 60 {
        encoded[4..6].copy_from_slice(&(schema_version + 1).to_le_bytes());
        let checksum = crc32c::crc32c(&encoded[..56]);
        encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
        let _ =
            vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

        encoded[4..6].copy_from_slice(&(schema_version.wrapping_sub(1)).to_le_bytes());
        let checksum = crc32c::crc32c(&encoded[..56]);
        encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
        let _ =
            vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

        let unknown_kind: u16 = 999;
        encoded[6..8].copy_from_slice(&unknown_kind.to_le_bytes());
        let checksum = crc32c::crc32c(&encoded[..56]);
        encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
        let _ =
            vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);

        let wrong_header_len: u32 = 99;
        encoded[8..12].copy_from_slice(&wrong_header_len.to_le_bytes());
        let checksum = crc32c::crc32c(&encoded[..56]);
        encoded[56..60].copy_from_slice(&checksum.to_le_bytes());
        let _ =
            vb_storage::decode_record::<vb_storage::JournalEvent>(&encoded, magic, max_payload);
    }
}

fn fuzz_codec_digest_verification(data: &[u8]) {
    let _ = vb_storage::verify_digest_match(data, [0u8; 32]);
    let _ = vb_storage::verify_digest_match(&[], [0u8; 32]);

    let hash: [u8; 32] = blake3::hash(data).into();
    let _ = vb_storage::verify_digest_match(data, hash);
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

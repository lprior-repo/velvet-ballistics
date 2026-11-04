//! Property-based tests for serialization roundtrip integrity.

use crate::RecordKind;
use crate::codec::{decode_record, encode_record, validate_known_record_kind};
use crate::constants::DIGEST_BYTES;
use crate::records::BlobRecord;
use blake3::Hasher;
use proptest::prelude::*;

use crate::constants::MAGIC_BLOB;

const MAX_PAYLOAD: u32 = 65536;

/// Compute a test digest for content.
fn test_digest(data: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize().as_bytes()[0..DIGEST_BYTES]
        .try_into()
        .unwrap_or([0u8; DIGEST_BYTES])
}

// Property: Blob content roundtrip preserves bytes.
proptest! {
    #[test]
    fn blob_content_roundtrip(data in "[a-zA-Z0-9 ]{0,5000}") {
        let bytes = data.as_bytes();
        let digest = test_digest(bytes);
        let record = BlobRecord {
            digest,
            bytes: bytes.to_vec(),
        };
        let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 1, &record, MAX_PAYLOAD)
            .expect("encode failed");

        let decoded = decode_record(&encoded, MAGIC_BLOB, MAX_PAYLOAD)
            .expect("decode failed");
        let envelope = decoded.0;
        let result: BlobRecord = decoded.1;

        assert_eq!(envelope.record_kind, 40); // RecordKind::Blob = 40
        assert_eq!(result.digest, record.digest);
        assert_eq!(result.bytes, record.bytes);
    }
}

// Property: Binary blob roundtrip preserves bytes.
proptest! {
    #[test]
    fn binary_blob_roundtrip(bytes in "[a-zA-Z0-9 ]{0,1000}") {
        let data: Vec<u8> = bytes.bytes().collect();
        let digest = test_digest(&data);
        let record = BlobRecord {
            digest,
            bytes: data.clone(),
        };
        let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 1, &record, MAX_PAYLOAD)
            .expect("encode failed");

        let decoded = decode_record(&encoded, MAGIC_BLOB, MAX_PAYLOAD)
            .expect("decode failed");
        let envelope = decoded.0;
        let result: BlobRecord = decoded.1;

        assert_eq!(envelope.record_kind, 40); // RecordKind::Blob = 40
        assert_eq!(result.bytes, data);
    }
}

// Property: Record kind validation is consistent.
proptest! {
    #[test]
    fn known_kind_validation(kind in 0u16..=65535) {
        let known = crate::codec::is_known_record_kind(kind);
        let validate_result = validate_known_record_kind(kind);
        assert_eq!(known, validate_result.is_ok());
    }
}

// Property: Empty blob roundtrips.
#[test]
fn empty_blob_roundtrip() {
    let digest = test_digest(&[]);
    let record = BlobRecord {
        digest,
        bytes: vec![],
    };
    let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 1, &record, MAX_PAYLOAD)
        .expect("encode failed");

    let decoded = decode_record(&encoded, MAGIC_BLOB, MAX_PAYLOAD).expect("decode failed");
    let result: BlobRecord = decoded.1;
    assert_eq!(result.bytes, vec![]);
}

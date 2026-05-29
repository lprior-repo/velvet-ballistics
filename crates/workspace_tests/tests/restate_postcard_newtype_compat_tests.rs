#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::{
    JournalError, RecordKind, WorkflowSourceRecord, decode_record, encode_record,
    MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES, RECORD_HEADER_BYTES,
};

const RUN_ID_ZERO_POSTCARD: &[u8] = &[0];
const RUN_ID_REPRESENTATIVE_POSTCARD: &[u8] = &[42];
const RUN_ID_MAX_POSTCARD: &[u8] = &[255, 255, 255, 255, 255, 255, 255, 255, 255, 1];
const RECORD_KIND_RUN_ACCEPTED_POSTCARD_ENUM: &[u8] = &[3];
const RECORD_KIND_RUN_ACCEPTED_ENVELOPE_ID_U16_LE: &[u8] = &[10, 0];
const RECORD_KIND_RUN_HEADER_POSTCARD_ENUM: &[u8] = &[2];
const RECORD_KIND_RUN_HEADER_ENVELOPE_ID_U16_LE: &[u8] = &[3, 0];

fn sample_workflow_source_record() -> WorkflowSourceRecord {
    WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0x44_u8; 32]),
        source: vec![0x76_u8, 0x62_u8, 0x2d_u8, 0x64_u8, 0x79_u8, 0x62_u8, 0x6a_u8],
    }
}

fn encode_sample_record() -> Result<Vec<u8>, JournalError> {
    encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        7_u64,
        &sample_workflow_source_record(),
        MAX_WORKFLOW_SOURCE_BYTES,
    )
}

fn exact_workflow_digest_from_postcard(bytes: &[u8]) -> Result<WorkflowDigest, postcard::Error> {
    match postcard::take_from_bytes::<WorkflowDigest>(bytes) {
        Ok((digest, remaining)) if remaining.is_empty() => Ok(digest),
        Ok((_digest, _remaining)) => Err(postcard::Error::DeserializeUnexpectedEnd),
        Err(error) => Err(error),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn run_id_postcard_roundtrip_generated_values(value in any::<u64>()) {
        let run_id = RunId::new(value);
        let encoded = postcard::to_allocvec(&run_id);
        prop_assert!(encoded.is_ok());
        if let Ok(bytes) = encoded {
            let decoded = postcard::from_bytes::<RunId>(&bytes);
            prop_assert!(decoded.is_ok());
            if let Ok(decoded_run_id) = decoded {
                prop_assert_eq!(decoded_run_id, run_id);
                prop_assert_eq!(decoded_run_id.get(), value);
            }
        }
    }

    #[test]
    fn workflow_digest_postcard_roundtrip_generated_32_byte_patterns(bytes in any::<[u8; 32]>()) {
        let digest = WorkflowDigest::from_bytes(bytes);
        prop_assert_eq!(digest.as_bytes(), bytes);
        let encoded = postcard::to_allocvec(&digest);
        prop_assert!(encoded.is_ok());
        if let Ok(wire) = encoded {
            let decoded = postcard::from_bytes::<WorkflowDigest>(&wire);
            prop_assert!(decoded.is_ok());
            if let Ok(decoded_digest) = decoded {
                prop_assert_eq!(decoded_digest.as_bytes(), bytes);
            }
        }
    }

    #[test]
    fn missing_bytes_short_header_returns_unexpected_eof(len in 0_usize..RECORD_HEADER_BYTES) {
        let bytes = vec![0_u8; len];
        let decoded = decode_record::<WorkflowSourceRecord>(&bytes, MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES);
        prop_assert!(matches!(decoded, Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn missing_bytes_short_declared_payload_returns_unexpected_eof(cut_delta in 1_usize..8_usize) {
        let record = encode_sample_record();
        prop_assert!(record.is_ok());
        if let Ok(bytes) = record {
            let cut = bytes.len().saturating_sub(cut_delta);
            prop_assume!(cut >= RECORD_HEADER_BYTES);
            prop_assume!(cut < bytes.len());
            let decoded = decode_record::<WorkflowSourceRecord>(&bytes[..cut], MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES);
            prop_assert!(matches!(decoded, Err(JournalError::UnexpectedEof)));
        }
    }

    #[test]
    fn trailing_bytes_raw_workflow_digest_postcard_decode_rejects_nonempty_suffix(bytes32 in any::<[u8; 32]>(), suffix in prop::collection::vec(any::<u8>(), 1_usize..=64_usize)) {
        let encoded = postcard::to_allocvec(&WorkflowDigest::from_bytes(bytes32));
        prop_assert!(encoded.is_ok());
        if let Ok(mut bytes) = encoded {
            bytes.extend_from_slice(&suffix);
            let decoded = exact_workflow_digest_from_postcard(&bytes);
            prop_assert!(decoded.is_err());
        }
    }
}

#[test]
fn run_id_golden_postcard_bytes_zero_representative_and_max() -> Result<(), postcard::Error> {
    assert_eq!(postcard::to_allocvec(&RunId::ZERO)?, RUN_ID_ZERO_POSTCARD);
    assert_eq!(postcard::to_allocvec(&RunId::new(42_u64))?, RUN_ID_REPRESENTATIVE_POSTCARD);
    assert_eq!(postcard::to_allocvec(&RunId::new(u64::MAX))?, RUN_ID_MAX_POSTCARD);
    Ok(())
}

#[test]
fn workflow_digest_golden_postcard_bytes_freeze_exact_32_byte_payload() -> Result<(), postcard::Error> {
    let zero = WorkflowDigest::from_bytes([0_u8; 32]);
    assert_eq!(postcard::to_allocvec(&zero)?, vec![0_u8; 32]);
    let pattern = WorkflowDigest::from_bytes([0xA5_u8; 32]);
    assert_eq!(postcard::to_allocvec(&pattern)?, vec![0xA5_u8; 32]);
    Ok(())
}

#[test]
fn record_kind_postcard_enum_and_envelope_id_u16_le_surfaces_are_named_and_distinct() -> Result<(), postcard::Error> {
    assert_eq!(postcard::to_allocvec(&RecordKind::RunAccepted)?, RECORD_KIND_RUN_ACCEPTED_POSTCARD_ENUM);
    assert_eq!(RecordKind::RunAccepted.id().to_le_bytes(), RECORD_KIND_RUN_ACCEPTED_ENVELOPE_ID_U16_LE);
    assert_ne!(RECORD_KIND_RUN_ACCEPTED_POSTCARD_ENUM, RECORD_KIND_RUN_ACCEPTED_ENVELOPE_ID_U16_LE);

    assert_eq!(postcard::to_allocvec(&RecordKind::RunHeader)?, RECORD_KIND_RUN_HEADER_POSTCARD_ENUM);
    assert_eq!(RecordKind::RunHeader.id().to_le_bytes(), RECORD_KIND_RUN_HEADER_ENVELOPE_ID_U16_LE);
    assert_ne!(RECORD_KIND_RUN_HEADER_POSTCARD_ENUM, RECORD_KIND_RUN_HEADER_ENVELOPE_ID_U16_LE);
    Ok(())
}

#[test]
fn migration_required_fixture_message_names_required_migration_for_byte_drift() {
    let migration_name = "vb-dybj-postcard-newtype-compat-v1";
    assert!(!migration_name.is_empty(), "changing frozen postcard bytes requires a named migration");
    assert_eq!(RUN_ID_ZERO_POSTCARD, &[0], "migration_required: RunIdZero frozen bytes changed without {migration_name}");
    assert_eq!(RECORD_KIND_RUN_ACCEPTED_POSTCARD_ENUM, &[3], "migration_required: RecordKind postcard_enum frozen bytes changed without {migration_name}");
}

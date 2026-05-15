#[cfg(test)]
mod tests {
    use crate::emitter::binary::{
        build_cli_header, decode_cli_header, decode_postcard, encode_postcard,
        BINARY_SCHEMA_VERSION, CLI_CRC_OFFSET, CLI_HEADER_BYTES, CLI_HEADER_LEN, CLI_MAGIC,
        MAX_CLI_PAYLOAD_BYTES,
    };
    use crate::emitter::error::EmitterError;
    use crate::envelope::EnvelopeKind;
    use serde::{Deserialize, Serialize};

    #[test]
    fn cli_magic_is_vbli() {
        assert_eq!(CLI_MAGIC, 0x5642_4C49);
        assert_eq!(b'V', 0x56);
        assert_eq!(b'B', 0x42);
        assert_eq!(b'L', 0x4C);
        assert_eq!(b'I', 0x49);
    }

    #[test]
    fn cli_header_length_is_52() {
        assert_eq!(CLI_HEADER_LEN, 52);
        assert_eq!(CLI_HEADER_BYTES, 52);
        assert_eq!(CLI_CRC_OFFSET, 48);
    }

    #[test]
    fn emitter_error_display() {
        let err = EmitterError::BadMagic { found: 0xDEAD_BEEF };
        assert!(format!("{}", err).contains("0xdeadbeef"));

        let err = EmitterError::PayloadTooLarge { len: 100, max: 50 };
        assert!(format!("{}", err).contains("100"));
        assert!(format!("{}", err).contains("50"));

        let err = EmitterError::MigrationRequired { from: 0, to: 1 };
        assert!(format!("{}", err).contains("migration"));
    }

    #[test]
    fn build_cli_header_produces_correct_length() {
        let payload = b"test payload";
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("header build should succeed");
        assert_eq!(header.len(), CLI_HEADER_BYTES);
    }

    #[test]
    fn cli_header_roundtrip() {
        let original_payload = b"hello world";
        let header = build_cli_header(
            EnvelopeKind::Success,
            original_payload.len() as u32,
            original_payload,
        )
        .expect("header build should succeed");

        let decoded = decode_cli_header(&header).expect("header decode should succeed");
        assert_eq!(decoded.magic, CLI_MAGIC);
        assert_eq!(decoded.schema_version, BINARY_SCHEMA_VERSION);
        assert_eq!(decoded.kind, EnvelopeKind::Success as u16);
        assert_eq!(decoded.header_len, CLI_HEADER_LEN);
        assert_eq!(decoded.payload_len, original_payload.len() as u32);
    }

    #[test]
    fn encode_decode_postcard_roundtrip() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct TestPayload {
            message: String,
            value: i32,
        }

        let payload = TestPayload {
            message: "test".to_string(),
            value: 42,
        };

        let encoded = encode_postcard(&payload, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES)
            .expect("encode should succeed");

        assert!(
            encoded.len() >= CLI_HEADER_BYTES + 1,
            "encoded size should include header and some payload"
        );

        let decoded: TestPayload =
            decode_postcard(&encoded, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES)
                .expect("decode should succeed");
        assert_eq!(decoded.message, "test");
        assert_eq!(decoded.value, 42);
    }

    #[test]
    fn postcard_rejects_wrong_kind() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct TestPayload {
            data: String,
        }

        let payload = TestPayload {
            data: "test".to_string(),
        };

        let encoded = encode_postcard(&payload, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES)
            .expect("encode should succeed");

        let result =
            decode_postcard::<TestPayload>(&encoded, EnvelopeKind::Error, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(result, Err(EmitterError::UnknownKind { .. })));
    }

    #[test]
    fn postcard_rejects_bad_magic() {
        let mut bytes = vec![0xFFu8; CLI_HEADER_BYTES + 10];
        let header =
            build_cli_header(EnvelopeKind::Success, 10, &[0u8; 10]).expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);

        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        bytes[2] = 0xFF;
        bytes[3] = 0xFF;

        let checksum = crc32c::crc32c(&bytes[..CLI_CRC_OFFSET]);
        bytes[CLI_CRC_OFFSET..CLI_CRC_OFFSET.saturating_add(4)]
            .copy_from_slice(&checksum.to_le_bytes());

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(result, Err(EmitterError::BadMagic { .. })));
    }

    #[test]
    fn postcard_rejects_bad_crc() {
        let payload = b"test payload for crc test";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        bytes[10] ^= 0xFF;

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(result, Err(EmitterError::HeaderChecksumMismatch)));
    }

    #[test]
    fn postcard_rejects_bad_payload_digest() {
        let payload = b"original payload";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        if let Some(byte) = bytes.get_mut(CLI_HEADER_BYTES) {
            *byte ^= 0xFF;
        }

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(result, Err(EmitterError::PayloadDigestMismatch)));
    }

    #[test]
    fn postcard_rejects_payload_too_large() {
        let payload = b"small payload";
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        let mut bytes = Vec::with_capacity(CLI_HEADER_BYTES + payload.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(payload);

        let result = decode_postcard::<String>(&bytes, EnvelopeKind::Success, 5);
        assert!(matches!(result, Err(EmitterError::PayloadTooLarge { .. })));
    }

    #[test]
    fn postcard_rejects_empty_input_before_payload_exposure() {
        let result = decode_postcard::<String>(&[], EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert_eq!(result, Err(EmitterError::UnexpectedEof));
    }

    #[test]
    fn postcard_rejects_truncated_header_before_payload_exposure() {
        let bytes = vec![0u8; CLI_HEADER_BYTES - 1];

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);

        assert_eq!(result, Err(EmitterError::UnexpectedEof));
    }

    #[test]
    fn postcard_rejects_header_length_mismatch_before_payload_exposure() {
        let payload = b"valid payload";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        bytes[8..12].copy_from_slice(&51u32.to_le_bytes());
        let checksum = crc32c::crc32c(&bytes[..CLI_CRC_OFFSET]);
        bytes[CLI_CRC_OFFSET..CLI_CRC_OFFSET.saturating_add(4)]
            .copy_from_slice(&checksum.to_le_bytes());

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);

        assert_eq!(
            result,
            Err(EmitterError::HeaderLengthMismatch { found: 51 })
        );
    }

    #[test]
    fn postcard_payload_bound_accepts_exact_max_and_rejects_max_plus_one() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct BoundedPayload {
            value: u8,
        }

        let payload = BoundedPayload { value: 7 };
        let encoded = encode_postcard(&payload, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES)
            .expect("encode should succeed");
        let payload_len = encoded.len() - CLI_HEADER_BYTES;
        let exact_max = match u32::try_from(payload_len) {
            Ok(value) => value,
            Err(error) => panic!("payload length must fit u32: {error}"),
        };

        let accepted: Result<BoundedPayload, EmitterError> =
            decode_postcard(&encoded, EnvelopeKind::Success, exact_max);
        assert_eq!(accepted, Ok(BoundedPayload { value: 7 }));

        let below_bound = exact_max.saturating_sub(1);
        let rejected: Result<BoundedPayload, EmitterError> =
            decode_postcard(&encoded, EnvelopeKind::Success, below_bound);
        assert_eq!(
            rejected,
            Err(EmitterError::PayloadTooLarge {
                len: exact_max,
                max: below_bound
            })
        );
    }

    #[test]
    fn postcard_rejects_unsupported_version() {
        let payload = b"test";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        bytes[4] = 0xFF;
        bytes[5] = 0xFF;

        let checksum = crc32c::crc32c(&bytes[..CLI_CRC_OFFSET]);
        bytes[CLI_CRC_OFFSET..CLI_CRC_OFFSET.saturating_add(4)]
            .copy_from_slice(&checksum.to_le_bytes());

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(
            result,
            Err(EmitterError::UnsupportedSchemaVersion { .. })
        ));
    }

    #[test]
    fn postcard_rejects_old_version() {
        let payload = b"test";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        bytes[4] = 0x00;
        bytes[5] = 0x00;

        let checksum = crc32c::crc32c(&bytes[..CLI_CRC_OFFSET]);
        bytes[CLI_CRC_OFFSET..CLI_CRC_OFFSET.saturating_add(4)]
            .copy_from_slice(&checksum.to_le_bytes());

        let result =
            decode_postcard::<String>(&bytes, EnvelopeKind::Success, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(
            result,
            Err(EmitterError::MigrationRequired { .. })
        ));
    }

    #[cfg(kani)]
    mod emitter_proofs {
        include!("../../../kani/vb-qi37.13.3/emitter_proofs.rs");
    }
}

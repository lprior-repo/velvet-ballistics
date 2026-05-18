#[cfg(test)]
mod tests {
    use crate::emitter::binary::{
        BINARY_SCHEMA_VERSION, CLI_CRC_OFFSET, CLI_HEADER_BYTES, CLI_HEADER_LEN, CLI_MAGIC,
        MAX_CLI_PAYLOAD_BYTES, build_cli_header, decode_cli_header, decode_postcard,
        encode_postcard,
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

    // =====================================================================
    // EmitterError Display — all untested variants
    // =====================================================================

    #[test]
    fn emitter_error_display_yaml_encode_failed() {
        let err = EmitterError::YamlEncodeFailed;
        let s = format!("{}", err);
        assert!(
            s.contains("YAML") || s.contains("encoding"),
            "should mention YAML encoding: {s}"
        );
    }

    #[test]
    fn emitter_error_display_postcard_encode_failed() {
        let err = EmitterError::PostcardEncodeFailed;
        let s = format!("{}", err);
        assert!(
            s.contains("Postcard") || s.contains("encoding"),
            "should mention Postcard encoding: {s}"
        );
    }

    #[test]
    fn emitter_error_display_postcard_decode_failed() {
        let err = EmitterError::PostcardDecodeFailed;
        let s = format!("{}", err);
        assert!(
            s.contains("Postcard") || s.contains("decoding"),
            "should mention Postcard decoding: {s}"
        );
    }

    #[test]
    fn emitter_error_display_length_overflow() {
        let err = EmitterError::LengthOverflow;
        let s = format!("{}", err);
        assert!(
            s.contains("length") || s.contains("overflow"),
            "should mention length overflow: {s}"
        );
    }

    #[test]
    fn emitter_error_display_payload_digest_mismatch() {
        let err = EmitterError::PayloadDigestMismatch;
        let s = format!("{}", err);
        assert!(
            s.contains("digest") || s.contains("mismatch"),
            "should mention digest mismatch: {s}"
        );
    }

    #[test]
    fn emitter_error_display_unexpected_eof() {
        let err = EmitterError::UnexpectedEof;
        let s = format!("{}", err);
        assert!(
            s.contains("EOF") || s.contains("shorter"),
            "should mention unexpected EOF: {s}"
        );
    }

    #[test]
    fn emitter_error_display_header_length_mismatch() {
        let err = EmitterError::HeaderLengthMismatch { found: 51 };
        let s = format!("{}", err);
        assert!(
            s.contains("51") || s.contains("header"),
            "should mention header length: {s}"
        );
    }

    #[test]
    fn emitter_error_display_unsupported_schema_version() {
        let err = EmitterError::UnsupportedSchemaVersion { version: 99 };
        let s = format!("{}", err);
        assert!(
            s.contains("99") || s.contains("unsupported"),
            "should mention version: {s}"
        );
    }

    #[test]
    fn emitter_error_display_migration_required() {
        let err = EmitterError::MigrationRequired { from: 0, to: 1 };
        let s = format!("{}", err);
        assert!(
            s.contains("0") && s.contains("1") && s.contains("migration"),
            "should show from/to: {s}"
        );
    }

    #[test]
    fn emitter_error_display_payload_length_overflow() {
        let err = EmitterError::PayloadLengthOverflow { len: 4_000_000_000 };
        let s = format!("{}", err);
        assert!(
            s.contains("overflow") || s.contains("length"),
            "should mention overflow: {s}"
        );
    }

    #[test]
    fn emitter_error_display_unknown_kind() {
        let err = EmitterError::UnknownKind { kind: 99 };
        let s = format!("{}", err);
        assert!(
            s.contains("99") || s.contains("unknown"),
            "should mention unknown kind: {s}"
        );
    }

    #[test]
    fn emitter_error_display_ansi_forbidden() {
        let err = EmitterError::AnsiForbidden;
        let s = format!("{}", err);
        assert!(
            s.contains("ANSI") || s.contains("escape"),
            "should mention ANSI: {s}"
        );
    }

    #[test]
    fn emitter_error_display_header_checksum_mismatch() {
        let err = EmitterError::HeaderChecksumMismatch;
        let s = format!("{}", err);
        assert!(
            s.contains("CRC") || s.contains("checksum") || s.contains("mismatch"),
            "should mention CRC/checksum: {s}"
        );
    }

    // =====================================================================
    // EmitterError Eq + Clone
    // =====================================================================

    #[test]
    fn emitter_error_eq_and_clone() {
        let err1 = EmitterError::PayloadTooLarge { len: 10, max: 5 };
        let err2 = EmitterError::PayloadTooLarge { len: 10, max: 5 };
        let err3 = EmitterError::PayloadTooLarge { len: 20, max: 5 };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
        let cloned = err1.clone();
        assert_eq!(err1, cloned);
    }

    #[test]
    fn emitter_error_bad_magic_debug_format() {
        let err = EmitterError::BadMagic { found: 0x5642_4C49 };
        let debug = format!("{:?}", err);
        assert!(
            debug.contains("BadMagic"),
            "debug should contain BadMagic: {debug}"
        );
        // The found value should appear somewhere in the debug output
        assert!(
            debug.contains("found"),
            "debug should mention 'found': {debug}"
        );
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
}

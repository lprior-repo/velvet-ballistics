#![forbid(unsafe_code)]

//! CLI text and binary envelope emitters.
//!
//! This module provides YAML text emission and Postcard binary emission
//! for CLI output envelopes following the v1 schema contract.
//!
//! # Binary Format (52-byte header)
//!
//! - bytes 0..4:   magic `VBLI` (0x56424C49)
//! - bytes 4..6:   schema version (u16 LE)
//! - bytes 6..8:   kind (u16 LE)
//! - bytes 8..12:  header_len (u32 LE = 52)
//! - bytes 12..16: payload_len (u32 LE)
//! - bytes 16..48: payload BLAKE3 digest (32 bytes)
//! - bytes 48..52: CRC32C of bytes 0..47
//!
//! # CRC Scope
//!
//! CRC32C is computed over bytes 0..47 only (header without the CRC field itself).
//!
//! # Digest Scope
//!
//! BLAKE3 digest is computed over the Postcard-serialized payload bytes only.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use saphyr::{Mapping, Scalar, Yaml, YamlEmitter};

use crate::envelope::{DiagnosticEntry, EnvelopeKind, OutputEnvelope};

/// CLI magic bytes: "VBLI" = 0x56424C49
pub const CLI_MAGIC: u32 = 0x5642_4C49;

/// Fixed CLI binary header length in bytes.
pub const CLI_HEADER_LEN: u32 = 52;

/// CLI binary header length in bytes as usize for array indexing.
pub const CLI_HEADER_BYTES: usize = 52;

/// CRC offset in the CLI header (bytes 0..47 are covered by CRC).
pub const CLI_CRC_OFFSET: usize = 48;

/// Digest byte width (BLAKE3 output).
pub const DIGEST_BYTES: usize = 32;

/// Maximum CLI payload size (16MB).
pub const MAX_CLI_PAYLOAD_BYTES: u32 = 16_777_216;

/// Text schema version string for YAML output.
pub const TEXT_SCHEMA_VERSION: &str = "velvet-ballastics/cli-output/v1";

/// Binary schema version for postcard output.
pub const BINARY_SCHEMA_VERSION: u16 = 1;

/// Emitter-specific errors for CLI output operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitterError {
    /// YAML serialization failed.
    YamlEncodeFailed,
    /// Postcard serialization failed.
    PostcardEncodeFailed,
    /// Postcard deserialization failed.
    PostcardDecodeFailed,
    /// Payload exceeds caller-provided maximum.
    PayloadTooLarge { len: u32, max: u32 },
    /// Host integer conversion would overflow.
    LengthOverflow,
    /// CRC32C validation failed.
    HeaderChecksumMismatch,
    /// BLAKE3 digest validation failed.
    PayloadDigestMismatch,
    /// Envelope bytes are shorter than declared header.
    UnexpectedEof,
    /// Binary header has wrong magic.
    BadMagic { found: u32 },
    /// Binary header length is not the fixed 52 bytes.
    HeaderLengthMismatch { found: u32 },
    /// Binary schema version is older than current and requires migration.
    MigrationRequired { from: u16, to: u16 },
    /// Binary schema version is newer or not supported.
    UnsupportedSchemaVersion { version: u16 },
    /// Payload length field overflows usize during allocation.
    PayloadLengthOverflow { len: u32 },
    /// Envelope kind is not recognized.
    UnknownKind { kind: u16 },
    /// Input contains ANSI escape sequences (forbidden in machine output).
    AnsiForbidden,
}

impl fmt::Display for EmitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitterError::YamlEncodeFailed => write!(f, "YAML encoding failed"),
            EmitterError::PostcardEncodeFailed => write!(f, "Postcard encoding failed"),
            EmitterError::PostcardDecodeFailed => write!(f, "Postcard decoding failed"),
            EmitterError::PayloadTooLarge { len, max } => {
                write!(f, "payload length {} exceeds maximum {}", len, max)
            }
            EmitterError::LengthOverflow => write!(f, "length overflow in header computation"),
            EmitterError::HeaderChecksumMismatch => write!(f, "CRC32C header checksum mismatch"),
            EmitterError::PayloadDigestMismatch => write!(f, "BLAKE3 payload digest mismatch"),
            EmitterError::UnexpectedEof => write!(f, "envelope bytes shorter than declared header"),
            EmitterError::BadMagic { found } => {
                write!(f, "wrong magic bytes: found {found:#x}, expected VBLI")
            }
            EmitterError::HeaderLengthMismatch { found } => {
                write!(f, "header length {} is not the expected 52 bytes", found)
            }
            EmitterError::MigrationRequired { from, to } => {
                write!(
                    f,
                    "binary schema version {} requires migration to {}",
                    from, to
                )
            }
            EmitterError::UnsupportedSchemaVersion { version } => {
                write!(f, "unsupported binary schema version: {}", version)
            }
            EmitterError::PayloadLengthOverflow { len } => {
                write!(f, "payload length {} would overflow during allocation", len)
            }
            EmitterError::UnknownKind { kind } => {
                write!(f, "unknown envelope kind: {}", kind)
            }
            EmitterError::AnsiForbidden => {
                write!(f, "ANSI escape sequences are forbidden in machine output")
            }
        }
    }
}

/// YAML-emittable representation of a CLI output envelope.
///
/// This is the shape we serialize to YAML for `--emit yaml` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlEnvelope {
    pub schema_version: String,
    pub kind: String,
    pub command: String,
    pub exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticEntry>>,
}

impl YamlEnvelope {
    /// Creates a new YAML envelope from an OutputEnvelope.
    pub fn from_envelope(envelope: &OutputEnvelope, exit_code: u8) -> Self {
        Self {
            schema_version: TEXT_SCHEMA_VERSION.to_string(),
            kind: envelope.kind.name().to_string(),
            command: envelope.metadata.command.clone(),
            exit_code,
            data: envelope.data.as_ref().map(|p| p.as_json().clone()),
            diagnostics: None, // Diagnostics go to stderr, not stdout
        }
    }
}

/// Encodes an envelope payload as YAML text.
///
/// # Errors
///
/// Returns `EmitterError::YamlEncodeFailed` if YAML serialization fails.
#[cfg(feature = "std")]
pub fn encode_yaml<T: Serialize>(payload: &T) -> Result<String, EmitterError> {
    // Manual YAML building to ensure consistent structure
    let json_value = serde_json::to_value(payload).map_err(|_| EmitterError::YamlEncodeFailed)?;
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);

    let doc = json_value_to_yaml(&json_value)?;
    emitter
        .dump(&doc)
        .map_err(|_| EmitterError::YamlEncodeFailed)?;
    Ok(output)
}

/// Converts a JSON value to a saphyr Yaml value recursively.
#[cfg(feature = "std")]
fn json_value_to_yaml(value: &serde_json::Value) -> Result<Yaml<'static>, EmitterError> {
    use alloc::borrow::Cow;
    match value {
        serde_json::Value::Null => Ok(Yaml::Value(Scalar::Null)),
        serde_json::Value::Bool(b) => Ok(Yaml::Value(Scalar::Boolean(*b))),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Yaml::Value(Scalar::Integer(i)))
            } else if let Some(u) = n.as_u64() {
                let val = i64::try_from(u).unwrap_or(i64::MAX);
                Ok(Yaml::Value(Scalar::Integer(val)))
            } else if let Some(f) = n.as_f64() {
                // Use string representation for floats since Scalar doesn't have Float variant
                Ok(Yaml::Value(Scalar::String(Cow::Owned(f.to_string()))))
            } else {
                Ok(Yaml::Value(Scalar::Null))
            }
        }
        serde_json::Value::String(s) => Ok(Yaml::Value(Scalar::String(Cow::Owned(s.clone())))),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_value_to_yaml).collect();
            items.map(Yaml::Sequence)
        }
        serde_json::Value::Object(obj) => {
            let mut mapping = Mapping::new();
            for (k, v) in obj {
                let key = Yaml::Value(Scalar::String(Cow::Owned(k.clone())));
                let val = json_value_to_yaml(v)?;
                mapping.insert(key, val);
            }
            Ok(Yaml::Mapping(mapping))
        }
    }
}

/// Encodes a payload as Postcard bytes.
pub fn encode_postcard<T: Serialize + core::fmt::Debug>(
    payload: &T,
    kind: EnvelopeKind,
    max_payload_len: u32,
) -> Result<Vec<u8>, EmitterError> {
    let payload_bytes =
        postcard::to_allocvec(payload).map_err(|_| EmitterError::PostcardEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| EmitterError::PayloadLengthOverflow {
            len: u32::try_from(payload_bytes.len()).unwrap_or(u32::MAX),
        })?;

    if payload_len > max_payload_len {
        return Err(EmitterError::PayloadTooLarge {
            len: payload_len,
            max: max_payload_len,
        });
    }

    let capacity = CLI_HEADER_BYTES
        .checked_add(payload_bytes.len())
        .ok_or(EmitterError::LengthOverflow)?;

    let header = build_cli_header(kind, payload_len, &payload_bytes)?;

    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(&header);
    encoded.extend_from_slice(&payload_bytes);
    Ok(encoded)
}

/// Decodes a Postcard envelope from bytes.
pub fn decode_postcard<'a, T: Deserialize<'a> + core::fmt::Debug>(
    bytes: &'a [u8],
    expected_kind: EnvelopeKind,
    max_payload_len: u32,
) -> Result<T, EmitterError> {
    if bytes.len() < CLI_HEADER_BYTES {
        return Err(EmitterError::UnexpectedEof);
    }

    let header = decode_cli_header(bytes)?;

    // Validate magic
    if header.magic != CLI_MAGIC {
        return Err(EmitterError::BadMagic {
            found: header.magic,
        });
    }

    // Validate schema version
    if header.schema_version < BINARY_SCHEMA_VERSION {
        return Err(EmitterError::MigrationRequired {
            from: header.schema_version,
            to: BINARY_SCHEMA_VERSION,
        });
    }
    if header.schema_version > BINARY_SCHEMA_VERSION {
        return Err(EmitterError::UnsupportedSchemaVersion {
            version: header.schema_version,
        });
    }

    // Validate kind
    let kind_val = header.kind;
    #[allow(clippy::as_conversions)]
    let expected_u16 = expected_kind as u16;
    if kind_val != expected_u16 {
        return Err(EmitterError::UnknownKind { kind: kind_val });
    }

    // Validate header length
    if header.header_len != CLI_HEADER_LEN {
        return Err(EmitterError::HeaderLengthMismatch {
            found: header.header_len,
        });
    }

    // Validate payload length against max
    if header.payload_len > max_payload_len {
        return Err(EmitterError::PayloadTooLarge {
            len: header.payload_len,
            max: max_payload_len,
        });
    }

    // Extract payload bytes
    let payload_start = CLI_HEADER_BYTES;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| EmitterError::PayloadLengthOverflow {
            len: header.payload_len,
        })?;
    let payload_end = payload_start.checked_add(payload_len_usize).ok_or(
        EmitterError::PayloadLengthOverflow {
            len: header.payload_len,
        },
    )?;

    if bytes.len() < payload_end {
        return Err(EmitterError::UnexpectedEof);
    }

    let payload_bytes = bytes
        .get(payload_start..payload_end)
        .ok_or(EmitterError::UnexpectedEof)?;

    // Verify payload digest
    let computed_digest = blake3::hash(payload_bytes);
    if computed_digest.as_bytes() != &header.payload_digest {
        return Err(EmitterError::PayloadDigestMismatch);
    }

    // Decode payload
    postcard::from_bytes(payload_bytes).map_err(|_| EmitterError::PostcardDecodeFailed)
}

/// CLI binary header structure.
#[derive(Debug, Clone, Copy)]
struct CliHeader {
    magic: u32,
    schema_version: u16,
    kind: u16,
    header_len: u32,
    payload_len: u32,
    payload_digest: [u8; DIGEST_BYTES],
    #[allow(dead_code)]
    header_checksum: u32,
}

/// Reads a u16 from bytes at the given offset.
fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, EmitterError> {
    let slice = bytes.get(offset..).ok_or(EmitterError::UnexpectedEof)?;
    if slice.len() < 2 {
        return Err(EmitterError::UnexpectedEof);
    }
    let arr: [u8; 2] = slice
        .get(..2)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;
    Ok(u16::from_le_bytes(arr))
}

/// Reads a u32 from bytes at the given offset.
fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, EmitterError> {
    let slice = bytes.get(offset..).ok_or(EmitterError::UnexpectedEof)?;
    if slice.len() < 4 {
        return Err(EmitterError::UnexpectedEof);
    }
    let arr: [u8; 4] = slice
        .get(..4)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(arr))
}

/// Writes a u16 to bytes at the given offset.
fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), EmitterError> {
    let slice = bytes
        .get_mut(offset..offset.saturating_add(2))
        .ok_or(EmitterError::UnexpectedEof)?;
    slice.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Writes a u32 to bytes at the given offset.
fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), EmitterError> {
    let slice = bytes
        .get_mut(offset..offset.saturating_add(4))
        .ok_or(EmitterError::UnexpectedEof)?;
    slice.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Builds the 52-byte CLI binary header.
fn build_cli_header(
    kind: EnvelopeKind,
    payload_len: u32,
    payload_bytes: &[u8],
) -> Result<[u8; CLI_HEADER_BYTES], EmitterError> {
    let mut header = [0u8; CLI_HEADER_BYTES];

    // Magic (4 bytes)
    write_u32(&mut header, 0, CLI_MAGIC)?;

    // Schema version (2 bytes)
    write_u16(&mut header, 4, BINARY_SCHEMA_VERSION)?;

    // Kind (2 bytes)
    #[allow(clippy::as_conversions)]
    let kind_u16 = kind as u16;
    write_u16(&mut header, 6, kind_u16)?;

    // Header length (4 bytes) - always 52
    write_u32(&mut header, 8, CLI_HEADER_LEN)?;

    // Payload length (4 bytes)
    write_u32(&mut header, 12, payload_len)?;

    // Payload digest (32 bytes) - BLAKE3 of payload only
    let digest = blake3::hash(payload_bytes);
    let digest_bytes = header.get_mut(16..48).ok_or(EmitterError::UnexpectedEof)?;
    digest_bytes.copy_from_slice(digest.as_bytes());

    // CRC (4 bytes) - CRC32C of bytes 0..47
    let checksum = crc32c::crc32c(&header[..CLI_CRC_OFFSET]);
    write_u32(&mut header, CLI_CRC_OFFSET, checksum)?;

    Ok(header)
}

/// Decodes and validates the CLI binary header.
fn decode_cli_header(bytes: &[u8]) -> Result<CliHeader, EmitterError> {
    // Read all header fields
    let magic = read_u32(bytes, 0)?;
    let schema_version = read_u16(bytes, 4)?;
    let kind = read_u16(bytes, 6)?;
    let header_len = read_u32(bytes, 8)?;
    let payload_len = read_u32(bytes, 12)?;

    // Read digest
    let payload_digest = bytes
        .get(16..48)
        .ok_or(EmitterError::UnexpectedEof)?
        .try_into()
        .map_err(|_| EmitterError::UnexpectedEof)?;

    // Read checksum
    let header_checksum = read_u32(bytes, CLI_CRC_OFFSET)?;

    // Validate CRC
    let crc_slice = bytes
        .get(..CLI_CRC_OFFSET)
        .ok_or(EmitterError::UnexpectedEof)?;
    let computed_crc = crc32c::crc32c(crc_slice);
    if computed_crc != header_checksum {
        return Err(EmitterError::HeaderChecksumMismatch);
    }

    Ok(CliHeader {
        magic,
        schema_version,
        kind,
        header_len,
        payload_len,
        payload_digest,
        header_checksum,
    })
}

/// Validates that a string does not contain ANSI escape sequences.
pub fn validate_no_ansi(text: &str) -> Result<(), EmitterError> {
    if text.contains('\x1B') {
        return Err(EmitterError::AnsiForbidden);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{MetadataEnvelope, OutputEnvelope, PayloadEnvelope, SchemaVersion};
    use vb_core::ids::RunId;

    #[test]
    fn cli_magic_is_vbli() {
        assert_eq!(CLI_MAGIC, 0x5642_4C49);
        // Verify ASCII interpretation
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

        // Decode and verify
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

        // Verify header structure - minimum length check
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

        // Try to decode with wrong kind
        let result =
            decode_postcard::<TestPayload>(&encoded, EnvelopeKind::Error, MAX_CLI_PAYLOAD_BYTES);
        assert!(matches!(result, Err(EmitterError::UnknownKind { .. })));
    }

    #[test]
    fn postcard_rejects_bad_magic() {
        let mut bytes = vec![0xFFu8; CLI_HEADER_BYTES + 10];
        // Write valid header first
        let header =
            build_cli_header(EnvelopeKind::Success, 10, &[0u8; 10]).expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);

        // Corrupt magic
        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        bytes[2] = 0xFF;
        bytes[3] = 0xFF;

        // Recompute CRC after corrupting magic so we get to the magic check
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

        // Corrupt a byte in the header (not the CRC field itself)
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

        // Corrupt payload
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

        // Try to decode with smaller max
        let result = decode_postcard::<String>(&bytes, EnvelopeKind::Success, 5);
        assert!(matches!(result, Err(EmitterError::PayloadTooLarge { .. })));
    }

    #[test]
    fn postcard_rejects_unsupported_version() {
        let payload = b"test";
        let mut bytes = vec![0u8; CLI_HEADER_BYTES + payload.len()];
        let header = build_cli_header(EnvelopeKind::Success, payload.len() as u32, payload)
            .expect("build should succeed");
        bytes[..CLI_HEADER_BYTES].copy_from_slice(&header);
        bytes[CLI_HEADER_BYTES..].copy_from_slice(payload);

        // Write future version
        bytes[4] = 0xFF;
        bytes[5] = 0xFF;

        // Recompute CRC
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

        // Write version 0
        bytes[4] = 0x00;
        bytes[5] = 0x00;

        // Recompute CRC
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

    #[test]
    fn validate_no_ansi_accepts_plain_text() {
        assert!(validate_no_ansi("hello world").is_ok());
        assert!(validate_no_ansi("").is_ok());
        assert!(validate_no_ansi("line1\nline2").is_ok());
    }

    #[test]
    fn validate_no_ansi_rejects_ansi() {
        assert!(matches!(
            validate_no_ansi("\x1B[31mred text\x1B[0m"),
            Err(EmitterError::AnsiForbidden)
        ));
        assert!(matches!(
            validate_no_ansi("\x1B[1;2;3m"),
            Err(EmitterError::AnsiForbidden)
        ));
    }

    #[cfg(kani)]
    mod emitter_proofs {
        include!("../../kani/vb-qi37.13.3/emitter_proofs.rs");
    }

    #[cfg(feature = "std")]
    #[test]
    fn yaml_envelope_from_envelope() {
        let run_id = RunId::new(123);
        let metadata = MetadataEnvelope::new(run_id, "verify".to_string(), 1000);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"passed": true}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            Some(payload),
            Vec::new(),
        )
        .expect("envelope build should succeed");

        let yaml_env = YamlEnvelope::from_envelope(&envelope, 0);

        assert_eq!(yaml_env.schema_version, TEXT_SCHEMA_VERSION);
        assert_eq!(yaml_env.kind, "Success");
        assert_eq!(yaml_env.command, "verify");
        assert_eq!(yaml_env.exit_code, 0);
        assert!(yaml_env.data.is_some());
        assert!(yaml_env.diagnostics.is_none());
    }
}

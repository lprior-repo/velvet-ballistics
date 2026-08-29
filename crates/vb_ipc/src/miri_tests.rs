//! Miri-testable harnesses proving IPC frame/codec/ingress functions handle
//! truncated/malformed data safely.
//!
//! These tests verify that no UB occurs on malformed input - all decode
//! functions must return errors rather than panic or cause UB.

#![forbid(unsafe_code)]

use std::panic::catch_unwind;

use vb_core::ids::{RunId, WorkflowDigest};

use crate::bounded::{BoundedPayload, MaxPayloadBytes};
use crate::codec::{decode_payload, encode_payload};
use crate::commands::IpcCommand;
use crate::constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
use crate::error::IpcError;
use crate::frame::{decode_frame_header, validate_frame_bounds, validate_frame_magic};
use crate::frame_types::{decode_frame, IpcFrame, IpcFrameHeader};
use crate::payloads::{IpcPayload, SubmitRunPayload};

// ---------------------------------------------------------------------------
// Frame magic validation
// ---------------------------------------------------------------------------

fn panic_free_validate_magic(bytes: &[u8]) -> Result<(), IpcError> {
    catch_unwind(|| validate_frame_magic(bytes)).expect("validate_frame_magic must not panic")
}

#[test]
fn validate_magic_on_empty_input_returns_error() {
    let result = panic_free_validate_magic(&[]);
    assert!(
        matches!(result, Err(IpcError::HeaderDecodeFailed)),
        "empty input must yield HeaderDecodeFailed, got {:?}",
        result
    );
}

#[test]
fn validate_magic_on_three_bytes_returns_error() {
    let result = panic_free_validate_magic(&[0x54, 0x4C, 0x42]);
    assert!(
        matches!(result, Err(IpcError::HeaderDecodeFailed)),
        "3 bytes must yield HeaderDecodeFailed (insufficient data), got {:?}",
        result
    );
}

#[test]
fn validate_magic_on_correct_magic_succeeds() {
    let result = panic_free_validate_magic(&IPC_MAGIC.to_le_bytes());
    assert!(
        result.is_ok(),
        "correct magic must succeed, got {:?}",
        result
    );
}

#[test]
fn validate_magic_on_wrong_magic_returns_error() {
    let result = panic_free_validate_magic(&0xDEADBEEFu32.to_le_bytes());
    assert!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "wrong magic must yield InvalidMagic, got {:?}",
        result
    );
}

#[test]
fn validate_magic_on_all_ff_returns_error() {
    let result = panic_free_validate_magic(&[0xFF; 4]);
    assert!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "all-0xFF bytes must yield InvalidMagic, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Frame header decode
// ---------------------------------------------------------------------------

fn panic_free_decode_header(bytes: &[u8; IPC_HEADER_LEN]) -> Result<IpcFrameHeader, IpcError> {
    catch_unwind(|| decode_frame_header(bytes)).expect("decode_frame_header must not panic")
}

#[test]
fn decode_header_on_truncated_slice_returns_error() {
    let truncated = [0u8; 10];
    let result = panic_free_decode_header(&truncated);
    assert!(
        matches!(result, Err(IpcError::HeaderDecodeFailed)),
        "truncated header must yield HeaderDecodeFailed, got {:?}",
        result
    );
}

#[test]
fn decode_header_with_valid_magic_all_zeros_yields_error() {
    // All zeros have magic = 0, not IPC_MAGIC
    let bytes = [0u8; IPC_HEADER_LEN];
    let result = panic_free_decode_header(&bytes);
    assert!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "all-zeros header must yield InvalidMagic, got {:?}",
        result
    );
}

#[test]
fn decode_header_with_bad_version_returns_error() {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    // Write correct magic at offset 0..4
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    // Write version 99 at offset 4..6
    bytes[4..6].copy_from_slice(&99_u16.to_le_bytes());
    let result = panic_free_decode_header(&bytes);
    assert!(
        matches!(result, Err(IpcError::UnsupportedVersion { .. })),
        "bad version must yield UnsupportedVersion, got {:?}",
        result
    );
}

#[test]
fn decode_header_with_valid_header_returns_ok() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 0);
    let bytes = header.encode().expect("encode must succeed");
    let result = panic_free_decode_header(&bytes);
    assert!(
        result.is_ok(),
        "valid header must decode OK, got {:?}",
        result
    );
    let decoded = result.unwrap();
    assert_eq!(decoded.command, IpcCommand::Health);
    assert_eq!(decoded.correlation, 42);
}

#[test]
fn decode_header_with_reserved_nonzero_returns_error() {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    bytes[8..10].copy_from_slice(&0_u16.to_le_bytes());
    // Non-zero reserved at offset 10..12
    bytes[10..12].copy_from_slice(&0x0001_u16.to_le_bytes());
    let result = panic_free_decode_header(&bytes);
    assert!(
        matches!(result, Err(IpcError::ReservedNonZero { .. })),
        "non-zero reserved must yield ReservedNonZero, got {:?}",
        result
    );
}

#[test]
fn decode_header_with_oversized_payload_len_returns_error() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, u32::MAX);
    let bytes = header.encode().expect("encode must succeed");
    let result = panic_free_decode_header(&bytes);
    assert!(
        matches!(result, Err(IpcError::PayloadTooLarge { .. })),
        "oversized payload_len must yield PayloadTooLarge, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Frame payload decode
// ---------------------------------------------------------------------------

fn panic_free_validate_bounds(
    header: &IpcFrameHeader,
    max_payload: MaxPayloadBytes,
) -> Result<(), IpcError> {
    catch_unwind(|| validate_frame_bounds(header, max_payload))
        .expect("validate_frame_bounds must not panic")
}

#[test]
fn validate_bounds_on_valid_header_succeeds() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 5);
    let max = MaxPayloadBytes::DEFAULT;
    let result = panic_free_validate_bounds(&header, max);
    assert!(
        result.is_ok(),
        "valid bounds must succeed, got {:?}",
        result
    );
}

#[test]
fn validate_bounds_with_zero_payload_succeeds() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
    let max = MaxPayloadBytes::DEFAULT;
    let result = panic_free_validate_bounds(&header, max);
    assert!(
        result.is_ok(),
        "zero payload must succeed, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Full frame decode
// ---------------------------------------------------------------------------

fn panic_free_decode_frame(
    header: &[u8; IPC_HEADER_LEN],
    payload: bytes::Bytes,
    max_payload: MaxPayloadBytes,
) -> Result<IpcFrame, IpcError> {
    catch_unwind(|| decode_frame(header, payload, max_payload))
        .expect("decode_frame must not panic")
}

#[test]
fn decode_frame_with_valid_health_payload_succeeds() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
    let bytes = header.encode().expect("encode must succeed");
    let payload = bytes::Bytes::from(Vec::new());
    let result = panic_free_decode_frame(&bytes, payload, MaxPayloadBytes::DEFAULT);
    assert!(
        result.is_ok(),
        "valid frame must decode OK, got {:?}",
        result
    );
}

#[test]
fn decode_frame_with_length_mismatch_returns_error() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 5);
    let bytes = header.encode().expect("encode must succeed");
    // Payload is shorter than declared
    let payload = bytes::Bytes::from(vec![0u8; 3]);
    let result = panic_free_decode_frame(&bytes, payload, MaxPayloadBytes::DEFAULT);
    assert!(
        matches!(result, Err(IpcError::PayloadLengthMismatch { .. })),
        "length mismatch must yield PayloadLengthMismatch, got {:?}",
        result
    );
}

#[test]
fn decode_frame_with_truncated_header_returns_error() {
    let truncated = [0u8; 10];
    let payload = bytes::Bytes::from(Vec::new());
    let result = panic_free_decode_frame(&truncated, payload, MaxPayloadBytes::DEFAULT);
    assert!(
        matches!(result, Err(_)),
        "truncated header must yield error, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Payload codec panic-freedom
// ---------------------------------------------------------------------------

fn panic_free_encode_payload(
    payload: &IpcPayload,
    max_payload: MaxPayloadBytes,
) -> Result<BoundedPayload, IpcError> {
    catch_unwind(|| encode_payload(payload, max_payload)).expect("encode_payload must not panic")
}

fn panic_free_decode_payload(bounded: &BoundedPayload) -> Result<IpcPayload, IpcError> {
    catch_unwind(|| decode_payload(bounded)).expect("decode_payload must not panic")
}

#[test]
fn encode_health_payload_is_panic_free() {
    let payload = IpcPayload::Health;
    let result = panic_free_encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert!(
        result.is_ok(),
        "Health payload encode must succeed, got {:?}",
        result
    );
}

#[test]
fn encode_submit_run_payload_is_panic_free() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([1; 32]),
        input: vec![1, 2, 3, 4],
    });
    let result = panic_free_encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert!(
        result.is_ok(),
        "SubmitRun payload encode must succeed, got {:?}",
        result
    );
}

#[test]
fn encode_decode_roundtrip_health() {
    let original = IpcPayload::Health;
    let encoded = panic_free_encode_payload(&original, MaxPayloadBytes::DEFAULT).expect("encode");
    let decoded = panic_free_decode_payload(&encoded).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn encode_decode_roundtrip_submit_run() {
    let original = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([5; 32]),
        input: vec![10, 20, 30],
    });
    let encoded = panic_free_encode_payload(&original, MaxPayloadBytes::DEFAULT).expect("encode");
    let decoded = panic_free_decode_payload(&encoded).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn encode_decode_roundtrip_cancel_run() {
    let original = IpcPayload::CancelRun {
        run_id: RunId::new(99),
    };
    let encoded = panic_free_encode_payload(&original, MaxPayloadBytes::DEFAULT).expect("encode");
    let decoded = panic_free_decode_payload(&encoded).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn decode_corrupted_postcard_bytes_returns_error_not_ub() {
    let corrupted =
        BoundedPayload::new(bytes::Bytes::from(vec![0xFF; 20]), MaxPayloadBytes::DEFAULT)
            .expect("bounded payload creation must not panic");
    let result = panic_free_decode_payload(&corrupted);
    assert!(
        !result.is_ok(),
        "corrupted postcard must fail, got {:?}",
        result
    );
}

#[test]
fn decode_empty_payload_bytes_returns_error_not_ub() {
    let empty =
        BoundedPayload::new(bytes::Bytes::from(Vec::new()), MaxPayloadBytes::DEFAULT).unwrap();
    let result = panic_free_decode_payload(&empty);
    assert!(
        matches!(result, Err(IpcError::PayloadDecodeFailed)),
        "empty bytes must yield PayloadDecodeFailed, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Ingress frame construction
// ---------------------------------------------------------------------------

fn panic_free_ingress_frame(
    run_id: RunId,
    workflow: WorkflowDigest,
    payload: bytes::Bytes,
    max_payload: MaxPayloadBytes,
) -> Result<crate::IngressFrame, IpcError> {
    catch_unwind(|| crate::IngressFrame::new(run_id, workflow, payload, max_payload))
        .expect("IngressFrame::new must not panic")
}

#[test]
fn ingress_frame_empty_payload_succeeds() {
    let result = panic_free_ingress_frame(
        RunId::new(1),
        WorkflowDigest::from_bytes([1; 32]),
        bytes::Bytes::from(Vec::new()),
        MaxPayloadBytes::DEFAULT,
    );
    assert!(
        result.is_ok(),
        "empty payload ingress frame must succeed, got {:?}",
        result
    );
}

#[test]
fn ingress_frame_with_payload_succeeds() {
    let result = panic_free_ingress_frame(
        RunId::new(2),
        WorkflowDigest::from_bytes([2; 32]),
        bytes::Bytes::from(vec![1, 2, 3, 4, 5]),
        MaxPayloadBytes::DEFAULT,
    );
    assert!(
        result.is_ok(),
        "payload ingress frame must succeed, got {:?}",
        result
    );
}

#[test]
fn ingress_frame_with_empty_workflow_digest_succeeds() {
    let result = panic_free_ingress_frame(
        RunId::new(3),
        WorkflowDigest::from_bytes([0; 32]),
        bytes::Bytes::from(Vec::new()),
        MaxPayloadBytes::DEFAULT,
    );
    assert!(
        result.is_ok(),
        "empty workflow digest must succeed, got {:?}",
        result
    );
}

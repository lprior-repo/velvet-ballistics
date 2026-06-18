//! RPO-IPC-003: production-bound IPC frame encode/read/decode properties.
//!
//! This proptest lane intentionally calls only public `vb_ipc` production APIs:
//! `IpcFrameHeader`, `IpcFrame`, `encode_frame`, bounded header/payload reads,
//! `validate_frame_bounds`, `decode_frame`, and `decode_frame_payload`.

use bytes::Bytes;
use proptest::prelude::*;
use std::io::Cursor;
use std::num::NonZeroUsize;
use vb_ipc::{
    decode_frame, decode_frame_payload, encode_frame, encode_payload, read_frame_header_bounded,
    read_frame_payload_bounded, validate_frame_bounds, IpcCommand, IpcError, IpcFrame,
    IpcFrameHeader, IpcPayload, MaxPayloadBytes, IPC_HEADER_LEN, IPC_MAGIC,
};

const RPO_IPC_003: &str = "RPO-IPC-003";
const TEST_MAX_PAYLOAD_BYTES: usize = 256;
const TEST_MAX_PAYLOAD_NONZERO: NonZeroUsize = match NonZeroUsize::new(TEST_MAX_PAYLOAD_BYTES) {
    Some(value) => value,
    None => NonZeroUsize::MIN,
};
const TEST_MAX_PAYLOAD: MaxPayloadBytes = MaxPayloadBytes::new(TEST_MAX_PAYLOAD_NONZERO);
const TRAILER: [u8; 2] = [0xA5, 0x5A];

fn any_wire_command() -> impl Strategy<Value = IpcCommand> {
    any::<u16>().prop_map(|wire_id| match IpcCommand::from_u16(wire_id) {
        Ok(command) => command,
        Err(_) => IpcCommand::UnknownCommand(wire_id),
    })
}

fn bounded_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=TEST_MAX_PAYLOAD_BYTES)
}

fn test_max_payload(limit: usize) -> Result<MaxPayloadBytes, TestCaseError> {
    let Some(nonzero) = NonZeroUsize::new(limit) else {
        return Err(TestCaseError::fail(format!(
            "{RPO_IPC_003}: generated zero max-payload limit"
        )));
    };
    Ok(MaxPayloadBytes::new(nonzero))
}

fn case_ipc<T>(result: Result<T, IpcError>, context: &str) -> Result<T, TestCaseError> {
    result
        .map_err(|error| TestCaseError::fail(format!("{RPO_IPC_003}: {context} failed: {error:?}")))
}

fn string_ipc<T>(result: Result<T, IpcError>, context: &str) -> Result<T, String> {
    result.map_err(|error| format!("{RPO_IPC_003}: {context} failed: {error:?}"))
}

fn checked_add_case(left: usize, right: usize, context: &str) -> Result<usize, TestCaseError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(TestCaseError::fail(format!(
            "{RPO_IPC_003}: {context} overflowed for {left} + {right}"
        ))),
    }
}

fn checked_add_string(left: usize, right: usize, context: &str) -> Result<usize, String> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(format!(
            "{RPO_IPC_003}: {context} overflowed for {left} + {right}"
        )),
    }
}

fn usize_to_u32_case(value: usize, context: &str) -> Result<u32, TestCaseError> {
    u32::try_from(value).map_err(|error| {
        TestCaseError::fail(format!(
            "{RPO_IPC_003}: {context} did not fit u32: {error:?}"
        ))
    })
}

fn usize_to_u32_string(value: usize, context: &str) -> Result<u32, String> {
    u32::try_from(value)
        .map_err(|error| format!("{RPO_IPC_003}: {context} did not fit u32: {error:?}"))
}

fn cursor_position_usize(cursor_position: u64, context: &str) -> Result<usize, TestCaseError> {
    usize::try_from(cursor_position).map_err(|error| {
        TestCaseError::fail(format!(
            "{RPO_IPC_003}: {context} position did not fit usize: {error:?}"
        ))
    })
}

fn header_from_frame(frame: &[u8]) -> Result<[u8; IPC_HEADER_LEN], TestCaseError> {
    let Some(header_slice) = frame.get(..IPC_HEADER_LEN) else {
        return Err(TestCaseError::fail(format!(
            "{RPO_IPC_003}: encoded frame shorter than IPC_HEADER_LEN"
        )));
    };
    <[u8; IPC_HEADER_LEN]>::try_from(header_slice).map_err(|error| {
        TestCaseError::fail(format!(
            "{RPO_IPC_003}: header slice did not fit fixed array: {error:?}"
        ))
    })
}

fn header_from_slice_string(bytes: &[u8], context: &str) -> Result<[u8; IPC_HEADER_LEN], String> {
    <[u8; IPC_HEADER_LEN]>::try_from(bytes)
        .map_err(|error| format!("{RPO_IPC_003}: {context}: {error:?}"))
}

fn assert_payload_too_large<T: core::fmt::Debug>(
    result: Result<T, IpcError>,
    expected_actual: usize,
    expected_limit: usize,
    context: &str,
) -> Result<(), String> {
    match result {
        Err(IpcError::PayloadTooLarge { actual, limit })
            if actual == expected_actual && limit == expected_limit =>
        {
            Ok(())
        }
        other => Err(format!(
            "{RPO_IPC_003}: {context}: expected PayloadTooLarge actual={expected_actual} limit={expected_limit}, got {other:?}"
        )),
    }
}

fn assert_invalid_magic<T: core::fmt::Debug>(
    result: Result<T, IpcError>,
    expected_actual: u32,
    context: &str,
) -> Result<(), String> {
    match result {
        Err(IpcError::InvalidMagic { actual }) if actual == expected_actual => Ok(()),
        other => Err(format!(
            "{RPO_IPC_003}: {context}: expected InvalidMagic actual={expected_actual:#010x}, got {other:?}"
        )),
    }
}

fn assert_payload_length_mismatch<T: core::fmt::Debug>(
    result: Result<T, IpcError>,
    expected_header: usize,
    expected_actual: usize,
    context: &str,
) -> Result<(), String> {
    match result {
        Err(IpcError::PayloadLengthMismatch { header, actual })
            if header == expected_header && actual == expected_actual =>
        {
            Ok(())
        }
        other => Err(format!(
            "{RPO_IPC_003}: {context}: expected PayloadLengthMismatch header={expected_header} actual={expected_actual}, got {other:?}"
        )),
    }
}

fn assert_equal<T: core::fmt::Debug + PartialEq>(
    actual: T,
    expected: T,
    context: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{RPO_IPC_003}: {context}: expected {expected:?}, got {actual:?}"
        ))
    }
}

fn boundary_payload(length: usize) -> Vec<u8> {
    vec![0xA5; length]
}

fn check_exact_payload_length_boundary(length: usize) -> Result<(), String> {
    let payload = boundary_payload(length);
    let frame = string_ipc(
        encode_frame(IpcCommand::Health, 0x55AA, 0x1234, &payload),
        "encode_frame boundary",
    )?;
    let expected_len = checked_add_string(IPC_HEADER_LEN, payload.len(), "frame boundary len")?;
    assert_equal(frame.len(), expected_len, "encoded boundary frame length")?;

    let header_slice = frame
        .get(..IPC_HEADER_LEN)
        .ok_or_else(|| format!("{RPO_IPC_003}: boundary frame missing header"))?;
    let header_bytes = header_from_slice_string(header_slice, "boundary header array")?;
    let header = string_ipc(
        IpcFrameHeader::decode(&header_bytes, TEST_MAX_PAYLOAD),
        "IpcFrameHeader::decode boundary",
    )?;
    assert_equal(header.command, IpcCommand::Health, "boundary command")?;
    assert_equal(header.flags, 0x55AA, "boundary flags")?;
    assert_equal(header.correlation, 0x1234, "boundary correlation")?;
    assert_equal(
        usize::try_from(header.payload_len)
            .map_err(|error| format!("{RPO_IPC_003}: payload_len fit: {error:?}"))?,
        payload.len(),
        "boundary header payload_len",
    )?;
    assert_equal(
        frame.get(IPC_HEADER_LEN..),
        Some(payload.as_slice()),
        "boundary payload bytes",
    )?;

    let decoded_frame = string_ipc(
        decode_frame(
            &header_bytes,
            Bytes::from(payload.clone()),
            TEST_MAX_PAYLOAD,
        ),
        "decode_frame boundary",
    )?;
    assert_equal(decoded_frame.header(), header, "boundary IpcFrame header")?;
    assert_equal(
        decoded_frame.payload().bytes().as_ref(),
        payload.as_slice(),
        "boundary IpcFrame payload bytes",
    )
}

proptest! {
    #[test]
    fn rpo_ipc_003_generated_encode_read_decode_preserves_lengths_and_bytes(
        command in any_wire_command(),
        flags in any::<u16>(),
        correlation in any::<u64>(),
        payload in bounded_payload(),
    ) {
        let frame = case_ipc(encode_frame(command, flags, correlation, &payload), "encode_frame")?;
        let expected_frame_len = checked_add_case(IPC_HEADER_LEN, payload.len(), "encoded frame length")?;
        prop_assert_eq!(frame.len(), expected_frame_len);
        prop_assert_eq!(frame.get(IPC_HEADER_LEN..), Some(payload.as_slice()));

        let header_bytes = header_from_frame(&frame)?;
        let decoded_header = case_ipc(
            IpcFrameHeader::decode(&header_bytes, TEST_MAX_PAYLOAD),
            "IpcFrameHeader::decode",
        )?;
        prop_assert_eq!(decoded_header.command.as_u16(), command.as_u16());
        prop_assert_eq!(decoded_header.flags, flags);
        prop_assert_eq!(decoded_header.correlation, correlation);
        let decoded_payload_len = cursor_position_usize(u64::from(decoded_header.payload_len), "payload_len")?;
        prop_assert_eq!(decoded_payload_len, payload.len());
        case_ipc(validate_frame_bounds(&decoded_header, TEST_MAX_PAYLOAD), "validate_frame_bounds")?;

        let decoded_frame = case_ipc(
            decode_frame(&header_bytes, Bytes::from(payload.clone()), TEST_MAX_PAYLOAD),
            "decode_frame",
        )?;
        prop_assert_eq!(decoded_frame.header(), decoded_header);
        prop_assert_eq!(decoded_frame.payload().bytes().as_ref(), payload.as_slice());

        let mut frame_with_trailer = frame.clone();
        frame_with_trailer.extend_from_slice(&TRAILER);
        let mut reader = Cursor::new(frame_with_trailer.as_slice());
        let read_header = case_ipc(read_frame_header_bounded(&mut reader, TEST_MAX_PAYLOAD), "read_frame_header_bounded")?;
        prop_assert_eq!(read_header, decoded_header);
        let read_payload = case_ipc(
            read_frame_payload_bounded(&mut reader, &read_header, TEST_MAX_PAYLOAD),
            "read_frame_payload_bounded",
        )?;
        prop_assert_eq!(read_payload.as_slice(), payload.as_slice());
        let reader_position = cursor_position_usize(reader.position(), "reader")?;
        prop_assert_eq!(reader_position, expected_frame_len);
        prop_assert_eq!(frame_with_trailer.get(reader_position..), Some(TRAILER.as_slice()));

        let rebuilt = case_ipc(
            IpcFrame::new(read_header, Bytes::from(read_payload), TEST_MAX_PAYLOAD),
            "IpcFrame::new",
        )?;
        prop_assert_eq!(rebuilt.header(), decoded_header);
        prop_assert_eq!(rebuilt.payload().bytes().as_ref(), payload.as_slice());
    }

    #[test]
    fn rpo_ipc_003_generated_oversize_declarations_reject_concrete_error(
        command in any_wire_command(),
        flags in any::<u16>(),
        correlation in any::<u64>(),
        limit in 1usize..=TEST_MAX_PAYLOAD_BYTES,
        extra in 1usize..=TEST_MAX_PAYLOAD_BYTES,
        payload_prefix in prop::collection::vec(any::<u8>(), 0..=4),
    ) {
        let max_payload = test_max_payload(limit)?;
        let declared_len = checked_add_case(limit, extra, "oversize declared length")?;
        let declared_len_u32 = usize_to_u32_case(declared_len, "oversize declared length")?;
        let header = IpcFrameHeader::new(command, flags, correlation, declared_len_u32);
        let header_bytes = case_ipc(header.encode(), "oversize header encode")?;

        let decoded_result = IpcFrameHeader::decode(&header_bytes, max_payload);
        prop_assert_eq!(
            decoded_result,
            Err(IpcError::PayloadTooLarge {
                actual: declared_len,
                limit,
            })
        );

        let bounds_result = validate_frame_bounds(&header, max_payload);
        prop_assert_eq!(
            bounds_result,
            Err(IpcError::PayloadTooLarge {
                actual: declared_len,
                limit,
            })
        );

        let mut header_reader = Cursor::new(header_bytes.to_vec());
        let read_header_result = read_frame_header_bounded(&mut header_reader, max_payload);
        prop_assert_eq!(
            read_header_result,
            Err(IpcError::PayloadTooLarge {
                actual: declared_len,
                limit,
            })
        );

        let mut payload_reader = Cursor::new(payload_prefix.as_slice());
        let payload_result = read_frame_payload_bounded(&mut payload_reader, &header, max_payload);
        prop_assert_eq!(
            payload_result,
            Err(IpcError::PayloadTooLarge {
                actual: declared_len,
                limit,
            })
        );
        prop_assert_eq!(payload_reader.position(), 0);
    }
}

#[test]
fn rpo_ipc_003_explicit_length_boundaries_zero_one_and_max_allowed() -> Result<(), String> {
    for length in [0, 1, TEST_MAX_PAYLOAD_BYTES] {
        check_exact_payload_length_boundary(length)?;
    }
    Ok(())
}

#[test]
fn rpo_ipc_003_explicit_oversize_rejected_before_payload_read() -> Result<(), String> {
    let declared_len = checked_add_string(
        TEST_MAX_PAYLOAD_BYTES,
        1,
        "explicit oversize declared length",
    )?;
    let declared_len_u32 = usize_to_u32_string(declared_len, "explicit oversize declared length")?;
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 99, declared_len_u32);
    let header_bytes = string_ipc(header.encode(), "explicit oversize header encode")?;

    assert_payload_too_large(
        IpcFrameHeader::decode(&header_bytes, TEST_MAX_PAYLOAD),
        declared_len,
        TEST_MAX_PAYLOAD_BYTES,
        "IpcFrameHeader::decode explicit oversize",
    )?;
    assert_payload_too_large(
        validate_frame_bounds(&header, TEST_MAX_PAYLOAD),
        declared_len,
        TEST_MAX_PAYLOAD_BYTES,
        "validate_frame_bounds explicit oversize",
    )?;

    let mut header_reader = Cursor::new(header_bytes.to_vec());
    assert_payload_too_large(
        read_frame_header_bounded(&mut header_reader, TEST_MAX_PAYLOAD),
        declared_len,
        TEST_MAX_PAYLOAD_BYTES,
        "read_frame_header_bounded explicit oversize",
    )?;

    let mut payload_reader = Cursor::new(TRAILER.as_slice());
    assert_payload_too_large(
        read_frame_payload_bounded(&mut payload_reader, &header, TEST_MAX_PAYLOAD),
        declared_len,
        TEST_MAX_PAYLOAD_BYTES,
        "read_frame_payload_bounded explicit oversize",
    )?;
    assert_equal(
        payload_reader.position(),
        0,
        "oversize payload reader position before read",
    )
}

#[test]
fn rpo_ipc_003_explicit_bad_magic_rejected_by_bounded_header_read() -> Result<(), String> {
    let good_header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
    let good_bytes = string_ipc(good_header.encode(), "bad-magic seed header encode")?;
    let bad_magic = IPC_MAGIC ^ 1;
    let suffix = good_bytes
        .get(4..)
        .ok_or_else(|| format!("{RPO_IPC_003}: good header missing suffix"))?;
    let mut bad_bytes_vec = Vec::with_capacity(IPC_HEADER_LEN);
    bad_bytes_vec.extend_from_slice(&bad_magic.to_le_bytes());
    bad_bytes_vec.extend_from_slice(suffix);
    let bad_bytes = header_from_slice_string(&bad_bytes_vec, "bad-magic header array")?;

    assert_invalid_magic(
        IpcFrameHeader::decode(&bad_bytes, TEST_MAX_PAYLOAD),
        bad_magic,
        "IpcFrameHeader::decode bad magic",
    )?;
    let mut reader = Cursor::new(bad_bytes_vec.as_slice());
    assert_invalid_magic(
        read_frame_header_bounded(&mut reader, TEST_MAX_PAYLOAD),
        bad_magic,
        "read_frame_header_bounded bad magic",
    )
}

#[test]
fn rpo_ipc_003_decode_frame_payload_checks_length_before_postcard() -> Result<(), String> {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 3);
    assert_payload_length_mismatch(
        decode_frame_payload(&header, TRAILER.as_slice()),
        3,
        TRAILER.len(),
        "decode_frame_payload mismatch",
    )
}

#[test]
fn rpo_ipc_003_decode_frame_payload_accepts_valid_health_postcard() -> Result<(), String> {
    let encoded = string_ipc(
        encode_payload(&IpcPayload::Health, TEST_MAX_PAYLOAD),
        "encode_payload Health",
    )?;
    let header = IpcFrameHeader::new(
        IpcCommand::Health,
        0,
        0,
        usize_to_u32_string(encoded.bytes().len(), "Health postcard payload len")?,
    );
    let decoded = string_ipc(
        decode_frame_payload(&header, encoded.bytes()),
        "decode_frame_payload Health",
    )?;
    assert_equal(decoded, IpcPayload::Health, "decoded Health payload")
}

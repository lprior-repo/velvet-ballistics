//! **PO-vb-hbav-023**: Proptest IpcError exhaustiveness.
//!
//! For arbitrary header+payload bytes, `decode_frame_header` and
//! `decode_frame_payload` must return `IpcError` variants matching the
//! currently-defined production variants.

use proptest::prelude::*;
use vb_ipc::{CommandFlags, IPC_HEADER_LEN, IpcCommand, IpcError, MaxPayloadBytes};

proptest! {
    /// Verify that every error result from IPC frame decode matches a known
    /// IpcError variant. Unknown variants must cause the test to fail.
    #[test]
    fn proptest_ipc_frame_errors_are_typed(
        header in prop::collection::vec(any::<u8>(), IPC_HEADER_LEN..=IPC_HEADER_LEN),
        payload in prop::collection::vec(any::<u8>(), 0..65536),
    ) {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes.copy_from_slice(&header);

        let max_payload = MaxPayloadBytes::DEFAULT;

        // Test header decode
        match vb_ipc::IpcFrameHeader::decode(&header_bytes, max_payload) {
            Ok(header) => match vb_ipc::frame::decode_frame_payload(&header, &payload) {
                Ok(_payload) => {}
                Err(error) => assert_known_ipc_error(error)?,
            },
            Err(error) => assert_known_ipc_error(error)?,
        }

        // Test simple header decode
        match vb_ipc::frame::decode_frame_header(&header_bytes) {
            Ok(_header) => {}
            Err(error) => assert_known_ipc_error(error)?,
        }
    }

    /// Test validate_frame_magic with arbitrary bytes.
    #[test]
    fn proptest_validate_frame_magic_typed(data in prop::collection::vec(any::<u8>(), 0..64)) {
        match vb_ipc::frame::validate_frame_magic(&data) {
            Ok(_) => {}
            Err(error) => assert_known_ipc_error(error)?,
        }
    }

    /// Actual encoded Health headers with low-byte non-zero flags trigger
    /// InvalidCommandFlags (valid mask is 0x0000 for Health).
    #[test]
    fn proptest_invalid_command_flags_qualifies(
        correlation in any::<u64>(),
        flags in 1u16..=0x00FF,
    ) {
        let header = vb_ipc::IpcFrameHeader::new(IpcCommand::Health, flags, correlation, 0);
        let encoded = header
            .encode()
            .map_err(|e| TestCaseError::fail(format!("encode failed: {e:?}")))?;

        let expected = IpcError::InvalidCommandFlags {
            command: IpcCommand::Health,
            flags,
        };

        assert_decode_error(
            vb_ipc::IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT),
            &expected,
            "non-zero low-byte Health flags",
        )?;
        assert_decode_error(
            vb_ipc::frame::decode_frame_header(&encoded),
            &expected,
            "non-zero low-byte Health flags via frame::decode_frame_header",
        )?;
    }

    /// Actual encoded Health headers with high-byte reserved flags trigger
    /// ReservedBitsSet (reserved global mask 0xFF00 takes precedence).
    #[test]
    fn proptest_reserved_bits_set_qualifies(
        correlation in any::<u64>(),
        flags in 0x0100u16..=u16::MAX,
    ) {
        let header = vb_ipc::IpcFrameHeader::new(IpcCommand::Health, flags, correlation, 0);
        let encoded = header
            .encode()
            .map_err(|e| TestCaseError::fail(format!("encode failed: {e:?}")))?;

        let expected = IpcError::ReservedBitsSet {
            command: IpcCommand::Health,
            actual: flags,
            reserved_mask: CommandFlags::reserved_global_mask(),
        };

        assert_decode_error(
            vb_ipc::IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT),
            &expected,
            "high-byte reserved Health flags",
        )?;
        assert_decode_error(
            vb_ipc::frame::decode_frame_header(&encoded),
            &expected,
            "high-byte reserved Health flags via frame::decode_frame_header",
        )?;
    }
}

/// Shared helper: assert exact decode error variant and qualify through oracle.
fn assert_decode_error(
    result: Result<vb_ipc::IpcFrameHeader, IpcError>,
    expected: &IpcError,
    description: &str,
) -> Result<(), TestCaseError> {
    match result {
        Ok(_) => Err(TestCaseError::fail(format!(
            "Expected error for {description}, got Ok"
        ))),
        Err(error) => {
            prop_assert_eq!(&error, expected, "Incorrect error for {}", description);
            assert_known_ipc_error(error)
        }
    }
}

/// Asserts that an IPC error is a known typed variant.
/// Fails if an unknown variant is encountered.
fn assert_known_ipc_error(error: IpcError) -> Result<(), TestCaseError> {
    match error {
        IpcError::Full
        | IpcError::Disconnected
        | IpcError::PayloadTooLarge { .. }
        | IpcError::InvalidMagic { .. }
        | IpcError::UnsupportedVersion { .. }
        | IpcError::UnknownCommand(_)
        | IpcError::InvalidCommandFlags { .. }
        | IpcError::ReservedBitsSet { .. }
        | IpcError::ReservedNonZero { .. }
        | IpcError::PayloadLengthMismatch { .. }
        | IpcError::HeaderEncodeFailed
        | IpcError::HeaderDecodeFailed
        | IpcError::PayloadLengthOutOfRange { .. }
        | IpcError::PayloadEncodeFailed
        | IpcError::PayloadDecodeFailed
        | IpcError::ResponseDecodeFailed => Ok(()),
        _ => Err(TestCaseError::fail(format!(
            "Unknown IpcError variant: {:?}. Update assert_known_ipc_error.",
            error
        ))),
    }
}

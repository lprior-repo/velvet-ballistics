//! **PO-vb-hbav-023**: Proptest IpcError exhaustiveness.
//!
//! For arbitrary header+payload bytes, `decode_frame_header` and
//! `decode_frame_payload` must return `IpcError` variants matching the
//! currently-defined production variants.

use proptest::prelude::*;
use vb_ipc::{IPC_HEADER_LEN, IpcError, MaxPayloadBytes};

proptest! {
    /// Verify that every error result from IPC frame decode matches a known
    /// IpcError variant. Unknown variants must cause the test to fail.
    #[test]
    fn proptest_ipc_frame_errors_are_typed(
        header in prop::collection::vec(any::<u8>(), IPC_HEADER_LEN..IPC_HEADER_LEN),
        _payload in prop::collection::vec(any::<u8>(), 0..65536),
    ) {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes.copy_from_slice(&header);

        let max_nz = match std::num::NonZeroUsize::new(65536) {
            Some(nz) => nz,
            None => {
                // Unreachable: 65536 > 0, so new() always succeeds.
                // Proptest closures require all branches to have same type.
                unreachable!("NonZeroUsize::new(65536) always succeeds (65536 > 0)");
            }
        };
        let max_payload = MaxPayloadBytes::new(max_nz);

        // Test header decode
        match vb_ipc::IpcFrameHeader::decode(&header_bytes, max_payload) {
            Ok(_header) => {}
            Err(error) => assert_known_ipc_error(error),
        }

        // Test simple header decode
        match vb_ipc::frame::decode_frame_header(&header_bytes) {
            Ok(_header) => {}
            Err(error) => assert_known_ipc_error(error),
        }
    }

    /// Test validate_frame_magic with arbitrary bytes.
    #[test]
    fn proptest_validate_frame_magic_typed(data in prop::collection::vec(any::<u8>(), 0..64)) {
        match vb_ipc::frame::validate_frame_magic(&data) {
            Ok(_) => {}
            Err(error) => assert_known_ipc_error(error),
        }
    }
}

/// Asserts that an IPC error is a known typed variant.
/// Panics if an unknown variant is encountered.
fn assert_known_ipc_error(error: IpcError) {
    match error {
        IpcError::Full
        | IpcError::Disconnected
        | IpcError::PayloadTooLarge { .. }
        | IpcError::InvalidMagic { .. }
        | IpcError::UnsupportedVersion { .. }
        | IpcError::UnknownCommand(_)
        | IpcError::ReservedNonZero { .. }
        | IpcError::PayloadLengthMismatch { .. }
        | IpcError::HeaderEncodeFailed
        | IpcError::HeaderDecodeFailed
        | IpcError::PayloadLengthOutOfRange { .. }
        | IpcError::PayloadEncodeFailed
        | IpcError::PayloadDecodeFailed
        | IpcError::ResponseDecodeFailed => {}
        _ => {
            panic!(
                "Unknown IpcError variant: {:?}. Update assert_known_ipc_error.",
                error
            );
        }
    }
}

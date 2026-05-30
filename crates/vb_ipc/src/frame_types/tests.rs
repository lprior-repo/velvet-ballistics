//! Tests for frame_types.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IpcCommand;
    use crate::constants::IPC_HEADER_LEN;
    use bytes::Bytes;

    fn make_valid_header_bytes() -> [u8; IPC_HEADER_LEN] {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 0);
        header.encode().expect("encode should succeed")
    }

    #[test]
    fn decode_rejects_invalid_magic() {
        let mut bytes = make_valid_header_bytes();
        bytes[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::InvalidMagic {
                actual: 0xDEADBEEF_u32,
            })
        );
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = make_valid_header_bytes();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 99 }));
    }

    #[test]
    fn decode_rejects_nonzero_reserved_field() {
        let mut bytes = make_valid_header_bytes();
        bytes[10..12].copy_from_slice(&7u16.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::ReservedNonZero { actual: 7 }));
    }

    #[test]
    fn decode_rejects_payload_too_large() {
        let mut bytes = make_valid_header_bytes();
        let limit = MaxPayloadBytes::DEFAULT.get() as u32;
        let oversized = limit.saturating_add(1);
        bytes[20..24].copy_from_slice(&oversized.to_le_bytes());

        let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: oversized as usize,
                limit: MaxPayloadBytes::DEFAULT.get(),
            })
        );
    }

    #[test]
    fn new_rejects_payload_length_mismatch() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
        let payload = Bytes::from(vec![0u8; 5]);

        let result = IpcFrame::new(header, payload, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PayloadLengthMismatch {
                header: 10,
                actual: 5,
            })
        );
    }

    #[test]
    fn header_getter_returns_expected_value() {
        let expected = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let frame = IpcFrame::new(expected, Bytes::new(), MaxPayloadBytes::DEFAULT)
            .expect("frame should build");

        assert_eq!(frame.header(), expected);
    }

    #[test]
    fn payload_getter_returns_expected_value() {
        let payload_data = vec![0xAB, 0xCD, 0xEF];
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_data.len() as u32);
        let frame = IpcFrame::new(
            header,
            Bytes::from(payload_data.clone()),
            MaxPayloadBytes::DEFAULT,
        )
        .expect("frame should build");

        assert_eq!(frame.payload().bytes().as_ref(), payload_data.as_slice());
    }

    #[test]
    fn decode_frame_propagates_header_errors() {
        let mut bytes = make_valid_header_bytes();
        bytes[0..4].copy_from_slice(&0u32.to_le_bytes());

        let result = decode_frame(&bytes, Bytes::new(), MaxPayloadBytes::DEFAULT);
        assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
    }

    #[test]
    fn decode_frame_succeeds_with_valid_header_and_payload() {
        let payload_data = vec![0x01, 0x02, 0x03];
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 7, payload_data.len() as u32);
        let header_bytes = header.encode().expect("encode should succeed");

        let result = decode_frame(
            &header_bytes,
            Bytes::from(payload_data.clone()),
            MaxPayloadBytes::DEFAULT,
        );
        let frame = result.expect("decode_frame should succeed");
        assert_eq!(frame.header(), header);
        assert_eq!(frame.payload().bytes().as_ref(), payload_data.as_slice());
    }
}
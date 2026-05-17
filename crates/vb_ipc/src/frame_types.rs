#![forbid(unsafe_code)]
//! IPC frame types.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::Bytes;
use std::io::Cursor;

use crate::bounded::{BoundedPayload, MaxPayloadBytes};
use crate::commands::IpcCommand;
use crate::constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
use crate::error::IpcError;

/// Fixed binary IPC frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcFrameHeader {
    /// IPC command kind.
    pub command: IpcCommand,
    /// Command-specific flags.
    pub flags: u16,
    /// Correlates requests and replies.
    pub correlation: u64,
    /// Postcard payload byte length.
    pub payload_len: u32,
}

impl IpcFrameHeader {
    /// Creates an IPC frame header.
    #[must_use]
    pub const fn new(command: IpcCommand, flags: u16, correlation: u64, payload_len: u32) -> Self {
        Self {
            command,
            flags,
            correlation,
            payload_len,
        }
    }

    /// Encodes the header using the §21 little-endian wire layout.
    pub fn encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
        let mut bytes = [0u8; IPC_HEADER_LEN];
        let mut cursor = std::io::Cursor::new(&mut bytes[..]);
        cursor
            .write_u32::<LittleEndian>(IPC_MAGIC)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(IPC_VERSION)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(self.command.as_u16())
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(self.flags)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(0_u16)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u64::<LittleEndian>(self.correlation)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u32::<LittleEndian>(self.payload_len)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        Ok(bytes)
    }

    /// Decodes and validates a fixed IPC header before payload allocation.
    pub fn decode(
        bytes: &[u8; IPC_HEADER_LEN],
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        let mut cursor = Cursor::new(bytes.as_slice());
        let magic = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if magic != IPC_MAGIC {
            return Err(IpcError::InvalidMagic { actual: magic });
        }

        let version = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if version != IPC_VERSION {
            return Err(IpcError::UnsupportedVersion { actual: version });
        }

        let command = IpcCommand::from_u16(
            cursor
                .read_u16::<LittleEndian>()
                .map_err(|_| IpcError::HeaderDecodeFailed)?,
        )?;
        let flags = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let reserved = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if reserved != 0 {
            return Err(IpcError::ReservedNonZero { actual: reserved });
        }
        let correlation = cursor
            .read_u64::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let payload_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let payload_len_usize = crate::error::u32_to_usize(payload_len)?;
        if payload_len_usize > max_payload.get() {
            return Err(IpcError::PayloadTooLarge {
                actual: payload_len_usize,
                limit: max_payload.get(),
            });
        }

        Ok(Self {
            command,
            flags,
            correlation,
            payload_len,
        })
    }
}

/// Decoded IPC frame with bounded postcard payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcFrame {
    header: IpcFrameHeader,
    payload: BoundedPayload,
}

impl IpcFrame {
    /// Builds a frame after enforcing header/payload length agreement.
    pub fn new(
        header: IpcFrameHeader,
        payload: Bytes,
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        let actual_len = payload.len();
        let expected_len = crate::error::u32_to_usize(header.payload_len)?;
        if actual_len != expected_len {
            return Err(IpcError::PayloadLengthMismatch {
                header: expected_len,
                actual: actual_len,
            });
        }

        Ok(Self {
            header,
            payload: BoundedPayload::new(payload, max_payload)?,
        })
    }

    /// Returns the decoded frame header.
    #[must_use]
    pub const fn header(&self) -> IpcFrameHeader {
        self.header
    }

    /// Returns bounded postcard payload bytes.
    #[must_use]
    pub const fn payload(&self) -> &BoundedPayload {
        &self.payload
    }
}

/// Decodes a fixed header and already-read payload bytes into a bounded frame.
pub fn decode_frame(
    header: &[u8; IPC_HEADER_LEN],
    payload: Bytes,
    max_payload: MaxPayloadBytes,
) -> Result<IpcFrame, IpcError> {
    IpcFrame::new(
        IpcFrameHeader::decode(header, max_payload)?,
        payload,
        max_payload,
    )
}

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

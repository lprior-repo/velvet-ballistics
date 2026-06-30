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
#[path = "frame_types/tests.rs"]
mod tests;

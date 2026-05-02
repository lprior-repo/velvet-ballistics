//! IPC frame types, constants, and command identifiers.

/// IPC frame magic: `VBLT` little-endian.
pub const IPC_MAGIC: u32 = 0x5642_4C54;
/// Supported IPC schema version.
pub const IPC_VERSION: u16 = 1;
/// Fixed IPC header length in bytes.
pub const IPC_HEADER_LEN: usize = 24;

/// Binary IPC command identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum IpcCommand {
    /// Submit a run using a previously compiled workflow artifact.
    SubmitRun = 1,
    /// Submit a run with inline validated runtime inputs.
    SubmitRunInline = 2,
    /// Cancel an active or queued run.
    CancelRun = 3,
    /// Inspect run state.
    InspectRun = 4,
    /// List persisted events for a run.
    ListEvents = 5,
    /// Answer a suspended ask.
    AnswerAsk = 6,
    /// Complete an external action ticket.
    CompleteAction = 7,
    /// Fail an external action ticket.
    FailAction = 8,
    /// Drain bounded trace records.
    DrainTrace = 9,
    /// Probe runtime health.
    Health = 10,
    /// Request graceful shutdown.
    Shutdown = 11,
}

impl IpcCommand {
    /// Parses a wire command identifier.
    pub fn from_u16(value: u16) -> Result<Self, super::IpcError> {
        match value {
            1 => Ok(Self::SubmitRun),
            2 => Ok(Self::SubmitRunInline),
            3 => Ok(Self::CancelRun),
            4 => Ok(Self::InspectRun),
            5 => Ok(Self::ListEvents),
            6 => Ok(Self::AnswerAsk),
            7 => Ok(Self::CompleteAction),
            8 => Ok(Self::FailAction),
            9 => Ok(Self::DrainTrace),
            10 => Ok(Self::Health),
            11 => Ok(Self::Shutdown),
            other => Err(super::IpcError::UnknownCommand(other)),
        }
    }

    /// Returns the wire command identifier.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::SubmitRun => 1,
            Self::SubmitRunInline => 2,
            Self::CancelRun => 3,
            Self::InspectRun => 4,
            Self::ListEvents => 5,
            Self::AnswerAsk => 6,
            Self::CompleteAction => 7,
            Self::FailAction => 8,
            Self::DrainTrace => 9,
            Self::Health => 10,
            Self::Shutdown => 11,
        }
    }
}

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
    pub const fn new(
        command: IpcCommand,
        flags: u16,
        correlation: u64,
        payload_len: u32,
    ) -> Self {
        Self {
            command,
            flags,
            correlation,
            payload_len,
        }
    }

    /// Encodes the header using the §21 little-endian wire layout.
    pub fn encode(self) -> Result<[u8; IPC_HEADER_LEN], super::IpcError> {
        let mut bytes = [0u8; IPC_HEADER_LEN];
        let mut cursor = std::io::Cursor::new(&mut bytes[..]);
        cursor
            .write_u32::<byteorder::LittleEndian>(IPC_MAGIC)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<byteorder::LittleEndian>(IPC_VERSION)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<byteorder::LittleEndian>(self.command.as_u16())
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<byteorder::LittleEndian>(self.flags)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<byteorder::LittleEndian>(0_u16)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u64::<byteorder::LittleEndian>(self.correlation)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u32::<byteorder::LittleEndian>(self.payload_len)
            .map_err(|_| super::IpcError::HeaderEncodeFailed)?;
        Ok(bytes)
    }

    /// Decodes and validates a fixed IPC header before payload allocation.
    pub fn decode(
        bytes: &[u8; IPC_HEADER_LEN],
        max_payload: super::MaxPayloadBytes,
    ) -> Result<Self, super::IpcError> {
        use byteorder::{LittleEndian, ReadBytesExt};
        use std::io::Cursor;

        let mut cursor = Cursor::new(bytes.as_slice());
        let magic = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        if magic != IPC_MAGIC {
            return Err(super::IpcError::InvalidMagic { actual: magic });
        }

        let version = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        if version != IPC_VERSION {
            return Err(super::IpcError::UnsupportedVersion { actual: version });
        }

        let command = IpcCommand::from_u16(
            cursor
                .read_u16::<LittleEndian>()
                .map_err(|_| super::IpcError::HeaderDecodeFailed)?,
        )?;
        let flags = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        let reserved = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        if reserved != 0 {
            return Err(super::IpcError::ReservedNonZero { actual: reserved });
        }
        let correlation = cursor
            .read_u64::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        let payload_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| super::IpcError::HeaderDecodeFailed)?;
        let payload_len_usize = u32_to_usize(payload_len)?;
        if payload_len_usize > max_payload.get() {
            return Err(super::IpcError::PayloadTooLarge {
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

/// Converts u32 payload length to usize, returning an error if out of range.
fn u32_to_usize(value: u32) -> Result<usize, super::IpcError> {
    match usize::try_from(value) {
        Ok(converted) => Ok(converted),
        Err(_) => Err(super::IpcError::PayloadLengthOutOfRange { actual: value }),
    }
}

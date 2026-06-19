#![forbid(unsafe_code)]
//! IPC error types.

use thiserror::Error;
use vb_core::DiagnosticCode;

/// IPC/memory ingress failures.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpcError {
    /// Queue is full and the producer must apply backpressure.
    #[error("memory ingress queue is full")]
    Full,
    /// All producers or consumers have disconnected.
    #[error("memory ingress queue is disconnected")]
    Disconnected,
    /// Payload exceeds the configured frame limit.
    #[error("ingress payload is too large: actual={actual}, limit={limit}")]
    PayloadTooLarge {
        /// Actual payload bytes.
        actual: usize,
        /// Configured limit.
        limit: usize,
    },
    /// Frame magic did not match `VBLT`.
    #[error("invalid IPC frame magic: actual={actual:#010x}")]
    InvalidMagic {
        /// Decoded magic value.
        actual: u32,
    },
    /// Frame version is not supported by this crate.
    #[error("unsupported IPC frame version: actual={actual}")]
    UnsupportedVersion {
        /// Decoded version value.
        actual: u16,
    },
    /// Command id is not part of the v1 command set.
    #[error("unknown IPC command: {0}")]
    UnknownCommand(u16),
    /// Reserved header field must remain zero.
    #[error("IPC reserved header field is non-zero: actual={actual}")]
    ReservedNonZero {
        /// Decoded reserved value.
        actual: u16,
    },
    /// Header payload length and supplied payload bytes disagree.
    #[error("IPC payload length mismatch: header={header}, actual={actual}")]
    PayloadLengthMismatch {
        /// Header-declared payload length.
        header: usize,
        /// Actual payload bytes supplied to the decoder.
        actual: usize,
    },
    /// Header could not be encoded to the fixed wire length.
    #[error("failed to encode IPC header")]
    HeaderEncodeFailed,
    /// Header bytes could not be read as fixed-width fields.
    #[error("failed to decode IPC header")]
    HeaderDecodeFailed,
    /// Payload length cannot fit this target architecture.
    #[error("IPC payload length cannot fit usize: actual={actual}")]
    PayloadLengthOutOfRange {
        /// Header-declared payload length.
        actual: u32,
    },
    /// Typed Postcard payload encoding failed.
    #[error("failed to encode IPC payload")]
    PayloadEncodeFailed,
    /// Typed Postcard payload decoding failed.
    #[error("failed to decode IPC payload")]
    PayloadDecodeFailed,
    /// Typed response payload decoding failed.
    #[error("failed to decode IPC response")]
    ResponseDecodeFailed,
    /// Frame's caller-capabilities envelope is missing or rejected.
    #[error("IPC frame missing or invalid caller capability envelope")]
    PermissionDenied,
    /// Unix socket peer-credentials check failed at accept time.
    #[error("IPC peer credentials check failed: {0}")]
    PeerCredentialsFailed(
        /// Reason for the peer-credentials rejection.
        &'static str,
    ),
}

impl IpcError {
    /// Runtime code for structurally invalid IPC frames.
    pub const IPC_FRAME_INVALID_RUNTIME_CODE: &str = "IPC_FRAME_INVALID";
    /// Runtime code for IPC payloads exceeding a configured bound.
    pub const IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE: &str = "IPC_PAYLOAD_TOO_LARGE";
    /// Runtime code for bounded IPC ingress queues at capacity.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";

    /// Diagnostic code for queue full.
    pub const FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x3001);
    /// Diagnostic code for disconnected.
    pub const DISCONNECTED_CODE: DiagnosticCode = DiagnosticCode::new(0x3002);
    /// Diagnostic code for payload too large.
    pub const PAYLOAD_TOO_LARGE_CODE: DiagnosticCode = DiagnosticCode::new(0x3003);
    /// Diagnostic code for invalid magic.
    pub const INVALID_MAGIC_CODE: DiagnosticCode = DiagnosticCode::new(0x3004);
    /// Diagnostic code for unsupported version.
    pub const UNSUPPORTED_VERSION_CODE: DiagnosticCode = DiagnosticCode::new(0x3005);
    /// Diagnostic code for unknown command.
    pub const UNKNOWN_COMMAND_CODE: DiagnosticCode = DiagnosticCode::new(0x3006);
    /// Diagnostic code for reserved non-zero.
    pub const RESERVED_NON_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x3007);
    /// Diagnostic code for payload length mismatch.
    pub const PAYLOAD_LENGTH_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x3008);
    /// Diagnostic code for header encode failed.
    pub const HEADER_ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x3009);
    /// Diagnostic code for header decode failed.
    pub const HEADER_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300A);
    /// Diagnostic code for payload length out of range.
    pub const PAYLOAD_LENGTH_OUT_OF_RANGE_CODE: DiagnosticCode = DiagnosticCode::new(0x300B);
    /// Diagnostic code for payload encode failed.
    pub const PAYLOAD_ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300C);
    /// Diagnostic code for payload decode failed.
    pub const PAYLOAD_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300D);
    /// Diagnostic code for response decode failed.
    pub const RESPONSE_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300E);
    /// Diagnostic code for missing or invalid caller capabilities.
    pub const PERMISSION_DENIED_CODE: DiagnosticCode = DiagnosticCode::new(0x300F);
    /// Diagnostic code for peer-credentials rejection on Unix sockets.
    pub const PEER_CREDENTIALS_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x3010);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::Full => Self::FULL_CODE,
            Self::Disconnected => Self::DISCONNECTED_CODE,
            Self::PayloadTooLarge { .. } => Self::PAYLOAD_TOO_LARGE_CODE,
            Self::InvalidMagic { .. } => Self::INVALID_MAGIC_CODE,
            Self::UnsupportedVersion { .. } => Self::UNSUPPORTED_VERSION_CODE,
            Self::UnknownCommand(_) => Self::UNKNOWN_COMMAND_CODE,
            Self::ReservedNonZero { .. } => Self::RESERVED_NON_ZERO_CODE,
            Self::PayloadLengthMismatch { .. } => Self::PAYLOAD_LENGTH_MISMATCH_CODE,
            Self::HeaderEncodeFailed => Self::HEADER_ENCODE_FAILED_CODE,
            Self::HeaderDecodeFailed => Self::HEADER_DECODE_FAILED_CODE,
            Self::PayloadLengthOutOfRange { .. } => Self::PAYLOAD_LENGTH_OUT_OF_RANGE_CODE,
            Self::PayloadEncodeFailed => Self::PAYLOAD_ENCODE_FAILED_CODE,
            Self::PayloadDecodeFailed => Self::PAYLOAD_DECODE_FAILED_CODE,
            Self::ResponseDecodeFailed => Self::RESPONSE_DECODE_FAILED_CODE,
            Self::PermissionDenied => Self::PERMISSION_DENIED_CODE,
            Self::PeerCredentialsFailed(_) => Self::PEER_CREDENTIALS_FAILED_CODE,
        }
    }

    /// Returns the stable section 17 runtime code when this IPC error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Full => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            Self::PayloadTooLarge { .. } | Self::PayloadLengthOutOfRange { .. } => {
                Some(Self::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE)
            }
            Self::InvalidMagic { .. }
            | Self::UnsupportedVersion { .. }
            | Self::UnknownCommand(_)
            | Self::ReservedNonZero { .. }
            | Self::PayloadLengthMismatch { .. }
            | Self::HeaderDecodeFailed
            | Self::PayloadDecodeFailed
            | Self::ResponseDecodeFailed => Some(Self::IPC_FRAME_INVALID_RUNTIME_CODE),
            Self::PermissionDenied | Self::PeerCredentialsFailed(_) => {
                Some(Self::IPC_FRAME_INVALID_RUNTIME_CODE)
            }
            Self::Disconnected | Self::HeaderEncodeFailed | Self::PayloadEncodeFailed => None,
        }
    }
}

/// Converts a u32 payload length to usize, returning an error if it doesn't fit.
///
/// On 64-bit platforms this is always valid since usize is 64-bit and u32 fits.
/// On 32-bit platforms, payloads larger than `u32::MAX` would fail the conversion,
/// but such payloads cannot be represented in the IPC frame anyway (payload_len is u32).
pub(crate) fn u32_to_usize(value: u32) -> Result<usize, IpcError> {
    match usize::try_from(value) {
        Ok(converted) => Ok(converted),
        Err(_) => Err(IpcError::PayloadLengthOutOfRange { actual: value }),
    }
}

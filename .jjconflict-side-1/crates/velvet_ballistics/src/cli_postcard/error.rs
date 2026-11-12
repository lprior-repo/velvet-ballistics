//! CLI Postcard Errors
//!
//! Error types for CLI Postcard decode operations.

/// Errors that can occur during Postcard decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PostcardError {
    /// Magic bytes do not match CLI_MAGIC.
    InvalidMagic,
    /// Header length does not match expected HEADER_SIZE.
    InvalidHeaderLength,
    /// Payload length exceeds MAX_PAYLOAD.
    PayloadTooLarge,
    /// Schema version is older than the supported contract.
    VersionTooOld,
    /// Schema version is newer than the supported contract.
    VersionTooNew,
    /// Message kind is not the supported CLI postcard payload kind.
    WrongKind,
    /// Payload digest check failed.
    DigestMismatch,
    /// CRC check of header failed.
    CrcMismatch,
    /// The decoded payload metadata does not match the supported CLI contract.
    PayloadMetadataMismatch,
    /// The decoded CLI payload body is not valid UTF-8 JSON.
    JsonPayloadDecodeFailed,
    /// Data too short to contain valid header.
    DecodeFailed,
}

impl std::fmt::Display for PostcardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid magic bytes in postcard header"),
            Self::InvalidHeaderLength => write!(f, "invalid header length in postcard"),
            Self::PayloadTooLarge => write!(f, "payload length exceeds maximum"),
            Self::VersionTooOld => write!(f, "postcard schema version is too old"),
            Self::VersionTooNew => write!(f, "postcard schema version is too new"),
            Self::WrongKind => write!(f, "postcard kind is not supported"),
            Self::DigestMismatch => write!(f, "payload digest mismatch"),
            Self::CrcMismatch => write!(f, "header CRC mismatch"),
            Self::PayloadMetadataMismatch => write!(f, "postcard payload metadata mismatch"),
            Self::JsonPayloadDecodeFailed => write!(f, "postcard JSON payload decode failed"),
            Self::DecodeFailed => write!(f, "postcard decode failed: data too short"),
        }
    }
}

impl std::error::Error for PostcardError {}

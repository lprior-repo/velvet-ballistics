//! Emitter-specific error types.

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmitterError {
    YamlEncodeFailed,
    PostcardEncodeFailed,
    PostcardDecodeFailed,
    PayloadTooLarge { len: u32, max: u32 },
    LengthOverflow,
    HeaderChecksumMismatch,
    PayloadDigestMismatch,
    UnexpectedEof,
    BadMagic { found: u32 },
    HeaderLengthMismatch { found: u32 },
    MigrationRequired { from: u16, to: u16 },
    UnsupportedSchemaVersion { version: u16 },
    PayloadLengthOverflow { len: u32 },
    UnknownKind { kind: u16 },
    AnsiForbidden,
}

impl fmt::Display for EmitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitterError::YamlEncodeFailed => write!(f, "YAML encoding failed"),
            EmitterError::PostcardEncodeFailed => write!(f, "Postcard encoding failed"),
            EmitterError::PostcardDecodeFailed => write!(f, "Postcard decoding failed"),
            EmitterError::PayloadTooLarge { len, max } => {
                write!(f, "payload length {} exceeds maximum {}", len, max)
            }
            EmitterError::LengthOverflow => write!(f, "length overflow in header computation"),
            EmitterError::HeaderChecksumMismatch => write!(f, "CRC32C header checksum mismatch"),
            EmitterError::PayloadDigestMismatch => write!(f, "BLAKE3 payload digest mismatch"),
            EmitterError::UnexpectedEof => {
                write!(f, "envelope bytes shorter than declared header")
            }
            EmitterError::BadMagic { found } => {
                write!(f, "wrong magic bytes: found {found:#x}, expected VBLI")
            }
            EmitterError::HeaderLengthMismatch { found } => {
                write!(f, "header length {} is not the expected 52 bytes", found)
            }
            EmitterError::MigrationRequired { from, to } => {
                write!(
                    f,
                    "binary schema version {} requires migration to {}",
                    from, to
                )
            }
            EmitterError::UnsupportedSchemaVersion { version } => {
                write!(f, "unsupported binary schema version: {}", version)
            }
            EmitterError::PayloadLengthOverflow { len } => {
                write!(f, "payload length {} would overflow during allocation", len)
            }
            EmitterError::UnknownKind { kind } => {
                write!(f, "unknown envelope kind: {}", kind)
            }
            EmitterError::AnsiForbidden => {
                write!(f, "ANSI escape sequences are forbidden in machine output")
            }
        }
    }
}

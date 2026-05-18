/// Accepted artifact validation failure source used at admission boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ArtifactInvalidSource {
    /// Payload digest validation failed.
    #[error("payload digest mismatch")]
    PayloadDigestMismatch,
}

/// Artifact envelope validation errors for accepted artifact v1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ArtifactEnvelopeError {
    /// Record magic did not match the expected family.
    #[error("bad record magic: {found:#010x}")]
    BadMagic {
        /// Found magic value.
        found: u32,
    },
    /// Record schema version is not supported.
    #[error("unsupported record schema version: {version}")]
    UnsupportedSchemaVersion {
        /// Found schema version.
        version: u16,
    },
    /// Record schema requires explicit migration.
    #[error("record schema migration required from {from} to {to}")]
    MigrationRequired {
        /// Found schema version.
        from: u16,
        /// Current schema version.
        to: u16,
    },
    /// Record kind is not valid for this magic family.
    #[error("record kind {kind} does not belong to magic {magic:#010x}")]
    RecordKindFamilyMismatch {
        /// Magic value.
        magic: u32,
        /// Record kind.
        kind: u16,
    },
    /// Header length was not the contract value.
    #[error("record header length mismatch: {found}")]
    HeaderLengthMismatch {
        /// Found header length.
        found: u32,
    },
    /// Payload length exceeded the configured maximum.
    #[error("record payload too large: {len} > {max}")]
    PayloadTooLarge {
        /// Payload length.
        len: u32,
        /// Maximum allowed length.
        max: u32,
    },
    /// Header CRC32C did not match.
    #[error("record header checksum mismatch")]
    HeaderChecksumMismatch,
    /// Payload BLAKE3 digest did not match.
    #[error("record payload digest mismatch")]
    PayloadDigestMismatch,
    /// Record ended before the declared header or payload length.
    #[error("unexpected end of record")]
    UnexpectedEof,
    /// Postcard payload decode failed.
    #[error("postcard payload decode failed")]
    PostcardDecodeFailed,
    /// Verification gate count is not 15.
    #[error("invalid gate count: {found}")]
    InvalidGateCount {
        /// Found gate count.
        found: u8,
    },
    /// A required proof flag is false.
    #[error("missing required proof flag: {flag:?}")]
    MissingRequiredProofFlag {
        /// The flag that is missing.
        flag: crate::admission::ProofFlag,
    },
}

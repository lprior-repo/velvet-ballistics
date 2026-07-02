#![forbid(unsafe_code)]
//! Key decode error types for storage key parsing.
//!
//! Separate from `JournalError` because key decoding is a pure parsing
//! operation that does not involve I/O or persistence.

/// Errors produced by `decode_storage_key` and `try_key_prefix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyDecodeError {
    /// Input is empty (length 0).
    EmptyKey,
    /// First byte is not one of the nine known prefix constants.
    UnknownPrefix {
        /// The unknown prefix byte encountered.
        prefix: u8,
    },
    /// Input length does not match the expected key size for the given prefix.
    KeyLengthMismatch {
        /// The prefix byte (one of the nine known prefixes).
        prefix: u8,
        /// The expected key length in bytes.
        expected: usize,
        /// The actual input length in bytes.
        actual: usize,
    },
    /// RunId field decoded as 0 (invalid per domain rules).
    InvalidRunId,
    /// EventSeq field decoded as u64::MAX (reserved sentinel).
    ReservedSeqSentinel,
}

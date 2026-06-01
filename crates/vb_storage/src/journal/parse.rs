//! Journal event parsing utilities.

use crate::{
    codec::decode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
};

/// Parses a journal event from raw bytes.
///
/// This is a convenience wrapper around [`decode_record`] that fixes the magic
/// and max payload length to the journal event contract values and returns
/// only the deserialized event (dropping the envelope).
///
/// # Errors
///
/// Returns an error if the bytes do not form a valid journal event record:
/// - [`JournalError::UnexpectedEof`] if the input is too short
/// - [`JournalError::BadMagic`] if the magic bytes do not match `MAGIC_JOURNAL_EVENT`
/// - [`JournalError::UnsupportedSchemaVersion`] if the schema version is not supported
/// - [`JournalError::UnknownRecordKind`] if the record kind is not a known journal event kind
/// - [`JournalError::PayloadTooLarge`] if the payload exceeds `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`
/// - [`JournalError::PayloadDigestMismatch`] if the payload digest does not match
/// - [`JournalError::UnexpectedTrailingBytes`] with byte offsets if bytes remain after the declared payload
/// - [`JournalError::HeaderChecksumMismatch`] if the header CRC fails
/// - [`JournalError::PostcardDecodeFailed`] if postcard deserialization fails
pub fn parse_event(data: &[u8]) -> Result<JournalEvent, JournalError> {
    let (_, event) =
        decode_record::<JournalEvent>(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    Ok(event)
}

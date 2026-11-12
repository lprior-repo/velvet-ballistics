#![forbid(unsafe_code)]
use super::JournalEvent;
use crate::JournalError;
use vb_core::SlotValue;

impl JournalEvent {
    /// Returns the slot value if this is a `SlotWrittenEvent` and a value was captured.
    ///
    /// Returns `Ok(None)` if no value was captured (absent optional payload).
    /// Returns `Ok(Some(slot_value))` if decoding succeeded.
    /// Returns `Err(JournalError::PostcardDecodeFailed(_))` if bytes are corrupt/truncated.
    /// Returns `Err(JournalError::PayloadTooLarge)` if bytes exceed the maximum allowed size.
    #[must_use = "slot_value returns a fallible result that must be handled"]
    pub fn slot_value(&self) -> Result<Option<SlotValue>, JournalError> {
        match self {
            Self::SlotWrittenEvent {
                value: Some(bytes), ..
            } => {
                // First check payload size bounds before attempting decode.
                let max_bytes = crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
                let len_u32 =
                    u32::try_from(bytes.len()).map_err(|_| JournalError::PayloadTooLarge {
                        len: u32::MAX,
                        max: max_bytes,
                    })?;
                if len_u32 > max_bytes {
                    return Err(JournalError::PayloadTooLarge {
                        len: len_u32,
                        max: max_bytes,
                    });
                }
                // Decode with typed error propagation instead of silent erasure.
                postcard::from_bytes(bytes)
                    .map(Some)
                    .map_err(JournalError::PostcardDecodeFailed)
            }
            // Explicit absent branch for optional payloads.
            Self::SlotWrittenEvent { value: None, .. } => Ok(None),
            // Non-SlotWrittenEvent variants have no slot value.
            _ => Ok(None),
        }
    }
}

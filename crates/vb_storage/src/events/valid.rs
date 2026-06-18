#![forbid(unsafe_code)]
//! Structural validity and slot-value decoding for `JournalEvent`.

use crate::error::JournalError;
use crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
use super::variant::JournalEvent;
use vb_core::SlotValue;

impl JournalEvent {
    /// Returns the slot value if this is a `SlotWrittenEvent` and a value was captured.
    ///
    /// Returns `Ok(None)` if no value was captured (absent optional payload).
    /// Returns `Ok(Some(slot_value))` if decoding succeeded.
    /// Returns `Err(JournalError::PostcardDecodeFailed)` if bytes are corrupt/truncated.
    /// Returns `Err(JournalError::PayloadTooLarge)` if bytes exceed the maximum allowed size.
    #[must_use = "slot_value returns a fallible result that must be handled"]
    pub fn slot_value(&self) -> Result<Option<SlotValue>, JournalError> {
        match self {
            Self::SlotWrittenEvent {
                value: Some(bytes), ..
            } => {
                // First check payload size bounds before attempting decode.
                let max_bytes = MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
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
                match postcard::from_bytes(bytes) {
                    Ok(value) => Ok(Some(value)),
                    Err(_) => Err(JournalError::PostcardDecodeFailed),
                }
            }
            // Explicit absent branch for optional payloads.
            Self::SlotWrittenEvent { value: None, .. } => Ok(None),
            // Non-SlotWrittenEvent variants have no slot value.
            _ => Ok(None),
        }
    }

    /// Returns true if this event passes basic structural validity checks.
    ///
    /// A valid event has:
    /// - A non-zero run identifier (zero is the null/placeholder value)
    /// - A sequence number within reasonable bounds (not u64::MAX)
    /// - Attempt numbers, when present, are non-zero (zero is ambiguous in replay)
    ///
    /// Note: This does NOT validate cryptographic digests or semantic consistency.
    /// Those are enforced by the codec round-trip and replay validation.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        // RunId(0) is the zero/placeholder value - valid events must have a real run
        if self.run_id().get() == 0 {
            return false;
        }
        // Sequence must not be at the max value (overflow sentinel)
        if self.seq().get() == u64::MAX {
            return false;
        }
        // Attempt numbers must be non-zero when present (zero is ambiguous)
        match self {
            Self::ActionScheduled { attempt, .. }
            | Self::ActionCompletedEvent { attempt, .. }
            | Self::ActionFailedEvent { attempt, .. }
            | Self::SlotWrittenEvent { attempt, .. }
            | Self::WaitScheduledEvent { attempt, .. }
            | Self::AskScheduledEvent { attempt, .. }
            | Self::AskAnsweredEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. } => *attempt != 0,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. } => ticket.attempt != 0,
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => true,
        }
    }
}

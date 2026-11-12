#![forbid(unsafe_code)]
//! Versioned, exact retry-delay authority; legacy waits carry none of these facts.

use vb_core::action::ActionTicket;

/// Version one retry timer payload, bound to a complete runtime checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetryTimerV1 {
    /// Exact supported payload version.
    pub version: u16,
    /// Failed invocation authorizing this timer.
    pub failed_ticket: ActionTicket,
    /// Next action attempt; the run attempt remains unchanged.
    pub next_attempt: u16,
    /// Persisted freshness token, preserved on recovery.
    pub generation: u64,
    /// UTC milliseconds sampled before the failure batch is appended.
    pub scheduled_unix_ms: u64,
    /// UTC milliseconds at which the next attempt may be scheduled.
    pub deadline_unix_ms: u64,
    /// Digest of the exact checkpoint bytes.
    pub checkpoint_digest: [u8; 32],
    /// Complete validated runtime state, encoded by its versioned checkpoint API.
    pub checkpoint: Vec<u8>,
}

impl RetryTimerV1 {
    /// Checks fixed bounds and integrity before runtime-specific identity validation.
    pub fn validate(&self) -> Result<(), crate::JournalError> {
        let ticket = self.failed_ticket;
        if self.version != 1
            || self.generation == 0
            || ticket.run.get() == 0
            || ticket.run_attempt == 0
            || ticket.attempt == 0
            || ticket.attempt.checked_add(1) != Some(self.next_attempt)
            || self.next_attempt > ticket.capacity
            || ticket.capacity > vb_core::limits::MAX_RETRY_ATTEMPTS
            || self.deadline_unix_ms <= self.scheduled_unix_ms
            || self
                .deadline_unix_ms
                .checked_sub(self.scheduled_unix_ms)
                .is_none_or(|delay| delay > vb_core::workflow::CompiledRetryPolicy::MAX_DELAY_MS)
            || self.checkpoint.is_empty()
            || u32::try_from(self.checkpoint.len())
                .ok()
                .is_none_or(|len| len > crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES)
            || !vb_core::action::action_ticket_has_valid_key(ticket)
            || blake3::hash(&self.checkpoint).as_bytes() != &self.checkpoint_digest
        {
            return Err(crate::JournalError::InvalidEvent);
        }
        Ok(())
    }
}

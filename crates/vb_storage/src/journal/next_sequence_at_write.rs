#![forbid(unsafe_code)]
//! `next_sequence_at_write` — write-time guard for Fjall-backed journals.
//!
//! The function is the kernel of the next-sequence-at-write contract
//! (C-2 in `.beads/vb-r8oso/contract.md`). Every append path
//! (`append_journaled`, `append_strict`, `append_strict_batch`,
//! `append_unfsynced`, `JournalWriteBatch::append_event`) consults this
//! function before committing an event. The contract is:
//!
//! - For a fresh run (no durable events), return `Ok(EventSeq::ZERO)`.
//! - Otherwise return `Ok(last_durable_event_seq(run).succ())`.
//! - On `EventSeq::MAX` saturation, return `Err(JournalError::SequenceOverflow)`.
//! - For `RunId::ZERO`, return `Err(JournalError::InvalidRunId)` so the
//!   invalid-identifier semantics stay consistent across storage paths.
//!
//! The lookup is key-only: `self.events.prefix(prefix).next_back()` with
//! no event-value decode. That avoids the BLAKE3 + postcard cost of
//! decoding every committed event under the run prefix.

use crate::codec;
use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::keys::run_prefix_key;
use crate::types::EventSeq;
use crate::types::StorageKey;
use vb_core::RunId;

impl FjallJournal {
    /// Returns the sequence value that the next successful append for
    /// `run` must carry.
    ///
    /// Semantics:
    ///
    /// - `EventSeq::ZERO` when no event has been durably written for `run`.
    /// - `last_durable_event_seq(run).succ()` otherwise.
    /// - `Err(JournalError::SequenceOverflow)` if the succ overflows
    ///   `u64::MAX`.
    /// - `Err(JournalError::InvalidRunId)` for `RunId::ZERO`.
    ///
    /// Implementation contract:
    ///
    /// - Key-only Fjall `prefix().next_back()` traversal; no event-value
    ///   decode (skips BLAKE3 + postcard).
    /// - Caller-observable atomic with the next append that follows.
    /// - Lock-free: uses the durable LSM snapshot visible to
    ///   `events.contains_key`. The function MUST NOT acquire
    ///   `self.write_lock`.
    /// - Never returns `Ok(EventSeq::MAX)` and never panics.
    ///
    /// # Errors
    ///
    /// Returns `JournalError::InvalidRunId` for `RunId::ZERO`,
    /// `JournalError::SequenceOverflow` when the durable tail has
    /// reached `EventSeq::MAX`, and
    /// `JournalError::MalformedKeyspaceRow` if a key under the run
    /// prefix is not a well-formed `StorageKey::RunEvent`.
    pub fn next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError> {
        if run == RunId::ZERO {
            return Err(JournalError::InvalidRunId { run });
        }
        match self.last_durable_event_seq(run)? {
            None => Ok(EventSeq::ZERO),
            Some(seq) => codec::next_seq(seq),
        }
    }

    /// Returns the largest sequence currently present in the `events`
    /// keyspace for `run`, or `None` if no events are stored.
    ///
    /// Key-only lookup. Used by [`Self::next_sequence_at_write`] and
    /// available for read-only callers that need to confirm the
    /// durable tail without decoding every value.
    ///
    /// # Errors
    ///
    /// Returns `JournalError::MalformedKeyspaceRow` for any key under
    /// the run prefix that is not a well-formed `StorageKey::RunEvent`,
    /// or any Fjall error propagated from the keyspace iterator.
    fn last_durable_event_seq(&self, run: RunId) -> Result<Option<EventSeq>, JournalError> {
        let prefix = run_prefix_key(run)?;
        let Some(item) = self.events.prefix(prefix).next_back() else {
            return Ok(None);
        };
        let (key, _) = item.into_inner().map_err(JournalError::from)?;
        match crate::keys::decode_storage_key(&key) {
            Ok(StorageKey::RunEvent { seq, .. }) => Ok(Some(seq)),
            Ok(_) => Err(JournalError::MalformedKeyspaceRow {
                prefix: crate::constants::PREFIX_RUN_EVENT,
                expected_len: crate::constants::JOURNAL_KEY_BYTES,
                actual_len: key.len(),
            }),
            Err(_) => Err(JournalError::MalformedKeyspaceRow {
                prefix: crate::constants::PREFIX_RUN_EVENT,
                expected_len: crate::constants::JOURNAL_KEY_BYTES,
                actual_len: key.len(),
            }),
        }
    }
}

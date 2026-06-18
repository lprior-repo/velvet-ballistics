#![forbid(unsafe_code)]
//! Atomic cross-keyspace write batch backed by Fjall.
//!
//! Accumulates writes across multiple keyspaces and commits them
//! atomically with a single WAL fsync.

use std::collections::HashSet;

use crate::{
    codec::encode_record,
    constants::{
        JOURNAL_KEY_BYTES, MAGIC_BLOB, MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT,
        MAGIC_WORKFLOW_SOURCE, MAX_BATCH_COUNT, MAX_BLOB_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    events::JournalEvent,
    keys::{
        blob_key, index_action_key, index_status_key, index_workflow_key, run_event_key,
        run_header_key, run_snapshot_key, workflow_source_key,
    },
    records::{BlobRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord},
    recovery::RunSnapshot,
};

use crate::journal::FjallJournal;

/// Default journal batch encoded-byte budget (1 MiB).
///
/// Matches the core `max_journal_batch_bytes` default of `1_048_576`.
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

// =========================================================================
// Domain types: BatchState and BatchByteLimit
// =========================================================================

/// Lifecycle state of a [`JournalWriteBatch`].
///
/// - [`BatchState::Open`] — the batch accepts operations.
/// - [`BatchState::Aborted`] — an unrecoverable error occurred; the batch
///   must not stage further writes and will silently discard on commit.
///
/// This is an explicit state machine rather than a boolean flag, making
/// illegal states unrepresentable at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatchState {
    /// The batch is accepting operations.
    #[default]
    Open,
    /// The batch was aborted due to a domain error (e.g. duplicate event).
    Aborted,
}

impl BatchState {
    /// Returns `true` when the batch is in the aborted state.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted)
    }
}

/// Strongly-typed byte budget for journal event admission.
///
/// Wraps `u64` so that callers cannot accidentally pass an arbitrary
/// integer where a batch byte limit is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BatchByteLimit(u64);

impl BatchByteLimit {
    /// Creates a byte limit from the given capacity.
    #[inline]
    pub const fn bounded(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Returns the limit as a plain `u64`.
    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Atomic cross-keyspace write batch backed by Fjall.
///
/// Accumulates writes across multiple keyspaces and commits them
/// atomically with a single WAL fsync.
///
/// # Invariant I1
/// `JournalWriteBatch` is `!Send + !Sync` because it contains
/// `PhantomData<*mut FjallJournal>` which is `!Send + !Sync`,
/// preventing any batch handle from crossing thread boundaries.
pub struct JournalWriteBatch<'j> {
    inner: fjall::OwnedWriteBatch,
    journal: &'j FjallJournal,
    staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>,
    /// Tracks staged compiled IR digests and their metadata hashes
    /// to detect same-batch metadata mutation attempts.
    #[cfg(test)]
    staged_ir_hashes: std::collections::HashMap<vb_core::WorkflowDigest, [u8; 32]>,
    /// Explicit lifecycle state: open or aborted.
    state: BatchState,
    /// Accumulated encoded-byte total for journal events accepted in this batch.
    staged_bytes: u64,
    /// Maximum encoded-byte budget for journal events in this batch.
    byte_limit: BatchByteLimit,
    _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}

impl<'j> JournalWriteBatch<'j> {
    /// Creates a new batch for the given journal.
    pub fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
            staged_event_keys: HashSet::new(),
            #[cfg(test)]
            staged_ir_hashes: std::collections::HashMap::new(),
            state: BatchState::default(),
            staged_bytes: 0,
            byte_limit: BatchByteLimit::bounded(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
            _not_send_or_sync: core::marker::PhantomData,
        }
    }

    /// Inserts a workflow source record into the batch.
    ///
    /// The source bytes are verified against the claimed digest before staging.
    pub fn put_workflow_source(
        &mut self,
        record: &WorkflowSourceRecord,
    ) -> Result<(), JournalError> {
        if let Err(e) =
            crate::journal::verify_content_digest(&record.source, &record.digest.as_bytes())
        {
            self.state = BatchState::Aborted;
            return Err(e);
        }
        let key = match workflow_source_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        let value = match encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            record,
            MAX_WORKFLOW_SOURCE_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.workflow_source, key, value);
        Ok(())
    }

    /// Inserts a compiled IR record into the batch.
    ///
    /// SECURITY: This is pub(crate) to restrict access to admission path only.
    /// External callers MUST use `submit_artifact` or `admit_compiled_artifact`
    /// which properly bind all artifact metadata (warnings, capabilities, seq).
    ///
    /// This queues the insert for atomic commit. The metadata hash is computed
    /// and stored with the record to prevent same-digest metadata mutation attacks.
    /// SECURITY: Validates metadata hash against existing records (both in the
    /// batch and in the journal) before inserting, preventing bypass attacks.
    #[cfg(test)]
    pub(crate) fn put_compiled_ir(
        &mut self,
        record: &crate::records::CompiledIrRecord,
    ) -> Result<(), JournalError> {
        // Validate the record structure first
        if let Err(e) = crate::admission::validate_compiled_ir_record(record) {
            self.state = BatchState::Aborted;
            return Err(e);
        }

        // Decode artifact to compute metadata hash
        let artifact = match crate::admission::decode_accepted_artifact_envelope(&record.ir) {
            Ok(a) => a,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        let h_pending = crate::admission::compute_artifact_metadata_hash(&artifact);

        // SECURITY: Check for same-batch staged record first
        // This catches mutation attempts within the same batch
        if let Some(&h_staged) = self.staged_ir_hashes.get(&record.digest) {
            if h_pending != h_staged {
                self.state = BatchState::Aborted;
                return Err(JournalError::MetadataMutation {
                    digest: record.digest,
                });
            }
        }

        // SECURITY: Check for existing record in journal and validate metadata hash
        // This prevents same-digest metadata mutation attacks via batch API
        let key = match crate::keys::compiled_ir_key(record.digest.as_bytes()) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        if let Ok(Some(existing)) = self.journal.compiled_ir(record.digest) {
            let existing_hash = existing.metadata_hash;
            match existing_hash {
                Some(h_existing) => {
                    // Subsequent write: metadata hash must match exactly
                    if h_pending != h_existing {
                        self.state = BatchState::Aborted;
                        return Err(JournalError::MetadataMutation {
                            digest: record.digest,
                        });
                    }
                }
                None => {
                    // Backward compatibility: existing record predates metadata hash.
                    // Compute hash from existing artifact - if it differs from pending,
                    // this indicates different artifacts with same digest (reject).
                    let existing_artifact =
                        crate::admission::decode_accepted_artifact_envelope(&existing.ir)?;
                    let h_existing =
                        crate::admission::compute_artifact_metadata_hash(&existing_artifact);
                    if h_pending != h_existing {
                        self.state = BatchState::Aborted;
                        return Err(JournalError::MetadataMutation {
                            digest: record.digest,
                        });
                    }
                }
            }
        }

        // Track this staged digest and its hash for same-batch detection
        self.staged_ir_hashes.insert(record.digest, h_pending);

        // Create record with computed metadata hash
        let mut record_with_hash = record.clone();
        record_with_hash.metadata_hash = Some(h_pending);

        // Encode the record
        let value = match encode_record(
            crate::constants::MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record_with_hash,
            crate::constants::MAX_COMPILED_IR_BYTES,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };

        // Queue the insert for atomic commit
        self.inner.insert(&self.journal.compiled_ir, key, value);
        Ok(())
    }

    /// Inserts a run header record into the batch.
    pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = run_header_key(record.run)?;
        let value = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            record,
            MAX_RUN_HEADER_BYTES,
        )?;
        self.inner.insert(&self.journal.run_header, key, value);
        Ok(())
    }

    /// Inserts a run snapshot record into the batch.
    pub fn put_snapshot(&mut self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
        let value = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        )?;
        self.inner.insert(&self.journal.run_snapshot, key, value);
        Ok(())
    }

    /// Inserts a blob record into the batch.
    ///
    /// The blob bytes are verified against the claimed digest before staging.
    pub fn put_blob(&mut self, record: &BlobRecord) -> Result<(), JournalError> {
        if let Err(e) = crate::journal::verify_content_digest(&record.bytes, &record.digest) {
            self.state = BatchState::Aborted;
            return Err(e);
        }
        let key = match blob_key(record.digest) {
            Ok(k) => k,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        let value = match encode_record(MAGIC_BLOB, RecordKind::Blob, 0, record, MAX_BLOB_BYTES) {
            Ok(v) => v,
            Err(e) => {
                self.state = BatchState::Aborted;
                return Err(e);
            }
        };
        self.inner.insert(&self.journal.blob, key, value);
        Ok(())
    }

    /// Inserts a status index marker into the batch.
    pub fn put_status_index(
        &mut self,
        state: crate::types::IndexStatusState,
        timestamp: u64,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.inner
            .insert(&self.journal.index_status, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts a workflow index marker into the batch.
    pub fn put_workflow_index(
        &mut self,
        workflow: vb_core::WorkflowId,
        run: vb_core::RunId,
    ) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.inner
            .insert(&self.journal.index_workflow, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts an action index marker into the batch.
    pub fn put_action_index(
        &mut self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.inner
            .insert(&self.journal.index_action, key, Vec::<u8>::new());
        Ok(())
    }

    /// Appends a journal event into the batch.
    ///
    /// # Invariant I20
    /// Duplicate event detection is enforced at `append_event` time by
    /// checking the journal's keyset for already-committed events.
    /// Same-batch idempotent inserts are allowed (duplicates within
    /// the same batch are collapsed at commit time).
    ///
    /// # Guard Precedence (C6)
    /// 1. Key construction
    /// 2. Durable duplicate check → aborts batch
    /// 3. Count capacity check (QueueFull)
    /// 4. Per-record encoding / payload size check (PayloadTooLarge)
    /// 5. Accumulated byte admission check (JournalBatchBytesExceeded)
    /// 6. Insert into inner OwnedWriteBatch
    ///
    /// # Preconditions (requires)
    /// - The batch is not already aborted.
    /// - `event.run_id()` and `event.seq()` form a valid key.
    /// - `event` payload is bounded by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`.
    ///
    /// # Postconditions (ensures)
    /// - On success: the event is staged in `inner`, `staged_bytes` is
    ///   incremented by the full encoded record length.
    /// - On `DuplicateEvent`: batch is aborted, no state mutated.
    /// - On `QueueFull`: no state mutated, batch remains open.
    /// - On `PayloadTooLarge`: no state mutated.
    /// - On `JournalBatchBytesExceeded`: no state mutated,
    ///   `staged_bytes` unchanged, batch remains open.
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.journal.events.contains_key(key)? {
            self.state = BatchState::Aborted;
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        if self.inner.len() >= MAX_BATCH_COUNT {
            return Err(JournalError::QueueFull);
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;

        // Byte admission check: guard 5 per C6 contract.
        //
        // Uses checked_add to avoid overflow; overflow is rejected
        // with the same JournalBatchBytesExceeded error as a budget
        // overrun.  The encoded_len conversion is try_from on
        // principle, though the bounded payload guarantees it always
        // fits in u64 on all practical targets.
        let limit = self.byte_limit.as_u64();
        if limit > 0 {
            let encoded_len =
                u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?;
            let attempted = match self.staged_bytes.checked_add(encoded_len) {
                Some(total) => total,
                None => {
                    return Err(JournalError::JournalBatchBytesExceeded {
                        attempted: u64::MAX,
                        limit,
                    });
                }
            };
            if attempted > limit {
                return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
            }
            self.staged_bytes = attempted;
        }

        self.inner.insert(&self.journal.events, key, value);
        Ok(())
    }

    /// Returns the number of operations in the batch.
    ///
    /// When the batch is aborted, returns `0` to prevent callers from
    /// assuming the staged count is still valid.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.state.is_aborted() { 0 } else { self.inner.len() }
    }

    /// Returns true if the batch contains no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the accumulated encoded-byte total for journal events
    /// accepted in this batch so far.
    #[must_use]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Returns the byte limit for this batch, if one is set.
    #[must_use]
    pub fn byte_limit(&self) -> Option<u64> {
        let limit = self.byte_limit.as_u64();
        if limit > 0 { Some(limit) } else { None }
    }

    /// Sets strict durability for the commit.
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }

    /// Commits the batch atomically.
    pub fn commit(self) -> Result<(), JournalError> {
        if self.state.is_aborted() {
            return Ok(());
        }
        self.inner.commit()?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "byte_accounting_tests.rs"]
mod byte_accounting_tests;

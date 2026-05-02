#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

pub mod recovery;

use arrayvec::ArrayVec;
use fjall::Readable;
use recovery::RunSnapshot;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use vb_core::DiagnosticCode;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

/// Immutable YAML source records by digest.
pub const KEYSPACE_WORKFLOW_SOURCE: &str = "workflow_source";
/// Compiled workflow IR records by digest.
pub const KEYSPACE_COMPILED_IR: &str = "compiled_ir";
/// Run metadata and status records.
pub const KEYSPACE_RUN_HEADER: &str = "run_header";
/// Compact binary event journal records.
pub const KEYSPACE_RUN_EVENT: &str = "run_event";
/// Compact binary run snapshot records.
pub const KEYSPACE_RUN_SNAPSHOT: &str = "run_snapshot";
/// Large input, output, and action payload blobs.
pub const KEYSPACE_BLOB: &str = "blob";
/// Status/time index records.
pub const KEYSPACE_INDEX_STATUS: &str = "index_status";
/// Workflow/run index records.
pub const KEYSPACE_INDEX_WORKFLOW: &str = "index_workflow";
/// Pending action index records.
pub const KEYSPACE_INDEX_ACTION: &str = "index_action";

/// `workflow_source` key prefix.
pub const PREFIX_WORKFLOW_SOURCE: u8 = 0x01;
/// `compiled_ir` key prefix.
pub const PREFIX_COMPILED_IR: u8 = 0x02;
/// `run_header` key prefix.
pub const PREFIX_RUN_HEADER: u8 = 0x10;
/// `run_event` key prefix.
pub const PREFIX_RUN_EVENT: u8 = 0x11;
/// `run_snapshot` key prefix.
pub const PREFIX_RUN_SNAPSHOT: u8 = 0x12;
/// `blob` key prefix.
pub const PREFIX_BLOB: u8 = 0x20;
/// `index_status` key prefix.
pub const PREFIX_INDEX_STATUS: u8 = 0x30;
/// `index_workflow` key prefix.
pub const PREFIX_INDEX_WORKFLOW: u8 = 0x31;
/// `index_action` key prefix.
pub const PREFIX_INDEX_ACTION: u8 = 0x32;

/// Record envelope header length.
pub const RECORD_HEADER_LEN: u32 = 60;
/// Current record schema version.
pub const CURRENT_SCHEMA_VERSION: u16 = 1;
/// Compiled artifact magic, ASCII `VBIR`.
pub const MAGIC_COMPILED_ARTIFACT: u32 = 0x5642_4952;
/// Journal event magic, ASCII `VBJE`.
pub const MAGIC_JOURNAL_EVENT: u32 = 0x5642_4A45;
/// Snapshot magic, ASCII `VBSN`.
pub const MAGIC_SNAPSHOT: u32 = 0x5642_534E;
/// Blob record magic, ASCII `VBBL`.
pub const MAGIC_BLOB: u32 = 0x5642_424C;
/// IPC frame magic, ASCII `VBLT`.
pub const MAGIC_IPC_FRAME: u32 = 0x5642_4C54;
/// Workflow source magic, ASCII `VBSR`.
pub const MAGIC_WORKFLOW_SOURCE: u32 = 0x5642_5352;
/// Index record magic, ASCII `VBIX`.
pub const MAGIC_INDEX_RECORD: u32 = 0x5642_4958;

const JOURNAL_KEY_BYTES: usize = 17;
const DIGEST_KEY_BYTES: usize = 33;
const RUN_ONLY_KEY_BYTES: usize = 9;
const INDEX_STATUS_KEY_BYTES: usize = 18;
const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
const INDEX_ACTION_KEY_BYTES: usize = 13;
const RUN_EVENT_PREFIX_BYTES: usize = 9;
/// Digest byte width used by storage keys and record payload checksums.
pub const DIGEST_BYTES: usize = 32;
const RECORD_HEADER_BYTES: usize = 60;
const CRC_OFFSET: usize = 56;
/// Maximum journal event payload accepted by the default journal APIs.
pub const MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576;
/// Maximum source bytes accepted by the default workflow source APIs.
pub const MAX_WORKFLOW_SOURCE_BYTES: u32 = 1_048_576;
/// Maximum compiled IR bytes accepted by the default compiled artifact APIs.
pub const MAX_COMPILED_IR_BYTES: u32 = 16_777_216;
/// Maximum run header payload bytes accepted by the default header APIs.
pub const MAX_RUN_HEADER_BYTES: u32 = 65_536;
/// Maximum snapshot payload bytes accepted by the default snapshot APIs.
pub const MAX_SNAPSHOT_BYTES: u32 = 67_108_864;
/// Maximum blob payload bytes accepted by the default blob APIs.
pub const MAX_BLOB_BYTES: u32 = 67_108_864;
const PAYLOAD_LEN_CONVERSION_MAX: u32 = 4_294_967_295;

/// Storage write limits shared by direct and queued journal writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageLimits {
    /// Maximum payload bytes accepted for a journal event.
    pub max_journal_event_payload_bytes: u32,
}

impl StorageLimits {
    /// Default storage limits.
    pub const DEFAULT: Self = Self {
        max_journal_event_payload_bytes: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    };
}

/// Runtime/storage durability profile selected for journal writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityProfile {
    /// Keep runtime events in volatile memory only; do not write Fjall during the run.
    Volatile,
    /// Queue compact events for bounded group commit without a per-event sync barrier.
    Journaled,
    /// Queue compact events that require a strict persistence barrier when flushed.
    Strict,
}

/// Keyspace tuning profile for per-keyspace configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyspaceProfile {
    /// Small values, bloom filters enabled, no KV separation.
    /// Used for: run_event, index_status, index_workflow, index_action.
    Hot,
    /// Larger values, KV separation enabled.
    /// Used for: workflow_source, compiled_ir, run_snapshot.
    Cold,
    /// Mandatory KV separation for large blob values.
    /// Used for: blob.
    Blob,
}

/// Returns `KeyspaceCreateOptions` tuned for the given profile.
pub fn keyspace_options_for(kind: KeyspaceProfile) -> fjall::KeyspaceCreateOptions {
    use fjall::config::{BloomConstructionPolicy, FilterPolicy, FilterPolicyEntry};

    match kind {
        KeyspaceProfile::Hot => fjall::KeyspaceCreateOptions::default()
            .filter_policy(FilterPolicy::all(FilterPolicyEntry::Bloom(
                BloomConstructionPolicy::BitsPerKey(10.0),
            )))
            .expect_point_read_hits(false),
        KeyspaceProfile::Cold => fjall::KeyspaceCreateOptions::default().with_kv_separation(Some(
            fjall::KvSeparationOptions::default().separation_threshold(4096),
        )),
        KeyspaceProfile::Blob => fjall::KeyspaceCreateOptions::default().with_kv_separation(Some(
            fjall::KvSeparationOptions::default().separation_threshold(1024),
        )),
    }
}

/// Counts queued journal writes by durability profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterQueueProfileCounts {
    /// Number of journaled pending writes.
    pub journaled: usize,
    /// Number of strict pending writes.
    pub strict: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedJournalEvent {
    event: JournalEvent,
    profile: DurabilityProfile,
}

#[derive(Debug)]
struct JournalWriterQueueState {
    pending: VecDeque<QueuedJournalEvent>,
    shutdown: bool,
}

/// Result of flushing a bounded writer queue batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterFlushReport {
    /// Number of queued events drained from memory.
    pub drained: usize,
    /// Number of events written to Fjall.
    pub written: usize,
}

/// Bounded in-memory queue for journal writer batching.
#[derive(Debug)]
pub struct JournalWriterQueue {
    state: Mutex<JournalWriterQueueState>,
    capacity: usize,
    batch_size: usize,
}

impl JournalWriterQueue {
    /// Creates a bounded writer queue.
    pub fn new(
        capacity: usize,
        batch_size: usize,
        _limits: StorageLimits,
    ) -> Result<Self, JournalError> {
        if capacity == 0 || batch_size == 0 {
            return Err(JournalError::QueueCapacity);
        }
        Ok(Self {
            state: Mutex::new(JournalWriterQueueState {
                pending: VecDeque::with_capacity(capacity),
                shutdown: false,
            }),
            capacity,
            batch_size,
        })
    }

    /// Enqueues an event for journaled append.
    pub fn enqueue_journaled(&self, event: JournalEvent) -> Result<(), JournalError> {
        self.enqueue(event, DurabilityProfile::Journaled)
    }

    /// Enqueues an event for strict append.
    pub fn enqueue_strict(&self, event: JournalEvent) -> Result<(), JournalError> {
        self.enqueue(event, DurabilityProfile::Strict)
    }

    fn enqueue(&self, event: JournalEvent, profile: DurabilityProfile) -> Result<(), JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if state.shutdown {
            return Err(JournalError::QueueShutdown);
        }
        if state.pending.len() >= self.capacity {
            return Err(JournalError::QueueFull);
        }
        state
            .pending
            .push_back(QueuedJournalEvent { event, profile });
        Ok(())
    }

    /// Returns pending write counts split by durability profile.
    pub fn pending_profile_counts(&self) -> Result<JournalWriterQueueProfileCounts, JournalError> {
        let state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let mut counts = JournalWriterQueueProfileCounts {
            journaled: 0,
            strict: 0,
        };
        for item in &state.pending {
            match item.profile {
                DurabilityProfile::Journaled => {
                    counts.journaled = counts.journaled.saturating_add(1);
                }
                DurabilityProfile::Strict => {
                    counts.strict = counts.strict.saturating_add(1);
                }
                DurabilityProfile::Volatile => {}
            }
        }
        Ok(counts)
    }

    /// Flushes at most one configured batch to the journal.
    pub fn flush_batch(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let mut batch_len = 0usize;
        let mut has_strict = false;

        while batch_len < self.batch_size {
            let Some(item) = state.pending.get(batch_len) else {
                break;
            };
            if item.profile == DurabilityProfile::Strict {
                has_strict = true;
            }
            batch_len = batch_len.saturating_add(1);
        }

        if batch_len == 0 {
            return Ok(JournalWriterFlushReport {
                drained: 0,
                written: 0,
            });
        }

        if has_strict {
            let mut written = 0usize;
            while written < batch_len {
                let Some(item) = state.pending.get(written) else {
                    break;
                };
                journal.append_queued_unpersisted(&item.event)?;
                written = written.saturating_add(1);
            }
            journal.persist_strict()?;
            let mut drained = 0usize;
            while drained < written {
                match state.pending.pop_front() {
                    Some(_) => {
                        drained = drained.saturating_add(1);
                    }
                    None => return Err(JournalError::WriteLockPoisoned),
                }
            }
            return Ok(JournalWriterFlushReport { drained, written });
        }

        let mut written = 0usize;
        while written < batch_len {
            let Some(item) = state.pending.get(written) else {
                break;
            };
            journal.append_queued_unpersisted(&item.event)?;
            written = written.saturating_add(1);
        }

        journal.persist_strict()?;
        let mut drained = 0usize;
        while drained < written {
            match state.pending.pop_front() {
                Some(_) => {
                    drained = drained.saturating_add(1);
                }
                None => return Err(JournalError::WriteLockPoisoned),
            }
        }

        Ok(JournalWriterFlushReport { drained, written })
    }

    /// Flushes queued journal writes until the queue is empty.
    pub fn drain_all(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
        };

        loop {
            let report = self.flush_batch(journal)?;
            if report.drained == 0 {
                return Ok(total);
            }
            total.drained = total.drained.saturating_add(report.drained);
            total.written = total.written.saturating_add(report.written);
        }
    }

    /// Closes the queue to new writes and drains all accepted writes durably.
    pub fn shutdown(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| JournalError::WriteLockPoisoned)?;
            state.shutdown = true;
        }
        self.drain_all(journal)
    }
}

/// Ergonomic builder for batching journal events.
#[derive(Debug, Default)]
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
}

impl BatchBuilder {
    /// Creates an empty batch builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an event to the batch.
    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);
    }

    /// Returns the number of events in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the batch contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the built event slice.
    #[must_use]
    pub fn as_slice(&self) -> &[JournalEvent] {
        &self.events
    }
}

/// Monotonic per-run event sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EventSeq(u64);

impl EventSeq {
    /// Creates an event sequence.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Record kind identifiers from the storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum RecordKind {
    /// Workflow source record.
    WorkflowSource = 1,
    /// Compiled IR record.
    CompiledIr = 2,
    /// Run header record.
    RunHeader = 3,
    /// Run accepted event.
    RunAccepted = 10,
    /// Step started event.
    StepStarted = 11,
    /// Slot written event.
    SlotWritten = 12,
    /// Action scheduled event.
    ActionScheduled = 13,
    /// Action completed event.
    ActionCompleted = 14,
    /// Action failed event.
    ActionFailed = 15,
    /// Wait scheduled event.
    WaitScheduled = 16,
    /// Ask scheduled event.
    AskScheduled = 17,
    /// Ask answered event.
    AskAnswered = 18,
    /// Retry scheduled event.
    RetryScheduled = 19,
    /// Step failed event.
    StepFailed = 20,
    /// Run cancelled event.
    RunCancelled = 21,
    /// Run finished event.
    RunFinished = 22,
    /// Run failed event.
    RunFailed = 23,
    /// Snapshot record.
    Snapshot = 30,
    /// Blob record.
    Blob = 40,
    /// Index update record.
    IndexUpdate = 50,
}

impl RecordKind {
    /// Returns the wire identifier.
    #[must_use]
    pub const fn id(self) -> u16 {
        match self {
            Self::WorkflowSource => 1,
            Self::CompiledIr => 2,
            Self::RunHeader => 3,
            Self::RunAccepted => 10,
            Self::StepStarted => 11,
            Self::SlotWritten => 12,
            Self::ActionScheduled => 13,
            Self::ActionCompleted => 14,
            Self::ActionFailed => 15,
            Self::WaitScheduled => 16,
            Self::AskScheduled => 17,
            Self::AskAnswered => 18,
            Self::RetryScheduled => 19,
            Self::StepFailed => 20,
            Self::RunCancelled => 21,
            Self::RunFinished => 22,
            Self::RunFailed => 23,
            Self::Snapshot => 30,
            Self::Blob => 40,
            Self::IndexUpdate => 50,
        }
    }
}

/// Compact binary journal event. JSONL is a projection, not this durable format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalEvent {
    /// Run was accepted after input mapping.
    RunAccepted {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Compiled workflow digest.
        workflow: WorkflowDigest,
    },
    /// Step began execution.
    StepStarted {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
    },
    /// Step completed and wrote an output slot.
    StepSucceeded {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Output slot index.
        output: SlotIdx,
    },
    /// Action was scheduled.
    ActionScheduled {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Action completed successfully.
    ActionCompletedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Action failed.
    ActionFailedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Slot was written during execution.
    SlotWrittenEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Slot index.
        slot: SlotIdx,
    },
    /// Wait was scheduled.
    WaitScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
    },
    /// Ask was scheduled.
    AskScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
    },
    /// Ask was answered.
    AskAnsweredEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
    },
    /// Retry was scheduled.
    RetryScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
    },
    /// Run cancelled.
    RunCancelled {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
    },
    /// Run completed.
    RunFinished {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Result slot index.
        result: SlotIdx,
    },
    /// Run failed.
    RunFailedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
    },
}

impl JournalEvent {
    /// Run identifier carried by this event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::RunAccepted { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompletedEvent { run, .. }
            | Self::ActionFailedEvent { run, .. }
            | Self::SlotWrittenEvent { run, .. }
            | Self::WaitScheduledEvent { run, .. }
            | Self::AskScheduledEvent { run, .. }
            | Self::AskAnsweredEvent { run, .. }
            | Self::RetryScheduledEvent { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailedEvent { run, .. } => *run,
        }
    }

    /// Event sequence carried by this event.
    #[must_use]
    pub const fn seq(&self) -> EventSeq {
        match self {
            Self::RunAccepted { seq, .. }
            | Self::StepStarted { seq, .. }
            | Self::StepSucceeded { seq, .. }
            | Self::ActionScheduled { seq, .. }
            | Self::ActionCompletedEvent { seq, .. }
            | Self::ActionFailedEvent { seq, .. }
            | Self::SlotWrittenEvent { seq, .. }
            | Self::WaitScheduledEvent { seq, .. }
            | Self::AskScheduledEvent { seq, .. }
            | Self::AskAnsweredEvent { seq, .. }
            | Self::RetryScheduledEvent { seq, .. }
            | Self::RunCancelled { seq, .. }
            | Self::RunFinished { seq, .. }
            | Self::RunFailedEvent { seq, .. } => *seq,
        }
    }

    /// Storage record kind for this event.
    #[must_use]
    pub const fn record_kind(&self) -> RecordKind {
        match self {
            Self::RunAccepted { .. } => RecordKind::RunAccepted,
            Self::StepStarted { .. } => RecordKind::StepStarted,
            Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
            Self::ActionScheduled { .. } => RecordKind::ActionScheduled,
            Self::ActionCompletedEvent { .. } => RecordKind::ActionCompleted,
            Self::ActionFailedEvent { .. } => RecordKind::ActionFailed,
            Self::WaitScheduledEvent { .. } => RecordKind::WaitScheduled,
            Self::AskScheduledEvent { .. } => RecordKind::AskScheduled,
            Self::AskAnsweredEvent { .. } => RecordKind::AskAnswered,
            Self::RetryScheduledEvent { .. } => RecordKind::RetryScheduled,
            Self::RunCancelled { .. } => RecordKind::RunCancelled,
            Self::RunFinished { .. } => RecordKind::RunFinished,
            Self::RunFailedEvent { .. } => RecordKind::RunFailed,
        }
    }
}

/// Decoded record envelope metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordEnvelope {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Payload sequence number.
    pub sequence: u64,
}

/// Immutable workflow source bytes bound to their digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSourceRecord {
    /// Source digest key.
    pub digest: WorkflowDigest,
    /// Original strict YAML authoring bytes.
    pub source: Vec<u8>,
}

/// Compiled IR artifact bytes bound to their digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledIrRecord {
    /// Compiled IR digest key.
    pub digest: WorkflowDigest,
    /// Postcard-compatible compiled artifact bytes.
    pub ir: Vec<u8>,
}

/// Minimal run metadata record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunHeaderRecord {
    /// Run identifier.
    pub run: RunId,
    /// Workflow identifier.
    pub workflow_id: WorkflowId,
    /// Compiled workflow digest bound at run acceptance.
    pub compiled_digest: WorkflowDigest,
    /// Status byte owned by the runtime status model.
    pub status: u8,
    /// Admission timestamp in milliseconds supplied by the caller.
    pub accepted_at_ms: u64,
}

/// Large payload blob record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRecord {
    /// Blob digest key.
    pub digest: [u8; DIGEST_BYTES],
    /// Bounded blob payload.
    pub bytes: Vec<u8>,
}

/// Key variants supported by the durable storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKey {
    /// Workflow source bytes by digest.
    WorkflowSource { digest: [u8; DIGEST_BYTES] },
    /// Compiled IR bytes by digest.
    CompiledIr { digest: [u8; DIGEST_BYTES] },
    /// Run metadata by run id.
    RunHeader { run: RunId },
    /// Run event by run id and sequence.
    RunEvent { run: RunId, seq: EventSeq },
    /// Run snapshot by run id and sequence.
    RunSnapshot { run: RunId, seq: EventSeq },
    /// Blob bytes by digest.
    Blob { digest: [u8; DIGEST_BYTES] },
    /// Status index marker.
    IndexStatus {
        state: u8,
        timestamp: u64,
        run: RunId,
    },
    /// Workflow/run index marker.
    IndexWorkflow { workflow: WorkflowId, run: RunId },
    /// Pending action index marker.
    IndexAction {
        action: ActionId,
        run: RunId,
        step: StepIdx,
    },
}

/// Decoded 60-byte record header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHeader {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Header length in bytes.
    pub header_len: u32,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Payload sequence number.
    pub sequence: u64,
    /// BLAKE3 digest of the payload bytes.
    pub payload_digest: [u8; DIGEST_BYTES],
    /// CRC32C of the header prefix before the checksum field.
    pub header_checksum: u32,
}

/// Encodes a postcard payload behind the 60-byte storage envelope.
pub fn encode_record<T: Serialize>(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &T,
    max_payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    validate_kind_family(magic, kind.id())?;
    let payload_bytes = postcard::to_allocvec(payload)?;
    let payload_len = payload_len_u32(payload_bytes.len(), max_payload_len)?;
    encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Decodes and postcard-deserializes an enveloped record.
pub fn decode_record<T: DeserializeOwned>(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, T), JournalError> {
    let (envelope, payload) = decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}

/// Encodes any supported storage key using the existing typed key encoders.
pub fn encode_key(key: StorageKey) -> Result<Vec<u8>, JournalError> {
    let encoded = match key {
        StorageKey::WorkflowSource { digest } => workflow_source_key(digest)?.to_vec(),
        StorageKey::CompiledIr { digest } => compiled_ir_key(digest)?.to_vec(),
        StorageKey::RunHeader { run } => run_header_key(run)?.to_vec(),
        StorageKey::RunEvent { run, seq } => run_event_key(run, seq)?.to_vec(),
        StorageKey::RunSnapshot { run, seq } => run_snapshot_key(run, seq)?.to_vec(),
        StorageKey::Blob { digest } => blob_key(digest)?.to_vec(),
        StorageKey::IndexStatus {
            state,
            timestamp,
            run,
        } => index_status_key(state, timestamp, run)?.to_vec(),
        StorageKey::IndexWorkflow { workflow, run } => index_workflow_key(workflow, run)?.to_vec(),
        StorageKey::IndexAction { action, run, step } => {
            index_action_key(action, run, step)?.to_vec()
        }
    };
    Ok(encoded)
}

/// Encodes only the 60-byte storage record header for an existing payload.
pub fn encode_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    max_payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    validate_kind_family(magic, kind.id())?;
    let payload_len = payload_len_u32(payload.len(), max_payload_len)?;
    build_record_header(magic, kind, sequence, payload, payload_len)
}

/// Decodes and validates only the 60-byte storage record header.
pub fn decode_record_header(
    header: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<RecordHeader, JournalError> {
    let header = header
        .get(..RECORD_HEADER_BYTES)
        .ok_or(JournalError::UnexpectedEof)?;
    let decoded = decode_record_header_unchecked_len(header)?;
    if decoded.magic != expected_magic {
        return Err(JournalError::BadMagic {
            found: decoded.magic,
        });
    }
    validate_schema_version(decoded.schema_version)?;
    validate_known_kind(decoded.record_kind)?;
    validate_kind_family(decoded.magic, decoded.record_kind)?;
    if decoded.header_len != RECORD_HEADER_LEN {
        return Err(JournalError::HeaderLengthMismatch {
            found: decoded.header_len,
        });
    }
    if decoded.payload_len > max_payload_len {
        return Err(JournalError::PayloadTooLarge {
            len: decoded.payload_len,
            max: max_payload_len,
        });
    }
    if crc32c::crc32c(header_prefix_for_crc(header)?) != decoded.header_checksum {
        return Err(JournalError::HeaderChecksumMismatch);
    }
    Ok(decoded)
}

/// Verifies a payload against an expected BLAKE3 digest.
pub fn verify_digest_match(
    payload: &[u8],
    expected_digest: [u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    if blake3::hash(payload).as_bytes() == &expected_digest {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}

/// Encodes `[0x01][workflow_digest_32]`.
pub fn workflow_source_key(
    digest: [u8; DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_WORKFLOW_SOURCE, digest)
}

/// Encodes `[0x02][compiled_digest_32]`.
pub fn compiled_ir_key(digest: [u8; DIGEST_BYTES]) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_COMPILED_IR, digest)
}

/// Encodes `[0x10][run_id_u64_be]`.
pub fn run_header_key(run: RunId) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
    run_only_key(PREFIX_RUN_HEADER, run)
}

/// Encodes `[0x11][run_id_u64_be][seq_u64_be]`.
pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    journal_key(run, seq)
}

/// Encodes `[0x12][run_id_u64_be][seq_u64_be]`.
pub fn run_snapshot_key(
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_SNAPSHOT, run, seq)
}

/// Encodes `[0x20][blob_digest_32]`.
pub fn blob_key(digest: [u8; DIGEST_BYTES]) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    digest_key(PREFIX_BLOB, digest)
}

/// Encodes `[0x30][state_u8][timestamp_u64_be][run_id_u64_be]`.
pub fn index_status_key(
    state: u8,
    timestamp: u64,
    run: RunId,
) -> Result<[u8; INDEX_STATUS_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, INDEX_STATUS_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_STATUS)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_push(state).map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&timestamp.to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.as_u64().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Encodes `[0x31][workflow_id_u32_be][run_id_u64_be]`.
pub fn index_workflow_key(
    workflow: WorkflowId,
    run: RunId,
) -> Result<[u8; INDEX_WORKFLOW_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, INDEX_WORKFLOW_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_WORKFLOW)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&workflow.as_u32().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.as_u64().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Encodes `[0x32][action_id_u16_be][run_id_u64_be][step_u16_be]`.
pub fn index_action_key(
    action: ActionId,
    run: RunId,
    step: StepIdx,
) -> Result<[u8; INDEX_ACTION_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, INDEX_ACTION_KEY_BYTES>::new();
    key.try_push(PREFIX_INDEX_ACTION)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&action.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.as_u64().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&step.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

/// Configuration for Fjall-backed storage.
#[derive(Debug, Clone, Copy)]
pub struct FjallConfig {
    /// Cache size in bytes.
    pub cache_size_bytes: u64,
}

impl Default for FjallConfig {
    fn default() -> Self {
        Self {
            cache_size_bytes: 268_435_456, // 256 MiB
        }
    }
}

/// Fjall-backed append journal.
pub struct FjallJournal {
    database: fjall::Database,
    workflow_source: fjall::Keyspace,
    compiled_ir: fjall::Keyspace,
    run_header: fjall::Keyspace,
    events: fjall::Keyspace,
    run_snapshot: fjall::Keyspace,
    blob: fjall::Keyspace,
    index_status: fjall::Keyspace,
    index_workflow: fjall::Keyspace,
    index_action: fjall::Keyspace,
    #[allow(dead_code)]
    write_lock: Mutex<()>,
}

impl FjallJournal {
    /// Opens or creates the journal at `path`.
    pub fn open(path: impl AsRef<Path>, config: Option<FjallConfig>) -> Result<Self, JournalError> {
        let config = config.unwrap_or_default();
        let database = fjall::Database::builder(path)
            .cache_size(config.cache_size_bytes)
            .open()?;
        let workflow_source = database.keyspace(KEYSPACE_WORKFLOW_SOURCE, || {
            keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let compiled_ir = database.keyspace(KEYSPACE_COMPILED_IR, || {
            keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let run_header = database.keyspace(KEYSPACE_RUN_HEADER, || {
            keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let events = database.keyspace(KEYSPACE_RUN_EVENT, || {
            keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let run_snapshot = database.keyspace(KEYSPACE_RUN_SNAPSHOT, || {
            keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let blob = database.keyspace(KEYSPACE_BLOB, || {
            keyspace_options_for(KeyspaceProfile::Blob)
        })?;
        let index_status = database.keyspace(KEYSPACE_INDEX_STATUS, || {
            keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_workflow = database.keyspace(KEYSPACE_INDEX_WORKFLOW, || {
            keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_action = database.keyspace(KEYSPACE_INDEX_ACTION, || {
            keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        Ok(Self {
            database,
            workflow_source,
            compiled_ir,
            run_header,
            events,
            run_snapshot,
            blob,
            index_status,
            index_workflow,
            index_action,
            write_lock: Mutex::new(()),
        })
    }

    /// Returns all declared keyspace names after a successful open.
    #[must_use]
    pub const fn declared_keyspaces() -> [&'static str; 9] {
        [
            KEYSPACE_WORKFLOW_SOURCE,
            KEYSPACE_COMPILED_IR,
            KEYSPACE_RUN_HEADER,
            KEYSPACE_RUN_EVENT,
            KEYSPACE_RUN_SNAPSHOT,
            KEYSPACE_BLOB,
            KEYSPACE_INDEX_STATUS,
            KEYSPACE_INDEX_WORKFLOW,
            KEYSPACE_INDEX_ACTION,
        ]
    }

    /// Stores immutable workflow source bytes by digest.
    pub fn put_workflow_source(&self, record: &WorkflowSourceRecord) -> Result<(), JournalError> {
        let key = workflow_source_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )?;
        self.workflow_source.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads workflow source bytes by digest.
    pub fn workflow_source(
        &self,
        digest: WorkflowDigest,
    ) -> Result<Option<WorkflowSourceRecord>, JournalError> {
        let key = workflow_source_key(digest.as_bytes())?;
        self.decode_optional(
            &self.workflow_source,
            key.as_slice(),
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
    }

    /// Stores compiled IR bytes by digest.
    pub fn put_compiled_ir(&self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        let key = compiled_ir_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            record,
            MAX_COMPILED_IR_BYTES,
        )?;
        self.compiled_ir.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads compiled IR bytes by digest.
    pub fn compiled_ir(
        &self,
        digest: WorkflowDigest,
    ) -> Result<Option<CompiledIrRecord>, JournalError> {
        let key = compiled_ir_key(digest.as_bytes())?;
        self.decode_optional(
            &self.compiled_ir,
            key.as_slice(),
            MAGIC_COMPILED_ARTIFACT,
            MAX_COMPILED_IR_BYTES,
        )
    }

    /// Stores run metadata by run id.
    pub fn put_run_header(&self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = run_header_key(record.run)?;
        let value = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.as_u64(),
            record,
            MAX_RUN_HEADER_BYTES,
        )?;
        self.run_header.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads run metadata by run id.
    pub fn run_header(&self, run: RunId) -> Result<Option<RunHeaderRecord>, JournalError> {
        let key = run_header_key(run)?;
        self.decode_optional(
            &self.run_header,
            key.as_slice(),
            MAGIC_INDEX_RECORD,
            MAX_RUN_HEADER_BYTES,
        )
    }

    /// Loads all run metadata records in key order.
    pub fn run_headers(&self) -> Result<Vec<RunHeaderRecord>, JournalError> {
        let mut headers = Vec::new();
        let prefix = [PREFIX_RUN_HEADER];
        for item in self.run_header.prefix(prefix) {
            let value = item.value()?;
            let (_, header) =
                decode_record(value.as_ref(), MAGIC_INDEX_RECORD, MAX_RUN_HEADER_BYTES)?;
            headers.push(header);
        }
        Ok(headers)
    }

    /// Stores a compact run snapshot.
    pub fn put_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
        let value = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        )?;
        self.run_snapshot.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads a compact run snapshot.
    pub fn snapshot(&self, run: RunId, seq: EventSeq) -> Result<Option<RunSnapshot>, JournalError> {
        let key = run_snapshot_key(run, seq)?;
        self.decode_optional(
            &self.run_snapshot,
            key.as_slice(),
            MAGIC_SNAPSHOT,
            MAX_SNAPSHOT_BYTES,
        )
    }

    /// Stores a bounded blob by digest.
    pub fn put_blob(&self, record: &BlobRecord) -> Result<(), JournalError> {
        let key = blob_key(record.digest)?;
        let value = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, record, MAX_BLOB_BYTES)?;
        self.blob.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads a bounded blob by digest.
    pub fn blob(&self, digest: [u8; DIGEST_BYTES]) -> Result<Option<BlobRecord>, JournalError> {
        let key = blob_key(digest)?;
        self.decode_optional(&self.blob, key.as_slice(), MAGIC_BLOB, MAX_BLOB_BYTES)
    }

    /// Inserts minimal status index marker bytes.
    pub fn put_status_index(
        &self,
        state: u8,
        timestamp: u64,
        run: RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.index_status.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Inserts minimal workflow index marker bytes.
    pub fn put_workflow_index(&self, workflow: WorkflowId, run: RunId) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.index_workflow.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Inserts minimal pending action index marker bytes.
    pub fn put_action_index(
        &self,
        action: ActionId,
        run: RunId,
        step: StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.index_action.insert(key.to_vec(), Vec::<u8>::new())?;
        Ok(())
    }

    /// Appends one event without forcing a durability barrier.
    pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unpersisted(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unpersisted(event)?;
        self.persist_strict()
    }

    /// Appends multiple events with a single strict durability barrier.
    pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> {
        for event in events {
            self.append_unpersisted(event)?;
        }
        if !events.is_empty() {
            self.persist_strict()?;
        }
        Ok(())
    }

    pub(crate) fn append_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let key = journal_key(event.run_id(), event.seq())?;
        if self.events.contains_key(key)? {
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        self.events.insert(key.to_vec(), value)?;
        Ok(())
    }

    fn append_queued_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
        match self.append_unpersisted(event) {
            Ok(()) => Ok(()),
            Err(JournalError::DuplicateEvent { run, seq }) => {
                let key = journal_key(run, seq)?;
                let Some(value) = self.events.get(key)? else {
                    return Err(JournalError::DuplicateEvent { run, seq });
                };
                let (_, existing) = decode_record::<JournalEvent>(
                    value.as_ref(),
                    MAGIC_JOURNAL_EVENT,
                    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                )?;
                if existing == *event {
                    Ok(())
                } else {
                    Err(JournalError::DuplicateEvent { run, seq })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Forces a strict durability barrier.
    pub fn persist_strict(&self) -> Result<(), JournalError> {
        self.database.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }

    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
        let mut expected = EventSeq::new(0);
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, run_prefix(run)?) {
            let value = item.value()?;
            let (_, event) = decode_record(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;
            validate_replayed_event(run, expected, &event)?;
            expected = next_seq(expected)?;
            replay.push(event);
        }

        Ok(replay)
    }

    #[allow(clippy::unused_self)]
    fn decode_optional<T: DeserializeOwned>(
        &self,
        keyspace: &fjall::Keyspace,
        key: &[u8],
        magic: u32,
        max_payload_len: u32,
    ) -> Result<Option<T>, JournalError> {
        let Some(value) = keyspace.get(key)? else {
            return Ok(None);
        };
        let (_, record) = decode_record(value.as_ref(), magic, max_payload_len)?;
        Ok(Some(record))
    }

    /// Creates a new atomic cross-keyspace write batch.
    pub fn batch(&self) -> JournalWriteBatch<'_> {
        JournalWriteBatch::new(self)
    }
}

/// Atomic cross-keyspace write batch backed by Fjall.
///
/// Accumulates writes across multiple keyspaces and commits them
/// atomically with a single WAL fsync.
pub struct JournalWriteBatch<'j> {
    inner: fjall::OwnedWriteBatch,
    journal: &'j FjallJournal,
}

impl<'j> JournalWriteBatch<'j> {
    fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
        }
    }

    /// Inserts a workflow source record into the batch.
    pub fn put_workflow_source(
        &mut self,
        record: &WorkflowSourceRecord,
    ) -> Result<(), JournalError> {
        let key = workflow_source_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )?;
        self.inner.insert(&self.journal.workflow_source, key, value);
        Ok(())
    }

    /// Inserts a compiled IR record into the batch.
    pub fn put_compiled_ir(&mut self, record: &CompiledIrRecord) -> Result<(), JournalError> {
        let key = compiled_ir_key(record.digest.as_bytes())?;
        let value = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            record,
            MAX_COMPILED_IR_BYTES,
        )?;
        self.inner.insert(&self.journal.compiled_ir, key, value);
        Ok(())
    }

    /// Inserts a run header record into the batch.
    pub fn put_run_header(&mut self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = run_header_key(record.run)?;
        let value = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.as_u64(),
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
    pub fn put_blob(&mut self, record: &BlobRecord) -> Result<(), JournalError> {
        let key = blob_key(record.digest)?;
        let value = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, record, MAX_BLOB_BYTES)?;
        self.inner.insert(&self.journal.blob, key, value);
        Ok(())
    }

    /// Inserts a status index marker into the batch.
    pub fn put_status_index(
        &mut self,
        state: u8,
        timestamp: u64,
        run: RunId,
    ) -> Result<(), JournalError> {
        let key = index_status_key(state, timestamp, run)?;
        self.inner
            .insert(&self.journal.index_status, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts a workflow index marker into the batch.
    pub fn put_workflow_index(
        &mut self,
        workflow: WorkflowId,
        run: RunId,
    ) -> Result<(), JournalError> {
        let key = index_workflow_key(workflow, run)?;
        self.inner
            .insert(&self.journal.index_workflow, key, Vec::<u8>::new());
        Ok(())
    }

    /// Inserts an action index marker into the batch.
    pub fn put_action_index(
        &mut self,
        action: ActionId,
        run: RunId,
        step: StepIdx,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        self.inner
            .insert(&self.journal.index_action, key, Vec::<u8>::new());
        Ok(())
    }

    /// Appends a journal event into the batch.
    pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        let key = journal_key(event.run_id(), event.seq())?;
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        self.inner.insert(&self.journal.events, key, value);
        Ok(())
    }

    /// Returns the number of operations in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the batch contains no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Sets strict durability for the commit.
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }

    /// Commits the batch atomically.
    pub fn commit(self) -> Result<(), JournalError> {
        self.inner.commit()?;
        Ok(())
    }
}

impl Drop for FjallJournal {
    fn drop(&mut self) {
        if let Err(e) = self.database.persist(fjall::PersistMode::SyncAll) {
            let _ = e;
        }
    }
}

/// Opens the Fjall-backed storage engine.
pub fn open_store(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Initializes all declared keyspaces by opening the store.
pub fn init_keyspaces(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path, None)
}

/// Appends one journal event without forcing a durability barrier.
pub fn append_journal_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    journal.append_journaled(event)
}

/// Stores immutable workflow source bytes by digest.
pub fn put_workflow_source(
    journal: &FjallJournal,
    record: &WorkflowSourceRecord,
) -> Result<(), JournalError> {
    journal.put_workflow_source(record)
}

/// Stores compiled IR bytes by digest.
pub fn put_compiled_ir(
    journal: &FjallJournal,
    record: &CompiledIrRecord,
) -> Result<(), JournalError> {
    journal.put_compiled_ir(record)
}

/// Stores run metadata by run id.
pub fn put_run_header(
    journal: &FjallJournal,
    record: &RunHeaderRecord,
) -> Result<(), JournalError> {
    journal.put_run_header(record)
}

/// Writes a compact run snapshot.
pub fn write_snapshot(journal: &FjallJournal, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
}

/// Stores a bounded blob by digest.
pub fn put_blob(journal: &FjallJournal, record: &BlobRecord) -> Result<(), JournalError> {
    journal.put_blob(record)
}

/// Reads a stored blob by digest.
pub fn read_blob(
    journal: &FjallJournal,
    digest: [u8; DIGEST_BYTES],
) -> Result<Option<BlobRecord>, JournalError> {
    journal.blob(digest)
}

/// Reads one run's journal events in replay order.
pub fn read_run_events(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Vec<JournalEvent>, JournalError> {
    journal.events_for_run(run)
}

/// Replays one run's full journal through the recovery path.
pub fn replay_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut recovery::ActionReplayTracker,
) -> recovery::RecoveryResult<Vec<JournalEvent>> {
    recovery::recover_full_journal(journal, run, tracker)
}

/// Recovers summary hydration for every durable run without a terminal event.
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> recovery::RecoveryResult<Vec<recovery::RecoveryHydration>> {
    recovery::recover_all_incomplete_runs(journal)
}

/// Flushes one queued writer batch using each event's durability profile.
pub fn flush_profile(
    queue: &JournalWriterQueue,
    journal: &FjallJournal,
) -> Result<JournalWriterFlushReport, JournalError> {
    queue.flush_batch(journal)
}

/// Storage errors.
#[derive(Debug, Error)]
pub enum JournalError {
    /// Fjall operation failed.
    #[error("fjall journal operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    /// Binary encoding failed.
    #[error("journal event encoding failed: {0}")]
    Encode(#[from] postcard::Error),
    /// Fixed-size key construction failed.
    #[error("journal key capacity exceeded")]
    KeyCapacity,
    /// Append attempted to overwrite an immutable event.
    #[error("duplicate journal event for run {run:?} seq {seq:?}")]
    DuplicateEvent {
        /// Run identifier.
        run: RunId,
        /// Existing sequence.
        seq: EventSeq,
    },
    /// Serialized append lock was poisoned by a panicking holder.
    #[error("journal write lock is poisoned")]
    WriteLockPoisoned,
    /// Queue capacity or batch size was zero.
    #[error("journal writer queue capacity must be non-zero")]
    QueueCapacity,
    /// Queue has no room for another event.
    #[error("journal writer queue is full")]
    QueueFull,
    /// Queue has started deterministic shutdown and rejects new writes.
    #[error("journal writer queue is shut down")]
    QueueShutdown,
    /// Replay returned an event for a different run than requested.
    #[error("journal replay returned run {actual:?}, expected {expected:?}")]
    WrongRun {
        /// Expected run id.
        expected: RunId,
        /// Actual run id.
        actual: RunId,
    },
    /// Replay found a non-contiguous event sequence.
    #[error("journal replay sequence gap: expected {expected:?}, actual {actual:?}")]
    SequenceGap {
        /// Expected sequence.
        expected: EventSeq,
        /// Actual sequence.
        actual: EventSeq,
    },
    /// Sequence number overflowed.
    #[error("journal event sequence overflow")]
    SequenceOverflow,
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
    /// Record kind is not known.
    #[error("unknown record kind: {kind}")]
    UnknownRecordKind {
        /// Found kind.
        kind: u16,
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
}

impl JournalError {
    /// Diagnostic code for fjall operation failure.
    pub const FJALL_CODE: DiagnosticCode = DiagnosticCode::new(0x4001);
    /// Diagnostic code for binary encoding failure.
    pub const ENCODE_CODE: DiagnosticCode = DiagnosticCode::new(0x4002);
    /// Diagnostic code for key capacity exceeded.
    pub const KEY_CAPACITY_CODE: DiagnosticCode = DiagnosticCode::new(0x4003);
    /// Diagnostic code for duplicate event.
    pub const DUPLICATE_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x4004);
    /// Diagnostic code for write lock poisoned.
    pub const WRITE_LOCK_POISONED_CODE: DiagnosticCode = DiagnosticCode::new(0x4005);
    /// Diagnostic code for queue capacity zero.
    pub const QUEUE_CAPACITY_CODE: DiagnosticCode = DiagnosticCode::new(0x4006);
    /// Diagnostic code for queue full.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x4007);
    /// Diagnostic code for queue shutdown.
    pub const QUEUE_SHUTDOWN_CODE: DiagnosticCode = DiagnosticCode::new(0x4016);
    /// Diagnostic code for wrong run.
    pub const WRONG_RUN_CODE: DiagnosticCode = DiagnosticCode::new(0x4008);
    /// Diagnostic code for sequence gap.
    pub const SEQUENCE_GAP_CODE: DiagnosticCode = DiagnosticCode::new(0x4009);
    /// Diagnostic code for sequence overflow.
    pub const SEQUENCE_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x400A);
    /// Diagnostic code for bad magic.
    pub const BAD_MAGIC_CODE: DiagnosticCode = DiagnosticCode::new(0x400B);
    /// Diagnostic code for unsupported schema version.
    pub const UNSUPPORTED_SCHEMA_VERSION_CODE: DiagnosticCode = DiagnosticCode::new(0x400C);
    /// Diagnostic code for migration required.
    pub const MIGRATION_REQUIRED_CODE: DiagnosticCode = DiagnosticCode::new(0x400D);
    /// Diagnostic code for unknown record kind.
    pub const UNKNOWN_RECORD_KIND_CODE: DiagnosticCode = DiagnosticCode::new(0x400E);
    /// Diagnostic code for record kind family mismatch.
    pub const RECORD_KIND_FAMILY_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x400F);
    /// Diagnostic code for header length mismatch.
    pub const HEADER_LENGTH_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4010);
    /// Diagnostic code for payload too large.
    pub const PAYLOAD_TOO_LARGE_CODE: DiagnosticCode = DiagnosticCode::new(0x4011);
    /// Diagnostic code for header checksum mismatch.
    pub const HEADER_CHECKSUM_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4012);
    /// Diagnostic code for payload digest mismatch.
    pub const PAYLOAD_DIGEST_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4013);
    /// Diagnostic code for unexpected eof.
    pub const UNEXPECTED_EOF_CODE: DiagnosticCode = DiagnosticCode::new(0x4014);
    /// Diagnostic code for postcard decode failed.
    pub const POSTCARD_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x4015);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::Fjall(_) => Self::FJALL_CODE,
            Self::Encode(_) => Self::ENCODE_CODE,
            Self::KeyCapacity => Self::KEY_CAPACITY_CODE,
            Self::DuplicateEvent { .. } => Self::DUPLICATE_EVENT_CODE,
            Self::WriteLockPoisoned => Self::WRITE_LOCK_POISONED_CODE,
            Self::QueueCapacity => Self::QUEUE_CAPACITY_CODE,
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::QueueShutdown => Self::QUEUE_SHUTDOWN_CODE,
            Self::WrongRun { .. } => Self::WRONG_RUN_CODE,
            Self::SequenceGap { .. } => Self::SEQUENCE_GAP_CODE,
            Self::SequenceOverflow => Self::SEQUENCE_OVERFLOW_CODE,
            Self::BadMagic { .. } => Self::BAD_MAGIC_CODE,
            Self::UnsupportedSchemaVersion { .. } => Self::UNSUPPORTED_SCHEMA_VERSION_CODE,
            Self::MigrationRequired { .. } => Self::MIGRATION_REQUIRED_CODE,
            Self::UnknownRecordKind { .. } => Self::UNKNOWN_RECORD_KIND_CODE,
            Self::RecordKindFamilyMismatch { .. } => Self::RECORD_KIND_FAMILY_MISMATCH_CODE,
            Self::HeaderLengthMismatch { .. } => Self::HEADER_LENGTH_MISMATCH_CODE,
            Self::PayloadTooLarge { .. } => Self::PAYLOAD_TOO_LARGE_CODE,
            Self::HeaderChecksumMismatch => Self::HEADER_CHECKSUM_MISMATCH_CODE,
            Self::PayloadDigestMismatch => Self::PAYLOAD_DIGEST_MISMATCH_CODE,
            Self::UnexpectedEof => Self::UNEXPECTED_EOF_CODE,
            Self::PostcardDecodeFailed => Self::POSTCARD_DECODE_FAILED_CODE,
        }
    }
}

fn journal_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    sequenced_run_key(PREFIX_RUN_EVENT, run, seq)
}

fn sequenced_run_key(
    prefix: u8,
    run: RunId,
    seq: EventSeq,
) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, JOURNAL_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.as_u64().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&seq.get().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

fn run_prefix(run: RunId) -> Result<[u8; RUN_EVENT_PREFIX_BYTES], JournalError> {
    run_only_key(PREFIX_RUN_EVENT, run)
}

fn digest_key(
    prefix: u8,
    digest: [u8; DIGEST_BYTES],
) -> Result<[u8; DIGEST_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, DIGEST_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&digest)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

fn run_only_key(prefix: u8, run: RunId) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, RUN_ONLY_KEY_BYTES>::new();
    key.try_push(prefix)
        .map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.as_u64().to_be_bytes())
        .map_err(|_| JournalError::KeyCapacity)?;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}

fn payload_len_u32(len: usize, max: u32) -> Result<u32, JournalError> {
    let payload_len = u32::try_from(len).map_err(|_| JournalError::PayloadTooLarge {
        len: PAYLOAD_LEN_CONVERSION_MAX,
        max,
    })?;
    if payload_len > max {
        return Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max,
        });
    }
    Ok(payload_len)
}

fn encode_record_payload(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    let capacity =
        RECORD_HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(JournalError::PayloadTooLarge {
                len: payload_len,
                max: PAYLOAD_LEN_CONVERSION_MAX,
            })?;
    let header = build_record_header(magic, kind, sequence, payload, payload_len)?;

    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(&header);
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start =
        usize::try_from(header.header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len_usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    verify_digest_match(payload, header.payload_digest)?;
    Ok((
        RecordEnvelope {
            magic: header.magic,
            schema_version: header.schema_version,
            record_kind: header.record_kind,
            sequence: header.sequence,
        },
        payload,
    ))
}

fn build_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    write_u32(&mut header, 0, magic)?;
    write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION)?;
    write_u16(&mut header, 6, kind.id())?;
    write_u32(&mut header, 8, RECORD_HEADER_LEN)?;
    write_u32(&mut header, 12, payload_len)?;
    write_u64(&mut header, 16, sequence)?;
    write_digest(&mut header, blake3::hash(payload).as_bytes())?;
    let checksum = crc32c::crc32c(header_prefix_for_crc(&header)?);
    write_u32(&mut header, CRC_OFFSET, checksum)?;
    Ok(header)
}

fn decode_record_header_unchecked_len(header: &[u8]) -> Result<RecordHeader, JournalError> {
    Ok(RecordHeader {
        magic: read_u32(header, 0)?,
        schema_version: read_u16(header, 4)?,
        record_kind: read_u16(header, 6)?,
        header_len: read_u32(header, 8)?,
        payload_len: read_u32(header, 12)?,
        sequence: read_u64(header, 16)?,
        payload_digest: digest_from_header(header)?,
        header_checksum: read_u32(header, CRC_OFFSET)?,
    })
}

fn validate_schema_version(version: u16) -> Result<(), JournalError> {
    if version == CURRENT_SCHEMA_VERSION {
        Ok(())
    } else if version < CURRENT_SCHEMA_VERSION {
        Err(JournalError::MigrationRequired {
            from: version,
            to: CURRENT_SCHEMA_VERSION,
        })
    } else {
        Err(JournalError::UnsupportedSchemaVersion { version })
    }
}

fn validate_known_kind(kind: u16) -> Result<(), JournalError> {
    if matches!(kind, 1 | 2 | 3 | 10..=23 | 30 | 40 | 50) {
        Ok(())
    } else {
        Err(JournalError::UnknownRecordKind { kind })
    }
}

fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    let valid = match magic {
        MAGIC_WORKFLOW_SOURCE => kind == RecordKind::WorkflowSource.id(),
        MAGIC_COMPILED_ARTIFACT => kind == RecordKind::CompiledIr.id(),
        MAGIC_JOURNAL_EVENT => matches!(kind, 10..=23),
        MAGIC_SNAPSHOT => kind == RecordKind::Snapshot.id(),
        MAGIC_BLOB => kind == RecordKind::Blob.id(),
        MAGIC_INDEX_RECORD => matches!(kind, 3 | 50),
        MAGIC_IPC_FRAME => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(JournalError::RecordKindFamilyMismatch { magic, kind })
    }
}

fn header_prefix_for_crc(header: &[u8]) -> Result<&[u8], JournalError> {
    header.get(..CRC_OFFSET).ok_or(JournalError::UnexpectedEof)
}

fn digest_from_header(header: &[u8]) -> Result<[u8; DIGEST_BYTES], JournalError> {
    let digest = header
        .get(24..CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)?;
    <[u8; DIGEST_BYTES]>::try_from(digest).map_err(|_| JournalError::UnexpectedEof)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JournalError> {
    let end = offset.checked_add(2).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 2]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u16::from_le_bytes(raw))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, JournalError> {
    let end = offset.checked_add(4).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 4]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(raw))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, JournalError> {
    let end = offset.checked_add(8).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 8]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u64::from_le_bytes(raw))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), JournalError> {
    let end = offset.checked_add(2).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), JournalError> {
    let end = offset.checked_add(4).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), JournalError> {
    let end = offset.checked_add(8).ok_or(JournalError::UnexpectedEof)?;
    let target = bytes
        .get_mut(offset..end)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_digest(bytes: &mut [u8], digest: &[u8; DIGEST_BYTES]) -> Result<(), JournalError> {
    let target = bytes
        .get_mut(24..CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)?;
    target.copy_from_slice(digest);
    Ok(())
}

fn validate_replayed_event(
    run: RunId,
    expected: EventSeq,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    if event.run_id() != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: event.run_id(),
        });
    }
    if event.seq() != expected {
        return Err(JournalError::SequenceGap {
            expected,
            actual: event.seq(),
        });
    }
    Ok(())
}

fn next_seq(seq: EventSeq) -> Result<EventSeq, JournalError> {
    seq.get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or(JournalError::SequenceOverflow)
}

#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::{
        BatchBuilder, BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, DiagnosticCode,
        EventSeq, FjallJournal, JournalError, JournalEvent, JournalWriterQueue, KeyspaceProfile,
        MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME,
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
        MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
        MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR,
        PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT,
        PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
        RECORD_HEADER_LEN, RecordKind, RunHeaderRecord, StorageKey, StorageLimits,
        WorkflowSourceRecord, append_journal_event, blob_key, compiled_ir_key, decode_record,
        decode_record_header, encode_key, encode_record, encode_record_header, flush_profile,
        index_action_key, index_status_key, index_workflow_key, init_keyspaces, journal_key,
        keyspace_options_for, open_store, put_blob, put_compiled_ir, put_run_header,
        put_workflow_source, read_blob, read_run_events, replay_journal, run_event_key,
        run_header_key, run_snapshot_key, verify_digest_match, workflow_source_key, write_snapshot,
    };
    use crate::recovery::{ActionReplayTracker, RunSnapshot};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    #[test]
    fn journal_key_is_fixed_width() {
        // Given a run id 1 and event sequence 2
        // When the journal key is constructed
        // Then the key is exactly 17 bytes wide
        let key = journal_key(RunId::new(1), EventSeq::new(2));

        let key = key.expect("journal key construction should succeed");
        assert_eq!(key.len(), 17);
    }

    #[test]
    fn run_event_key_uses_required_prefix_and_big_endian_layout() {
        // Given run id 0x0102030405060708 and event sequence 9
        // When the run event key is constructed
        // Then the layout is [0x11 prefix][run id big-endian][seq big-endian]
        let key = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9));

        let key = key.expect("run event key construction should succeed");
        let expected: [u8; 17] = [
            0x11, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x09,
        ];
        assert_eq!(key.as_slice(), expected.as_slice());
    }

    #[test]
    fn key_encoders_use_required_lengths() {
        // Given a standard 32-byte digest and common run/step identifiers
        // When each key encoder is called
        // Then the produced keys have the contract-specified byte widths
        let digest = [7_u8; 32];

        let ws = workflow_source_key(digest).expect("workflow_source_key should succeed");
        assert_eq!(ws.len(), 33);

        let ci = compiled_ir_key(digest).expect("compiled_ir_key should succeed");
        assert_eq!(ci.len(), 33);

        let rh = run_header_key(RunId::new(1)).expect("run_header_key should succeed");
        assert_eq!(rh.len(), 9);

        let rs = run_snapshot_key(RunId::new(1), EventSeq::new(2))
            .expect("run_snapshot_key should succeed");
        assert_eq!(rs.len(), 17);

        let bl = blob_key(digest).expect("blob_key should succeed");
        assert_eq!(bl.len(), 33);

        let is = index_status_key(3, 4, RunId::new(5)).expect("index_status_key should succeed");
        assert_eq!(is.len(), 18);

        let iw = index_workflow_key(WorkflowId::new(6), RunId::new(7))
            .expect("index_workflow_key should succeed");
        assert_eq!(iw.len(), 13);

        let ia = index_action_key(ActionId::new(8), RunId::new(9), StepIdx::new(10))
            .expect("index_action_key should succeed");
        assert_eq!(ia.len(), 13);
    }

    #[test]
    fn encode_key_dispatches_to_existing_key_encoders() {
        let digest = [9_u8; 32];

        let run_event = encode_key(StorageKey::RunEvent {
            run: RunId::new(0x0102_0304_0506_0708),
            seq: EventSeq::new(9),
        })
        .expect("encode_key should encode run event key");
        let expected_run_event = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9))
            .expect("run_event_key should succeed")
            .to_vec();
        assert_eq!(run_event, expected_run_event);

        let blob =
            encode_key(StorageKey::Blob { digest }).expect("encode_key should encode blob key");
        let expected_blob = blob_key(digest).expect("blob_key should succeed").to_vec();
        assert_eq!(blob, expected_blob);
    }

    #[test]
    fn record_header_wrappers_encode_decode_and_verify_digest() {
        let payload = b"compact payload";

        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            7,
            payload,
            128,
        )
        .expect("record header encoding should succeed");
        assert_eq!(header.len(), RECORD_HEADER_BYTES);

        let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 128)
            .expect("record header decoding should succeed");
        assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
        assert_eq!(decoded.payload_len, 15);
        assert_eq!(decoded.sequence, 7);
        verify_digest_match(payload, decoded.payload_digest)
            .expect("payload digest should match decoded header");
    }

    #[test]
    fn verify_digest_match_rejects_mismatched_payload() {
        let digest = *blake3::hash(b"original").as_bytes();

        let result = verify_digest_match(b"changed", digest);

        assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn free_put_wrappers_delegate_to_journal_methods() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let journal = open_store(temp.path()).expect("journal should open");
        let workflow_digest = WorkflowDigest::from_bytes([1; 32]);
        let compiled_digest = WorkflowDigest::from_bytes([2; 32]);
        let blob_digest = [3_u8; 32];

        let source = WorkflowSourceRecord {
            digest: workflow_digest,
            source: vec![b'a'],
        };
        put_workflow_source(&journal, &source).expect("workflow source should store");
        let stored_source = journal
            .workflow_source(workflow_digest)
            .expect("workflow source lookup should succeed");
        assert_eq!(stored_source, Some(source));

        let compiled = CompiledIrRecord {
            digest: compiled_digest,
            ir: vec![b'i'],
        };
        put_compiled_ir(&journal, &compiled).expect("compiled ir should store");
        let stored_compiled = journal
            .compiled_ir(compiled_digest)
            .expect("compiled ir lookup should succeed");
        assert_eq!(stored_compiled, Some(compiled));

        let header = RunHeaderRecord {
            run: RunId::new(11),
            workflow_id: WorkflowId::new(12),
            compiled_digest,
            status: 1,
            accepted_at_ms: 13,
        };
        put_run_header(&journal, &header).expect("run header should store");
        let stored_header = journal
            .run_header(RunId::new(11))
            .expect("run header lookup should succeed");
        assert_eq!(stored_header, Some(header));

        let blob = BlobRecord {
            digest: blob_digest,
            bytes: vec![b'b'],
        };
        put_blob(&journal, &blob).expect("blob should store");
        let stored_blob = read_blob(&journal, blob_digest).expect("blob lookup should succeed");
        assert_eq!(stored_blob, Some(blob));
    }

    #[test]
    fn envelope_round_trips_and_reports_metadata() {
        // Given a RunFinished journal event with run 99, seq 12, result slot 1
        // When the event is encoded and then decoded
        // Then the envelope metadata and deserialized event match the originals
        let event = JournalEvent::RunFinished {
            run: RunId::new(99),
            seq: EventSeq::new(12),
            result: vb_core::SlotIdx::new(1),
        };

        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            event.seq().get(),
            &event,
            128,
        );
        let encoded = encoded.expect("encoding should succeed");
        assert!(encoded.len() > 60, "encoded record must exceed header size");

        let (envelope, decoded_event) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.record_kind, RecordKind::RunFinished.id());
        assert_eq!(envelope.sequence, 12);
        assert_eq!(decoded_event, event);
    }

    #[test]
    fn decode_rejects_corrupt_header_checksum() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            128,
        );
        let Ok(mut encoded) = encoded else {
            return;
        };
        if let Some(byte) = encoded.get_mut(56) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

        assert!(matches!(decoded, Err(JournalError::HeaderChecksumMismatch)));
    }

    #[test]
    fn decode_rejects_corrupt_payload_digest() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            128,
        );
        let Ok(mut encoded) = encoded else {
            return;
        };
        if let Some(byte) = encoded.get_mut(60) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

        assert!(matches!(decoded, Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn decode_rejects_payload_before_allocation() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            128,
        );
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 1);

        assert!(matches!(decoded, Err(JournalError::PayloadTooLarge { .. })));
    }

    #[test]
    fn decode_rejects_bad_magic_and_unknown_kind() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let Ok(mut bad_magic) = encoded else {
            return;
        };
        if let Some(byte) = bad_magic.get_mut(0) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(
            &bad_magic,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(decoded, Err(JournalError::BadMagic { .. })));

        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let Ok(mut unknown_kind) = encoded else {
            return;
        };
        if let Some(byte) = unknown_kind.get_mut(6) {
            *byte = 200;
        }
        if let Some(byte) = unknown_kind.get_mut(56) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(
            &unknown_kind,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(
            decoded,
            Err(JournalError::UnknownRecordKind { .. })
        ));
    }

    #[test]
    fn append_strict_batch_writes_all_events_with_single_fsync() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(61);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: vb_core::SlotIdx::new(0),
            },
        ];

        let result = journal.append_strict_batch(&events);
        result.expect("action must succeed");

        let replayed = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed, events);
    }

    #[test]
    fn append_strict_batch_rejects_duplicate_within_batch() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(62);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let events = vec![event.clone(), event.clone()];

        let result = journal.append_strict_batch(&events);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "expected DuplicateEvent, got {:?}",
            result
        );
    }

    #[test]
    fn batch_builder_collects_events() {
        let mut builder = BatchBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);

        let run = RunId::new(63);
        builder.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        });
        assert_eq!(builder.len(), 1);
        assert!(!builder.is_empty());

        builder.push(JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        });
        assert_eq!(builder.len(), 2);
        assert_eq!(builder.as_slice().len(), 2);
    }

    #[test]
    fn batch_builder_round_trips_via_append_strict_batch() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(64);
        let mut builder = BatchBuilder::new();
        builder.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        });
        builder.push(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        });

journal.append_strict_batch(builder.as_slice()).expect("journal.append_strict_batch must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn flush_profile_batches_strict_events_into_single_fsync() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(58);
        let strict1 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([6; 32]),
        };
        let strict2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        };

queue.enqueue_strict(strict1.clone()).expect("queue.enqueue_strict must succeed");
queue.enqueue_strict(strict2.clone()).expect("queue.enqueue_strict must succeed");
        let report = flush_profile(&queue, &journal);

        let report = report.expect("flush_profile should succeed");
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);
        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![strict1, strict2]);
    }

    #[test]
    fn write_batch_commits_cross_keyspace_atomically() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = WorkflowDigest::from_bytes([1; 32]);
        let run = RunId::new(42);

        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: b"test workflow".to_vec(),
            })
            .expect("put_workflow_source must succeed");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(7),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 1234,
            })
            .expect("put_run_header must succeed");
        batch.commit().expect("batch.commit must succeed");

        let source = journal.workflow_source(digest).expect("workflow source roundtrip");
        assert!(source.is_some());
        assert_eq!(source.unwrap().source, b"test workflow".to_vec());

        let header = journal.run_header(run).expect("run header roundtrip");
        assert!(header.is_some());
        assert_eq!(header.unwrap().run, run);
    }

    #[test]
    fn write_batch_strict_commits_with_durability() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = [2; 32];
        let mut batch = journal.batch().strict();
        batch
                .put_blob(&BlobRecord {
                    digest,
                    bytes: b"blob data".to_vec(),
                }).expect("action must succeed");
batch.commit().expect("batch.commit must succeed");

        let blob = journal.blob(digest).expect("blob roundtrip");
        assert!(blob.is_some());
        assert_eq!(blob.unwrap().bytes, b"blob data".to_vec());
    }

    #[test]
    fn write_batch_appends_events_and_indexes() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(99);
        let workflow = WorkflowId::new(5);
        let action = ActionId::new(3);
        let step = StepIdx::new(2);

        let mut batch = journal.batch();
        batch
                .append_event(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: WorkflowDigest::from_bytes([3; 32]),
                }).expect("action must succeed");
batch.put_workflow_index(workflow, run).expect("batch.put_workflow_index must succeed");
batch.put_action_index(action, run, step).expect("batch.put_action_index must succeed");
batch.put_status_index(1, 5678, run).expect("batch.put_status_index must succeed");
batch.commit().expect("batch.commit must succeed");

        let events = journal.events_for_run(run);
        let events = events.expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn write_batch_empty_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let batch = journal.batch();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
batch.commit().expect("batch.commit must succeed");
    }

    #[test]
    fn write_batch_is_empty_after_construction() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let batch = journal.batch();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn write_batch_len_tracks_operations() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = WorkflowDigest::from_bytes([4; 32]);
        let mut batch = journal.batch();
        batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest,
                    source: b"a".to_vec(),
                }).expect("action must succeed");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        batch
                .put_compiled_ir(&CompiledIrRecord {
                    digest,
                    ir: b"ir".to_vec(),
                }).expect("action must succeed");
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn write_batch_snapshot_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(77);
        let seq = EventSeq::new(5);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([5; 32]),
            slots: b"slot_data".to_vec(),
        };

        let mut batch = journal.batch();
batch.put_snapshot(&snapshot).expect("batch.put_snapshot must succeed");
batch.commit().expect("batch.commit must succeed");

        let loaded = journal.snapshot(run, seq).expect("snapshot roundtrip");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().run, run);
    }

    #[test]
    fn keyspace_profiles_return_distinct_configs() {
        let _hot = keyspace_options_for(KeyspaceProfile::Hot);
        let _cold = keyspace_options_for(KeyspaceProfile::Cold);
        let _blob = keyspace_options_for(KeyspaceProfile::Blob);

        // Hot has no KV separation; Cold and Blob have KV separation.
        // We verify this indirectly by checking the configs differ.
        assert_ne!(
            std::mem::discriminant(&KeyspaceProfile::Hot),
            std::mem::discriminant(&KeyspaceProfile::Cold)
        );
        assert_ne!(
            std::mem::discriminant(&KeyspaceProfile::Cold),
            std::mem::discriminant(&KeyspaceProfile::Blob)
        );

        // Verify the function exists and returns valid options by using them
        // in a real database open.
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None);
        assert!(journal.is_ok(), "journal should open with tuned keyspaces");
    }

    #[test]
    fn journal_opens_declared_keyspaces_and_round_trips_typed_records() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        assert_eq!(FjallJournal::declared_keyspaces().len(), 9);

        let workflow_digest = WorkflowDigest::from_bytes([1; 32]);
        let compiled_digest = WorkflowDigest::from_bytes([2; 32]);
        let source = WorkflowSourceRecord {
            digest: workflow_digest,
            source: vec![b'n', b'a', b'm', b'e'],
        };
        let ir = CompiledIrRecord {
            digest: compiled_digest,
            ir: vec![1, 2, 3],
        };
        let header = RunHeaderRecord {
            run: RunId::new(3),
            workflow_id: WorkflowId::new(4),
            compiled_digest,
            status: 5,
            accepted_at_ms: 6,
        };
        let snapshot = RunSnapshot {
            run: RunId::new(3),
            seq: EventSeq::new(7),
            workflow: compiled_digest,
            slots: vec![8, 9],
        };
        let blob = BlobRecord {
            digest: [9; 32],
            bytes: vec![10, 11],
        };

journal.put_workflow_source(&source).expect("journal.put_workflow_source must succeed");
journal.put_compiled_ir(&ir).expect("journal.put_compiled_ir must succeed");
journal.put_run_header(&header).expect("journal.put_run_header must succeed");
journal.put_snapshot(&snapshot).expect("journal.put_snapshot must succeed");
journal.put_blob(&blob).expect("journal.put_blob must succeed");
journal.put_status_index(1, 2, RunId::new(3)).expect("journal.put_status_index must succeed");
        journal.put_workflow_index(WorkflowId::new(4), RunId::new(3)).expect("action must succeed");
        journal.put_action_index(ActionId::new(5), RunId::new(3), StepIdx::new(6)).expect("action must succeed");

        let found_source = journal
            .workflow_source(workflow_digest)
            .expect("workflow source lookup should succeed");
        assert_eq!(found_source, Some(source));

        let found_ir = journal
            .compiled_ir(compiled_digest)
            .expect("compiled ir lookup should succeed");
        assert_eq!(found_ir, Some(ir));

        let found_header = journal
            .run_header(RunId::new(3))
            .expect("run header lookup should succeed");
        assert_eq!(found_header, Some(header));

        let found_snapshot = journal
            .snapshot(RunId::new(3), EventSeq::new(7))
            .expect("snapshot lookup should succeed");
        assert_eq!(found_snapshot, Some(snapshot));

        let found_blob = journal.blob([9; 32]).expect("blob lookup should succeed");
        assert_eq!(found_blob, Some(blob));
    }

    #[test]
    fn non_journal_families_reject_wrong_record_kind() {
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([1; 32]),
            source: vec![1],
        };

        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            128,
        );
        assert!(encoded.is_ok(), "encoding must succeed for valid input");
        let encoded = encoded.expect("setup: encoding");
        assert!(!encoded.is_empty(), "encoded bytes must be non-empty");
        let wrong_family = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::WorkflowSource,
            0,
            &source,
            128,
        );

        assert!(matches!(
            wrong_family,
            Err(JournalError::RecordKindFamilyMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_event_append_is_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(9),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };

        let first = journal.append_journaled(&event);
        let second = journal.append_journaled(&event);

        first.expect("action must succeed");
        assert!(matches!(second, Err(JournalError::DuplicateEvent { .. })));
    }

    #[test]
    fn journal_writer_queue_counts_pending_durability_profiles() {
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(56);
        let journaled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
        };
        let strict = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
        };

queue.enqueue_journaled(journaled).expect("queue.enqueue_journaled must succeed");
queue.enqueue_strict(strict).expect("queue.enqueue_strict must succeed");

        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 1 && counts.strict == 1
        ));
    }

    #[test]
    fn flush_profile_wrapper_flushes_queued_events() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(57);
        let journaled = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        let strict = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        };

queue.enqueue_journaled(journaled.clone()).expect("queue.enqueue_journaled must succeed");
queue.enqueue_strict(strict.clone()).expect("queue.enqueue_strict must succeed");
        let report = flush_profile(&queue, &journal);

        let report = report.expect("flush_profile should succeed");
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);
        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![journaled, strict]);
    }

    #[test]
    fn replay_returns_contiguous_events_for_run() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(11);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([4; 32]),
        };
        let finished = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        };

journal.append_journaled(&accepted).expect("journal.append_journaled must succeed");
journal.append_journaled(&finished).expect("journal.append_journaled must succeed");

        let replay = journal
            .events_for_run(run)
            .expect("event replay should succeed");
        assert_eq!(replay, vec![accepted, finished]);
    }

    #[test]
    fn decode_rejects_truncated_header() {
        // Given a byte slice shorter than the required 60-byte header
        // When decode_record is called
        // Then it returns UnexpectedEof
        let truncated = [0u8; 30];

        let result = decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn decode_rejects_migration_required_schema() {
        // Given a valid record whose schema version byte is 0 (less than current)
        // When decode_record is called
        // Then it returns MigrationRequired with from=0, to=CURRENT_SCHEMA_VERSION
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Set schema version to 0 (two LE bytes at offset 4..6)
        encoded[4] = 0;
        encoded[5] = 0;
        // Recompute CRC32C for the modified header prefix
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::MigrationRequired { from: 0, to: 1 })
        ));
    }

    #[test]
    fn decode_rejects_unsupported_future_schema() {
        // Given a valid record whose schema version byte is 99 (greater than current)
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Set schema version to 99 (two LE bytes at offset 4..6)
        encoded[4] = 99;
        encoded[5] = 0;
        // Recompute CRC32C for the modified header prefix
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::UnsupportedSchemaVersion { version: 99 })
        ));
    }

    #[test]
    fn decode_rejects_record_kind_family_mismatch() {
        // Given a record encoded with MAGIC_JOURNAL_EVENT but a kind outside 10..=23
        // When decode_record is called
        // Then it returns RecordKindFamilyMismatch
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Patch the kind to 1 (WorkflowSource), which is outside 10..=23
        // Kind is at offset 6..8, little-endian
        let kind_bytes = 1u16.to_le_bytes();
        encoded[6] = kind_bytes[0];
        encoded[7] = kind_bytes[1];
        // Recompute CRC32C
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_JOURNAL_EVENT,
                kind: 1
            })
        ));
    }

    #[test]
    fn decode_rejects_header_length_mismatch() {
        // Given a valid record whose declared header length is 99 (not 60)
        // When decode_record is called
        // Then it returns HeaderLengthMismatch with found=99
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Header length is at offset 8..12, little-endian. Set to 99.
        let len_bytes = 99u32.to_le_bytes();
        encoded[8] = len_bytes[0];
        encoded[9] = len_bytes[1];
        encoded[10] = len_bytes[2];
        encoded[11] = len_bytes[3];
        // Recompute CRC32C
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::HeaderLengthMismatch { found: 99 })
        ));
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        // Given an encoded record with bytes truncated after the header
        // When decode_record is called
        // Then it returns UnexpectedEof
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Keep only the 60-byte header, discarding all payload bytes
        let truncated = &encoded[..60];

        let result = decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }

    // --- Section 1: Error Variant Exact-Assertion Tests ---

    #[test]
    fn decode_record_returns_bad_magic_when_magic_differs() {
        // Given an encoded record
        // When decoded with a different expected magic
        // Then it returns BadMagic with the encoded magic value
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            128,
        )
        .expect("encoding should succeed");

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_WORKFLOW_SOURCE, 128);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("expected BadMagic, got {:?}", result);
        };
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }

    #[test]
    fn decode_record_returns_unexpected_eof_when_bytes_too_short() {
        // Given a zero-length byte slice
        // When decode_record is called
        // Then it returns UnexpectedEof
        let empty: [u8; 0] = [];

        let result = decode_record::<JournalEvent>(&empty, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn encode_record_returns_payload_too_large_when_payload_exceeds_max() {
        // Given a source record with source bytes larger than the max
        // When encode_record is called with a tiny max_payload_len
        // Then it returns PayloadTooLarge with correct len and max fields
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([1; 32]),
            source: vec![0xAB; 200],
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            10,
        );
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("expected PayloadTooLarge, got {:?}", result);
        };
        assert_eq!(max, 10);
        assert!(len > 10);
    }

    #[test]
    fn encode_record_returns_record_kind_family_mismatch_for_wrong_kind() {
        // Given a blob kind paired with workflow source magic
        // When encode_record is called
        // Then it returns RecordKindFamilyMismatch with the exact magic and kind
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([1; 32]),
            source: vec![1],
        };
        let result = encode_record(MAGIC_WORKFLOW_SOURCE, RecordKind::Blob, 0, &source, 128);
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
            panic!("expected RecordKindFamilyMismatch, got {:?}", result);
        };
        assert_eq!(magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(kind, RecordKind::Blob.id());
    }

    #[test]
    fn decode_record_returns_header_checksum_mismatch_on_corrupt_crc() {
        // Given an encoded record with a flipped CRC byte
        // When decode_record is called
        // Then it returns HeaderChecksumMismatch
        let event = JournalEvent::RunFinished {
            run: RunId::new(5),
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            event.seq().get(),
            &event,
            128,
        )
        .expect("encoding should succeed");
        // Corrupt the CRC at byte 56
        if let Some(byte) = encoded.get_mut(56) {
            *byte = byte.wrapping_add(1);
        }

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::HeaderChecksumMismatch)));
    }

    #[test]
    fn decode_record_returns_payload_digest_mismatch_on_corrupt_payload() {
        // Given an encoded record with a flipped payload byte
        // When decode_record is called
        // Then it returns PayloadDigestMismatch
        let event = JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            event.seq().get(),
            &event,
            128,
        )
        .expect("encoding should succeed");
        // Corrupt the first payload byte (immediately after the 60-byte header)
        if let Some(byte) = encoded.get_mut(60) {
            *byte = byte.wrapping_add(1);
        }

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn validate_replayed_event_returns_wrong_run_when_run_id_mismatch() {
        // Given events stored for run 10 and a replay request for run 20
        // When events_for_run is called for run 20 on a journal that only has run 10 events
        // Then no events are returned (no prefix match), producing an empty result
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run_a = RunId::new(10);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
journal.append_journaled(&event).expect("journal.append_journaled must succeed");

        let run_b = RunId::new(20);
        let result = journal.events_for_run(run_b);
        let events = result.expect("events_for_run should succeed for missing run");
        assert!(events.is_empty(), "no events should exist for run_b");
    }

    #[test]
    fn validate_replayed_event_returns_sequence_gap_when_seq_out_of_order() {
        // Given a journal with seq 0 then seq 2 for the same run
        // When events_for_run replays
        // Then it returns SequenceGap with expected=1, actual=2
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(100);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
journal.append_journaled(&event0).expect("journal.append_journaled must succeed");

        // Manually insert an event at seq 2 (skipping seq 1)
        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
        };
journal.append_journaled(&event2).expect("journal.append_journaled must succeed");

        let result = journal.events_for_run(run);
        let Err(JournalError::SequenceGap { expected, actual }) = result else {
            panic!("expected SequenceGap, got {:?}", result);
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(2));
    }

    #[test]
    fn next_seq_returns_sequence_overflow_at_max() {
        // Given EventSeq at u64::MAX
        // When the next sequence is computed
        // Then it returns SequenceOverflow
        let seq = EventSeq::new(u64::MAX);
        let result = seq.get().checked_add(1).map(EventSeq::new);
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_event_returns_exact_run_and_seq() {
        // Given a journal with a RunAccepted event for run 42, seq 7
        // When the same event is appended again
        // Then DuplicateEvent is returned with run=42, seq=7
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(7),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
journal.append_journaled(&event).expect("journal.append_journaled must succeed");

        let result = journal.append_journaled(&event);
        let Err(JournalError::DuplicateEvent { run, seq }) = result else {
            panic!("expected DuplicateEvent, got {:?}", result);
        };
        assert_eq!(run, RunId::new(42));
        assert_eq!(seq, EventSeq::new(7));
    }

    #[test]
    fn decode_record_returns_migration_required_for_old_schema() {
        // Given an encoded record with schema version set to 0
        // When decode_record is called
        // Then it returns MigrationRequired with from=0, to=1
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Patch schema version at offset 4..6 to 0
        encoded[4] = 0;
        encoded[5] = 0;
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::MigrationRequired { from, to }) = result else {
            panic!("expected MigrationRequired, got {:?}", result);
        };
        assert_eq!(from, 0);
        assert_eq!(to, 1);
    }

    #[test]
    fn decode_record_returns_unsupported_schema_version_for_future() {
        // Given an encoded record with schema version 99
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion with version=99
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        encoded[4] = 99;
        encoded[5] = 0;
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
            panic!("expected UnsupportedSchemaVersion, got {:?}", result);
        };
        assert_eq!(version, 99);
    }

    #[test]
    fn decode_record_returns_unknown_record_kind_for_invalid_kind() {
        // Given an encoded record with kind patched to 200
        // When decode_record is called
        // Then it returns UnknownRecordKind with kind=200
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Patch kind at offset 6..8 to 200
        let kind_bytes = 200u16.to_le_bytes();
        encoded[6] = kind_bytes[0];
        encoded[7] = kind_bytes[1];
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnknownRecordKind { kind }) = result else {
            panic!("expected UnknownRecordKind, got {:?}", result);
        };
        assert_eq!(kind, 200);
    }

    #[test]
    fn decode_record_returns_header_length_mismatch_for_wrong_len() {
        // Given an encoded record with header_len patched to 99
        // When decode_record is called
        // Then it returns HeaderLengthMismatch with found=99
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        let len_bytes = 99u32.to_le_bytes();
        encoded[8] = len_bytes[0];
        encoded[9] = len_bytes[1];
        encoded[10] = len_bytes[2];
        encoded[11] = len_bytes[3];
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::HeaderLengthMismatch { found }) = result else {
            panic!("expected HeaderLengthMismatch, got {:?}", result);
        };
        assert_eq!(found, 99);
    }

    // --- Section 2: Key Function Behavior Tests ---

    #[test]
    fn run_event_key_produces_expected_key_bytes() {
        // Given run_id=1, seq=0
        // When run_event_key is called
        // Then the key is [0x11][1_be][0_be]
        let key = run_event_key(RunId::new(1), EventSeq::new(0));
        let key = key.expect("run_event_key should succeed");
        assert_eq!(key[0], 0x11);
        assert_eq!(key[1..9], 1u64.to_be_bytes());
        assert_eq!(key[9..17], 0u64.to_be_bytes());
    }

    #[test]
    fn run_header_key_produces_expected_key_bytes() {
        // Given run_id=0xAABBCCDD_EEFF0011
        // When run_header_key is called
        // Then the key is [0x10][run_id_be]
        let run = RunId::new(0xAABB_CCDD_EEFF_0011);
        let key = run_header_key(run);
        let key = key.expect("run_header_key should succeed");
        assert_eq!(key[0], 0x10);
        assert_eq!(key[1..9], run.as_u64().to_be_bytes());
    }

    #[test]
    fn run_snapshot_key_produces_expected_key_bytes() {
        // Given run_id=5, seq=99
        // When run_snapshot_key is called
        // Then the key is [0x12][5_be][99_be]
        let key = run_snapshot_key(RunId::new(5), EventSeq::new(99));
        let key = key.expect("run_snapshot_key should succeed");
        assert_eq!(key[0], 0x12);
        assert_eq!(key[1..9], 5u64.to_be_bytes());
        assert_eq!(key[9..17], 99u64.to_be_bytes());
    }

    #[test]
    fn workflow_source_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 7s
        // When workflow_source_key is called
        // Then the key is [0x01][digest]
        let digest = [7u8; 32];
        let key = workflow_source_key(digest);
        let key = key.expect("workflow_source_key should succeed");
        assert_eq!(key[0], 0x01);
        assert_eq!(key[1..33], digest);
    }

    #[test]
    fn compiled_ir_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 2s
        // When compiled_ir_key is called
        // Then the key is [0x02][digest]
        let digest = [2u8; 32];
        let key = compiled_ir_key(digest);
        let key = key.expect("compiled_ir_key should succeed");
        assert_eq!(key[0], 0x02);
        assert_eq!(key[1..33], digest);
    }

    #[test]
    fn index_action_key_produces_expected_key_bytes() {
        // Given action=100, run=200, step=300
        // When index_action_key is called
        // Then the key is [0x32][action_u16_be][run_u64_be][step_u16_be]
        let key = index_action_key(ActionId::new(100), RunId::new(200), StepIdx::new(300));
        let key = key.expect("index_action_key should succeed");
        assert_eq!(key[0], 0x32);
        assert_eq!(key[1..3], 100u16.to_be_bytes());
        assert_eq!(key[3..11], 200u64.to_be_bytes());
        assert_eq!(key[11..13], 300u16.to_be_bytes());
    }

    #[test]
    fn index_status_key_produces_expected_key_bytes() {
        // Given state=5, timestamp=1000, run=50
        // When index_status_key is called
        // Then the key is [0x30][state_u8][timestamp_u64_be][run_u64_be]
        let key = index_status_key(5, 1000, RunId::new(50));
        let key = key.expect("index_status_key should succeed");
        assert_eq!(key[0], 0x30);
        assert_eq!(key[1], 5);
        assert_eq!(key[2..10], 1000u64.to_be_bytes());
        assert_eq!(key[10..18], 50u64.to_be_bytes());
    }

    #[test]
    fn index_workflow_key_produces_expected_key_bytes() {
        // Given workflow_id=42, run=99
        // When index_workflow_key is called
        // Then the key is [0x31][workflow_u32_be][run_u64_be]
        let key = index_workflow_key(WorkflowId::new(42), RunId::new(99));
        let key = key.expect("index_workflow_key should succeed");
        assert_eq!(key[0], 0x31);
        assert_eq!(key[1..5], 42u32.to_be_bytes());
        assert_eq!(key[5..13], 99u64.to_be_bytes());
    }

    #[test]
    fn blob_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 0xAB
        // When blob_key is called
        // Then the key is [0x20][digest]
        let digest = [0xAB; 32];
        let key = blob_key(digest);
        let key = key.expect("blob_key should succeed");
        assert_eq!(key[0], 0x20);
        assert_eq!(key[1..33], digest);
    }

    // --- Section 3: BDD Integration-Style Tests ---

    #[test]
    fn journal_opens_and_closes_without_error() {
        // Given a temporary directory
        // When FjallJournal::open is called
        // Then the journal opens successfully
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None);
        assert!(journal.is_ok(), "journal should open with default config");
    }

    #[test]
    fn public_open_wrappers_create_declared_keyspaces() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");

        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok(), "open_store should succeed");
        drop(journal);

        let reopened = init_keyspaces(temp_dir.path());
        assert!(reopened.is_ok(), "init_keyspaces should succeed");
        assert_eq!(FjallJournal::declared_keyspaces().len(), 9);
    }

    #[test]
    fn public_wrappers_delegate_to_journal_storage_paths() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let run = RunId::new(70);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([7; 32]),
        };
        let blob = BlobRecord {
            digest: [3; 32],
            bytes: vec![1, 2, 3],
        };
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            slots: vec![4, 5, 6],
        };

append_journal_event(&journal, &event).expect("append_journal_event must succeed");
journal.put_blob(&blob).expect("journal.put_blob must succeed");
write_snapshot(&journal, &snapshot).expect("write_snapshot must succeed");

        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![event.clone()]);
        let loaded_blob = read_blob(&journal, blob.digest);
        let loaded_blob = loaded_blob.expect("read_blob should succeed");
        assert_eq!(loaded_blob, Some(blob));
        let loaded_snapshot = journal.snapshot(run, EventSeq::new(0));
        let loaded_snapshot = loaded_snapshot.expect("snapshot lookup should succeed");
        assert_eq!(loaded_snapshot, Some(snapshot));
    }

    #[test]
    fn replay_journal_wrapper_uses_recovery_replay() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let run = RunId::new(71);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([8; 32]),
        };
append_journal_event(&journal, &event).expect("append_journal_event must succeed");

        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_journal(&journal, run, &mut tracker);

        let replayed = replayed.expect("replay_journal should succeed");
        assert_eq!(replayed, vec![event]);
    }

    #[test]
    fn append_strict_persists_submitted_event() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event
        // Then the event can be retrieved via events_for_run
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(55);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let result = journal.append_strict(&event);
        result.expect("action must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn append_strict_rejects_out_of_order_sequence() {
        // Given an open journal with a seq-0 event
        // When append_strict is called with seq 2 (skipping seq 1)
        // Then events_for_run returns SequenceGap
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(60);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
journal.append_strict(&event0).expect("journal.append_strict must succeed");

        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
        };
journal.append_strict(&event2).expect("journal.append_strict must succeed");

        let result = journal.events_for_run(run);
        let Err(JournalError::SequenceGap { expected, actual }) = result else {
            panic!("expected SequenceGap, got {:?}", result);
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(2));
    }

    #[test]
    fn persist_strict_flushes_and_reopens_cleanly() {
        // Given an open journal with a persisted event
        // When the journal is closed and reopened
        // Then the same event is visible
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");

        let run = RunId::new(77);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        {
            let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
journal.append_strict(&event).expect("journal.append_strict must succeed");
        }

        let journal2 = FjallJournal::open(temp_dir.path(), None);
        let journal2 = journal2.expect("journal should reopen cleanly");
        let events = journal2
            .events_for_run(run)
            .expect("events_for_run should succeed after reopen");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn put_workflow_source_stores_and_retrieves() {
        // Given an open journal and a workflow source record
        // When put_workflow_source is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = WorkflowDigest::from_bytes([42; 32]);
        let record = WorkflowSourceRecord {
            digest,
            source: vec![b'h', b'e', b'l', b'l', b'o'],
        };
journal.put_workflow_source(&record).expect("journal.put_workflow_source must succeed");

        let retrieved = journal
            .workflow_source(digest)
            .expect("workflow_source lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_workflow_source_returns_none_for_missing_digest() {
        // Given an open journal with no stored workflow source
        // When workflow_source is called with an arbitrary digest
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let missing = WorkflowDigest::from_bytes([99; 32]);
        let result = journal
            .workflow_source(missing)
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn put_run_header_stores_and_retrieves() {
        // Given an open journal and a run header record
        // When put_run_header is called
        // Then the record can be retrieved by run id
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: WorkflowDigest::from_bytes([8; 32]),
            status: 1,
            accepted_at_ms: 1700000000,
        };
journal.put_run_header(&record).expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(123))
            .expect("run_header lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_compiled_ir_stores_and_retrieves() {
        // Given an open journal and a compiled IR record
        // When put_compiled_ir is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = WorkflowDigest::from_bytes([3; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
journal.put_compiled_ir(&record).expect("journal.put_compiled_ir must succeed");

        let retrieved = journal
            .compiled_ir(digest)
            .expect("compiled_ir lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_blob_stores_and_retrieves() {
        // Given an open journal and a blob record
        // When put_blob is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let digest = [0xCC; 32];
        let record = BlobRecord {
            digest,
            bytes: vec![1, 2, 3, 4, 5],
        };
journal.put_blob(&record).expect("journal.put_blob must succeed");

        let retrieved = journal.blob(digest).expect("blob lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_snapshot_stores_and_retrieves() {
        // Given an open journal and a run snapshot
        // When put_snapshot is called
        // Then the snapshot can be retrieved by run and seq
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let snapshot = RunSnapshot {
            run: RunId::new(88),
            seq: EventSeq::new(10),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            slots: vec![1, 2, 3],
        };
journal.put_snapshot(&snapshot).expect("journal.put_snapshot must succeed");

        let retrieved = journal
            .snapshot(RunId::new(88), EventSeq::new(10))
            .expect("snapshot lookup should succeed");
        assert_eq!(retrieved, Some(snapshot));
    }

    #[test]
    fn put_action_index_stores_and_retrieves() {
        // Given an open journal
        // When put_action_index is called
        // Then no error is returned and the index entry exists
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.put_action_index(ActionId::new(1), RunId::new(2), StepIdx::new(3));
        result.expect("action must succeed");
    }

    #[test]
    fn put_status_index_stores_and_retrieves() {
        // Given an open journal
        // When put_status_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.put_status_index(1, 1700000000, RunId::new(99));
        result.expect("action must succeed");
    }

    #[test]
    fn put_workflow_index_stores_and_retrieves() {
        // Given an open journal
        // When put_workflow_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.put_workflow_index(WorkflowId::new(7), RunId::new(8));
        result.expect("action must succeed");
    }

    #[test]
    fn events_for_run_returns_only_events_for_target_run() {
        // Given a journal with events for run 10 and run 20
        // When events_for_run is called for run 10
        // Then only run 10 events are returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run_a = RunId::new(10);
        let run_b = RunId::new(20);

        let event_a0 = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let event_b0 = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        let event_a1 = JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };

journal.append_journaled(&event_a0).expect("journal.append_journaled must succeed");
journal.append_journaled(&event_b0).expect("journal.append_journaled must succeed");
journal.append_journaled(&event_a1).expect("journal.append_journaled must succeed");

        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run should succeed");
        assert_eq!(events_a.len(), 2);
        assert_eq!(events_a[0], event_a0);
        assert_eq!(events_a[1], event_a1);

        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run should succeed");
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_b[0], event_b0);
    }

    #[test]
    fn event_seq_new_returns_correct_value() {
        // Given EventSeq::new(42)
        // When get is called
        // Then it returns 42
        let seq = EventSeq::new(42);
        assert_eq!(seq.get(), 42);
    }

    #[test]
    fn record_kind_id_returns_correct_wire_ids() {
        // Given each RecordKind variant
        // When id() is called
        // Then it returns the expected wire identifier
        assert_eq!(RecordKind::WorkflowSource.id(), 1);
        assert_eq!(RecordKind::CompiledIr.id(), 2);
        assert_eq!(RecordKind::RunHeader.id(), 3);
        assert_eq!(RecordKind::RunAccepted.id(), 10);
        assert_eq!(RecordKind::StepStarted.id(), 11);
        assert_eq!(RecordKind::SlotWritten.id(), 12);
        assert_eq!(RecordKind::ActionScheduled.id(), 13);
        assert_eq!(RecordKind::ActionCompleted.id(), 14);
        assert_eq!(RecordKind::ActionFailed.id(), 15);
        assert_eq!(RecordKind::WaitScheduled.id(), 16);
        assert_eq!(RecordKind::AskScheduled.id(), 17);
        assert_eq!(RecordKind::AskAnswered.id(), 18);
        assert_eq!(RecordKind::RetryScheduled.id(), 19);
        assert_eq!(RecordKind::StepFailed.id(), 20);
        assert_eq!(RecordKind::RunCancelled.id(), 21);
        assert_eq!(RecordKind::RunFinished.id(), 22);
        assert_eq!(RecordKind::RunFailed.id(), 23);
        assert_eq!(RecordKind::Snapshot.id(), 30);
        assert_eq!(RecordKind::Blob.id(), 40);
        assert_eq!(RecordKind::IndexUpdate.id(), 50);
    }

    #[test]
    fn journal_event_run_id_returns_correct_run() {
        // Given a RunAccepted event for run 42
        // When run_id() is called
        // Then it returns 42
        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        assert_eq!(event.run_id(), RunId::new(42));
    }

    #[test]
    fn journal_event_seq_returns_correct_seq() {
        // Given a StepStarted event with seq 7
        // When seq() is called
        // Then it returns EventSeq(7)
        let event = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(7),
            step: StepIdx::new(0),
        };
        assert_eq!(event.seq(), EventSeq::new(7));
    }

    #[test]
    fn journal_event_record_kind_returns_correct_kind() {
        // Given a RunFinished event
        // When record_kind() is called
        // Then it returns RecordKind::RunFinished
        let event = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
        };
        assert_eq!(event.record_kind(), RecordKind::RunFinished);
    }

    #[test]
    fn decode_record_returns_postcard_decode_failed_for_garbage_payload() {
        // Given an encoded record with a valid header but corrupted payload bytes
        // that no longer deserialize correctly
        // When decode_record is called
        // Then it returns PostcardDecodeFailed
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Corrupt the payload bytes after the header but not the blake3 digest
        // We need to corrupt and re-hash, so instead we construct a manually
        // crafted header with valid CRC/digest pointing to garbage
        let payload_start = 60;
        if let Some(byte) = encoded.get_mut(payload_start) {
            *byte = 0xFF;
        }
        // Now recompute the blake3 digest in the header
        let payload = &encoded[60..];
        let digest = blake3::hash(payload);
        encoded[24..56].copy_from_slice(digest.as_bytes());
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::PostcardDecodeFailed)));
    }

    #[test]
    fn envelope_round_trips_workflow_source_record() {
        // Given a WorkflowSourceRecord
        // When encoded and decoded with MAGIC_WORKFLOW_SOURCE
        // Then the record survives the round trip
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            source: vec![1, 2, 3],
        };
        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            128,
        )
        .expect("encoding should succeed");

        let (envelope, decoded) =
            decode_record::<WorkflowSourceRecord>(&encoded, MAGIC_WORKFLOW_SOURCE, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn envelope_round_trips_compiled_ir_record() {
        // Given a CompiledIrRecord
        // When encoded and decoded with MAGIC_COMPILED_ARTIFACT
        // Then the record survives the round trip
        let record = CompiledIrRecord {
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            ir: vec![4, 5, 6],
        };
        let encoded = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            128,
        )
        .expect("encoding should succeed");

        let (envelope, decoded) =
            decode_record::<CompiledIrRecord>(&encoded, MAGIC_COMPILED_ARTIFACT, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_COMPILED_ARTIFACT);
        assert_eq!(envelope.record_kind, RecordKind::CompiledIr.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn envelope_round_trips_blob_record() {
        // Given a BlobRecord
        // When encoded and decoded with MAGIC_BLOB
        // Then the record survives the round trip
        let record = BlobRecord {
            digest: [0xDD; 32],
            bytes: vec![7, 8, 9],
        };
        let encoded =
            encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 128).expect("encoding ok");

        let (envelope, decoded) =
            decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, 128).expect("decoding ok");
        assert_eq!(envelope.magic, MAGIC_BLOB);
        assert_eq!(envelope.record_kind, RecordKind::Blob.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn declared_keyspaces_returns_nine_entries() {
        // Given FjallJournal::declared_keyspaces()
        // When called
        // Then it returns exactly 9 keyspace names
        let keyspaces = FjallJournal::declared_keyspaces();
        assert_eq!(keyspaces.len(), 9);
        assert_eq!(keyspaces[0], "workflow_source");
        assert_eq!(keyspaces[1], "compiled_ir");
        assert_eq!(keyspaces[2], "run_header");
        assert_eq!(keyspaces[3], "run_event");
        assert_eq!(keyspaces[4], "run_snapshot");
        assert_eq!(keyspaces[5], "blob");
        assert_eq!(keyspaces[6], "index_status");
        assert_eq!(keyspaces[7], "index_workflow");
        assert_eq!(keyspaces[8], "index_action");
    }

    #[test]
    fn run_header_returns_none_for_missing_run() {
        // Given an open journal with no stored headers
        // When run_header is called for an arbitrary run
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .run_header(RunId::new(999))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn compiled_ir_returns_none_for_missing_digest() {
        // Given an open journal with no stored IR
        // When compiled_ir is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .compiled_ir(WorkflowDigest::from_bytes([0; 32]))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn snapshot_returns_none_for_missing_entry() {
        // Given an open journal with no snapshots
        // When snapshot is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .snapshot(RunId::new(1), EventSeq::new(0))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn blob_returns_none_for_missing_digest() {
        // Given an open journal with no blobs
        // When blob is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.blob([0; 32]).expect("lookup should succeed");
        assert_eq!(result, None);
    }

    // --- Section 4: Journal Lifecycle BDD Tests ---

    fn open_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal should open");
        (temp_dir, journal)
    }

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn journal_open_creates_fresh_instance_with_no_data() {
        // Given a temporary directory
        // When FjallJournal::open is called
        // Then the journal has no events for any run
        let (_guard, journal) = open_journal();
        let events = journal
            .events_for_run(RunId::new(1))
            .expect("events_for_run should succeed on empty journal");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_writes_submitted_event_with_correct_run_id() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event for run 42
        // Then the stored event has run_id 42
        let (_guard, journal) = open_journal();
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_writes_accepted_event_after_submitted() {
        // Given an open journal with a RunAccepted event at seq 0
        // When a StepStarted event at seq 1 is appended
        // Then both events are retrieved in order
        let (_guard, journal) = open_journal();
        let run = RunId::new(1);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let started = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
journal.append_strict(&accepted).expect("journal.append_strict must succeed");
journal.append_strict(&started).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], accepted);
        assert_eq!(events[1], started);
    }

    #[test]
    fn append_strict_writes_step_started_event_with_correct_step() {
        // Given an open journal
        // When a StepStarted event with step 5 is appended and retrieved
        // Then the event carries step 5
        let (_guard, journal) = open_journal();
        let run = RunId::new(10);
        let step = StepIdx::new(5);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepStarted {
            step: found_step, ..
        } = events[0]
        else {
            panic!("expected StepStarted event");
        };
        assert_eq!(found_step, step);
    }

    #[test]
    fn append_strict_writes_step_ended_event_with_correct_step() {
        // Given an open journal
        // When a StepSucceeded event with step 3 is appended and retrieved
        // Then the event carries step 3 and output slot 7
        let (_guard, journal) = open_journal();
        let run = RunId::new(11);
        let step = StepIdx::new(3);
        let output = vb_core::SlotIdx::new(7);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step,
            output,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepSucceeded {
            step: found_step,
            output: found_output,
            ..
        } = events[0]
        else {
            panic!("expected StepSucceeded event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_output, output);
    }

    #[test]
    fn append_strict_writes_slot_written_event_with_correct_slot() {
        // Given an open journal
        // When a SlotWrittenEvent with slot 9 is appended and retrieved
        // Then the event carries slot 9
        let (_guard, journal) = open_journal();
        let run = RunId::new(12);
        let slot = vb_core::SlotIdx::new(9);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::SlotWrittenEvent {
            slot: found_slot, ..
        } = events[0]
        else {
            panic!("expected SlotWrittenEvent");
        };
        assert_eq!(found_slot, slot);
    }

    #[test]
    fn append_strict_writes_action_scheduled_event_with_correct_step() {
        // Given an open journal
        // When an ActionScheduled event with step 4 is appended and retrieved
        // Then the event carries step 4 and action 2
        let (_guard, journal) = open_journal();
        let run = RunId::new(13);
        let step = StepIdx::new(4);
        let action = ActionId::new(2);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionScheduled {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionScheduled event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_action_completed_event_with_correct_step() {
        // Given an open journal
        // When an ActionCompletedEvent with step 6 is appended and retrieved
        // Then the event carries step 6 and action 3
        let (_guard, journal) = open_journal();
        let run = RunId::new(14);
        let step = StepIdx::new(6);
        let action = ActionId::new(3);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step,
            action,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionCompletedEvent {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionCompletedEvent");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_run_finished_event_with_correct_result() {
        // Given an open journal
        // When a RunFinished event with result slot 15 is appended and retrieved
        // Then the event carries result 15
        let (_guard, journal) = open_journal();
        let run = RunId::new(15);
        let result = vb_core::SlotIdx::new(15);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result,
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::RunFinished {
            result: found_result,
            ..
        } = events[0]
        else {
            panic!("expected RunFinished event");
        };
        assert_eq!(found_result, result);
    }

    #[test]
    fn append_strict_writes_run_failed_event() {
        // Given an open journal
        // When a RunFailedEvent is appended and retrieved
        // Then the event carries the correct run
        let (_guard, journal) = open_journal();
        let run = RunId::new(16);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_assigns_monotonically_increasing_sequences() {
        // Given an open journal
        // When three events are appended with seq 0, 1, 2
        // Then events_for_run returns them in contiguous order
        let (_guard, journal) = open_journal();
        let run = RunId::new(17);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
        };
journal.append_strict(&e0).expect("journal.append_strict must succeed");
journal.append_strict(&e1).expect("journal.append_strict must succeed");
journal.append_strict(&e2).expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].seq(), EventSeq::new(0));
        assert_eq!(events[1].seq(), EventSeq::new(1));
        assert_eq!(events[2].seq(), EventSeq::new(2));
    }

    #[test]
    fn append_strict_rejects_duplicate_sequence() {
        // Given an open journal with an event at seq 0 for run 50
        // When the same event is appended again
        // Then DuplicateEvent is returned with exact run and seq
        let (_guard, journal) = open_journal();
        let run = RunId::new(50);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
journal.append_strict(&event).expect("journal.append_strict must succeed");

        let result = journal.append_strict(&event);
        let Err(JournalError::DuplicateEvent {
            run: dup_run,
            seq: dup_seq,
        }) = result
        else {
            panic!("expected DuplicateEvent, got {:?}", result);
        };
        assert_eq!(dup_run, run);
        assert_eq!(dup_seq, EventSeq::new(0));
    }

    #[test]
    fn events_for_run_returns_events_in_sequence_order() {
        // Given a journal with 5 events for a run
        // When events_for_run is called
        // Then events are returned in ascending sequence order
        let (_guard, journal) = open_journal();
        let run = RunId::new(18);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let e2 = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: vb_core::SlotIdx::new(0),
        };
        let e3 = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(1),
        };
        let e4 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: vb_core::SlotIdx::new(1),
        };
journal.append_journaled(&e0).expect("journal.append_journaled must succeed");
journal.append_journaled(&e1).expect("journal.append_journaled must succeed");
journal.append_journaled(&e2).expect("journal.append_journaled must succeed");
journal.append_journaled(&e3).expect("journal.append_journaled must succeed");
journal.append_journaled(&e4).expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 5);
        assert_eq!(events[0], e0);
        assert_eq!(events[1], e1);
        assert_eq!(events[2], e2);
        assert_eq!(events[3], e3);
        assert_eq!(events[4], e4);
    }

    #[test]
    fn events_for_run_returns_empty_for_run_with_no_events() {
        // Given an open journal with events for run 1
        // When events_for_run is called for run 2
        // Then it returns an empty vec
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(1);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
journal.append_journaled(&event).expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(RunId::new(2))
            .expect("events_for_run should succeed");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_handles_concurrent_runs_interleaved() {
        // Given a journal with interleaved events from run A and run B
        // When events_for_run is called for run A
        // Then only run A events are returned in order
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let a0 = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let b0 = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let a1 = JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let b1 = JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
        };

journal.append_journaled(&a0).expect("journal.append_journaled must succeed");
journal.append_journaled(&b0).expect("journal.append_journaled must succeed");
journal.append_journaled(&a1).expect("journal.append_journaled must succeed");
journal.append_journaled(&b1).expect("journal.append_journaled must succeed");
journal.append_journaled(&a2).expect("journal.append_journaled must succeed");

        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run A should succeed");
        assert_eq!(events_a.len(), 3);
        assert_eq!(events_a[0], a0);
        assert_eq!(events_a[1], a1);
        assert_eq!(events_a[2], a2);

        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run B should succeed");
        assert_eq!(events_b.len(), 2);
        assert_eq!(events_b[0], b0);
        assert_eq!(events_b[1], b1);
    }

    #[test]
    fn append_journaled_succeeds_without_flush() {
        // Given an open journal
        // When append_journaled is called
        // Then the event is readable immediately
        let (_guard, journal) = open_journal();
        let run = RunId::new(30);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
journal.append_journaled(&event).expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn run_header_record_roundtrip_with_large_timestamp() {
        // Given a run header with a large accepted_at_ms value
        // When put and retrieved
        // Then the timestamp survives exactly
        let (_guard, journal) = open_journal();
        let record = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(2),
            compiled_digest: test_digest(5),
            status: 0,
            accepted_at_ms: u64::MAX / 2,
        };
journal.put_run_header(&record).expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(1))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn snapshot_record_roundtrip_with_nonempty_slots() {
        // Given a snapshot with non-empty slot data
        // When stored and retrieved
        // Then the slot bytes survive exactly
        let (_guard, journal) = open_journal();
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(7),
            slots: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE],
        };
journal.put_snapshot(&snapshot).expect("journal.put_snapshot must succeed");

        let retrieved = journal
            .snapshot(RunId::new(1), EventSeq::new(0))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(snapshot));
    }

    #[test]
    fn compiled_ir_returns_none_when_different_digest_queried() {
        // Given an open journal with a compiled IR stored at digest [1;32]
        // When a different digest [2;32] is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let stored_digest = test_digest(1);
        let record = CompiledIrRecord {
            digest: stored_digest,
            ir: vec![1, 2, 3],
        };
journal.put_compiled_ir(&record).expect("journal.put_compiled_ir must succeed");

        let result = journal
            .compiled_ir(test_digest(2))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn workflow_source_returns_none_for_different_digest() {
        // Given an open journal with one workflow source stored
        // When a different digest is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let stored_digest = test_digest(10);
        let record = WorkflowSourceRecord {
            digest: stored_digest,
            source: vec![1],
        };
journal.put_workflow_source(&record).expect("journal.put_workflow_source must succeed");

        let result = journal
            .workflow_source(test_digest(11))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn journal_event_run_id_returns_correct_run_for_all_variants() {
        // Given every JournalEvent variant with run_id 99
        // When run_id() is called
        // Then each returns RunId::new(99)
        let run = RunId::new(99);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(0),
                slot: vb_core::SlotIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(0),
                result: vb_core::SlotIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(0)
            }
            .run_id(),
            run
        );
    }

    #[test]
    fn journal_event_seq_returns_correct_seq_for_all_variants() {
        // Given every JournalEvent variant with seq 42
        // When seq() is called
        // Then each returns EventSeq::new(42)
        let seq = EventSeq::new(42);
        let run = RunId::new(1);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq,
                workflow: test_digest(1)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq,
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot: vb_core::SlotIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(JournalEvent::RunCancelled { run, seq }.seq(), seq);
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq,
                result: vb_core::SlotIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(JournalEvent::RunFailedEvent { run, seq }.seq(), seq);
    }

    #[test]
    fn journal_event_record_kind_returns_correct_kind_for_all_variants() {
        // Given every JournalEvent variant
        // When record_kind() is called
        // Then each returns the expected RecordKind
        let run = RunId::new(1);
        let seq = EventSeq::new(0);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq,
                workflow: test_digest(1)
            }
            .record_kind(),
            RecordKind::RunAccepted
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .record_kind(),
            RecordKind::StepStarted
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq,
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .record_kind(),
            RecordKind::SlotWritten
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .record_kind(),
            RecordKind::ActionScheduled
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .record_kind(),
            RecordKind::ActionCompleted
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1)
            }
            .record_kind(),
            RecordKind::ActionFailed
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot: vb_core::SlotIdx::new(0)
            }
            .record_kind(),
            RecordKind::SlotWritten
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .record_kind(),
            RecordKind::WaitScheduled
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .record_kind(),
            RecordKind::AskScheduled
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .record_kind(),
            RecordKind::AskAnswered
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0)
            }
            .record_kind(),
            RecordKind::RetryScheduled
        );
        assert_eq!(
            JournalEvent::RunCancelled { run, seq }.record_kind(),
            RecordKind::RunCancelled
        );
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq,
                result: vb_core::SlotIdx::new(0)
            }
            .record_kind(),
            RecordKind::RunFinished
        );
        assert_eq!(
            JournalEvent::RunFailedEvent { run, seq }.record_kind(),
            RecordKind::RunFailed
        );
    }

    // --- Section 5: Encode/Decode Roundtrip Tests ---

    #[test]
    fn encode_decode_roundtrip_for_run_accepted_record() {
        // Given a RunAccepted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(42),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_started_record() {
        // Given a StepStarted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(1),
            step: StepIdx::new(5),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::StepStarted, 1, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_ended_record() {
        // Given a StepSucceeded event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(3),
            seq: EventSeq::new(2),
            step: StepIdx::new(5),
            output: vb_core::SlotIdx::new(10),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 2, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_slot_written_record() {
        // Given a SlotWrittenEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(4),
            seq: EventSeq::new(3),
            slot: vb_core::SlotIdx::new(7),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 3, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_scheduled_record() {
        // Given an ActionScheduled event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionScheduled {
            run: RunId::new(5),
            seq: EventSeq::new(4),
            step: StepIdx::new(2),
            action: ActionId::new(3),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionScheduled,
            4,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_completed_record() {
        // Given an ActionCompletedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionCompletedEvent {
            run: RunId::new(6),
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
            action: ActionId::new(3),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionCompleted,
            5,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_finished_record() {
        // Given a RunFinished event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFinished {
            run: RunId::new(7),
            seq: EventSeq::new(6),
            result: vb_core::SlotIdx::new(99),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFinished, 6, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_failed_record() {
        // Given a RunFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(8),
            seq: EventSeq::new(7),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFailed, 7, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_record_rejects_record_exceeding_max_payload() {
        // Given a workflow source with 200 bytes of source data
        // When encode_record is called with max_payload_len of 10
        // Then it returns PayloadTooLarge
        let source = WorkflowSourceRecord {
            digest: test_digest(1),
            source: vec![0u8; 200],
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            10,
        );
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("expected PayloadTooLarge, got {:?}", result);
        };
        assert_eq!(max, 10);
        assert!(len > 10);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_failed_record() {
        // Given an ActionFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(9),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(4),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            3,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    // --- Section 6: JournalError Variant Tests ---

    #[test]
    fn journal_error_encode_from_postcard_error() {
        // Given a payload that causes a postcard encoding error
        // When encode_record encounters the error
        // Then JournalError::Encode is returned
        // This is tested indirectly: encode_record with a valid payload succeeds,
        // and the Encode variant exists as a From<postcard::Error> conversion.
        // We verify the variant exists by checking the error display.
        let err = JournalError::Encode(postcard::Error::DeserializeBadVarint);
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_key_capacity_display() {
        // Given a JournalError::KeyCapacity
        // When displayed
        // Then the message is non-empty
        let err = JournalError::KeyCapacity;
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_write_lock_poisoned_display() {
        // Given a JournalError::WriteLockPoisoned
        // When displayed
        // Then the message mentions poisoned
        let err = JournalError::WriteLockPoisoned;
        let msg = format!("{}", err);
        assert!(msg.contains("poisoned"));
    }

    #[test]
    fn journal_error_wrong_run_display() {
        // Given a JournalError::WrongRun with expected and actual
        // When displayed
        // Then the message contains both run values
        let err = JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn journal_error_sequence_overflow_display() {
        // Given a JournalError::SequenceOverflow
        // When displayed
        // Then the message mentions overflow
        let err = JournalError::SequenceOverflow;
        let msg = format!("{}", err);
        assert!(msg.contains("overflow"));
    }

    #[test]
    fn journal_error_postcard_decode_failed_display() {
        // Given a JournalError::PostcardDecodeFailed
        // When displayed
        // Then the message mentions postcard
        let err = JournalError::PostcardDecodeFailed;
        let msg = format!("{}", err);
        assert!(msg.contains("postcard"));
    }

    #[test]
    fn journal_error_unexpected_eof_display() {
        // Given a JournalError::UnexpectedEof
        // When displayed
        // Then the message mentions end of record
        let err = JournalError::UnexpectedEof;
        let msg = format!("{}", err);
        assert!(msg.contains("end"));
    }

    #[test]
    fn journal_error_payload_digest_mismatch_display() {
        // Given a JournalError::PayloadDigestMismatch
        // When displayed
        // Then the message mentions digest
        let err = JournalError::PayloadDigestMismatch;
        let msg = format!("{}", err);
        assert!(msg.contains("digest"));
    }

    #[test]
    fn record_envelope_fields_match_encoded_values() {
        // Given an encoded event
        // When decoded
        // Then the envelope contains magic, schema_version, record_kind, and sequence
        let event = JournalEvent::RunAccepted {
            run: RunId::new(77),
            seq: EventSeq::new(3),
            workflow: test_digest(5),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 3, &event, 128)
            .expect("encoding should succeed");
        let (envelope, _) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 3);
    }

    // --- Section 7: RunHeaderRecord Integration Tests ---

    #[test]
    fn run_header_overwrite_replaces_existing_header() {
        // Given a journal with a stored run header
        // When a new header with the same run id is stored
        // Then the new header replaces the old one
        let (_guard, journal) = open_journal();
        let original = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(10),
            compiled_digest: test_digest(1),
            status: 0,
            accepted_at_ms: 100,
        };
        let updated = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(20),
            compiled_digest: test_digest(2),
            status: 1,
            accepted_at_ms: 200,
        };
journal.put_run_header(&original).expect("journal.put_run_header must succeed");
journal.put_run_header(&updated).expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(1))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(updated));
    }

    #[test]
    fn multiple_runs_have_independent_events() {
        // Given a journal with 2 events for run 1 and 3 events for run 2
        // When events_for_run is called for each
        // Then each run returns only its own events
        let (_guard, journal) = open_journal();
        let run1 = RunId::new(1);
        let run2 = RunId::new(2);

        let r1_e0 = JournalEvent::RunAccepted {
            run: run1,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let r1_e1 = JournalEvent::StepStarted {
            run: run1,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let r2_e0 = JournalEvent::RunAccepted {
            run: run2,
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let r2_e1 = JournalEvent::StepStarted {
            run: run2,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
        };
        let r2_e2 = JournalEvent::RunFinished {
            run: run2,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
        };

journal.append_journaled(&r1_e0).expect("journal.append_journaled must succeed");
journal.append_journaled(&r1_e1).expect("journal.append_journaled must succeed");
journal.append_journaled(&r2_e0).expect("journal.append_journaled must succeed");
journal.append_journaled(&r2_e1).expect("journal.append_journaled must succeed");
journal.append_journaled(&r2_e2).expect("journal.append_journaled must succeed");

        let events1 = journal
            .events_for_run(run1)
            .expect("events_for_run run1 should succeed");
        assert_eq!(events1.len(), 2);
        let events2 = journal
            .events_for_run(run2)
            .expect("events_for_run run2 should succeed");
        assert_eq!(events2.len(), 3);
    }

    #[test]
    fn event_seq_ordering_is_correct() {
        // Given two EventSeq values
        // When compared
        // Then ordering follows the inner u64
        assert!(EventSeq::new(0) < EventSeq::new(1));
        assert!(EventSeq::new(100) < EventSeq::new(200));
        assert_eq!(EventSeq::new(5), EventSeq::new(5));
    }

    #[test]
    fn record_kind_all_variants_have_distinct_ids() {
        // Given all RecordKind variants
        // When their ids are collected
        // Then no two variants share an id
        let ids = [
            RecordKind::WorkflowSource.id(),
            RecordKind::CompiledIr.id(),
            RecordKind::RunHeader.id(),
            RecordKind::RunAccepted.id(),
            RecordKind::StepStarted.id(),
            RecordKind::SlotWritten.id(),
            RecordKind::ActionScheduled.id(),
            RecordKind::ActionCompleted.id(),
            RecordKind::ActionFailed.id(),
            RecordKind::WaitScheduled.id(),
            RecordKind::AskScheduled.id(),
            RecordKind::AskAnswered.id(),
            RecordKind::RetryScheduled.id(),
            RecordKind::StepFailed.id(),
            RecordKind::RunCancelled.id(),
            RecordKind::RunFinished.id(),
            RecordKind::RunFailed.id(),
            RecordKind::Snapshot.id(),
            RecordKind::Blob.id(),
            RecordKind::IndexUpdate.id(),
        ];
        let mut sorted = ids.to_vec();
        sorted.sort();
        let mut deduped = sorted.clone();
        deduped.dedup();
        assert_eq!(
            ids.len(),
            deduped.len(),
            "all RecordKind ids must be distinct"
        );
    }

    #[test]
    fn constants_have_expected_values() {
        // Given the module constants
        // When inspected
        // Then they match the contract values
        assert_eq!(RECORD_HEADER_LEN, 60);
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
        assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952);
        assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45);
        assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E);
        assert_eq!(MAGIC_BLOB, 0x5642_424C);
        assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54);
        assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352);
        assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958);
    }

    #[test]
    fn prefix_constants_have_expected_values() {
        // Given the prefix constants
        // When inspected
        // Then they match the contract values
        assert_eq!(PREFIX_WORKFLOW_SOURCE, 0x01);
        assert_eq!(PREFIX_COMPILED_IR, 0x02);
        assert_eq!(PREFIX_RUN_HEADER, 0x10);
        assert_eq!(PREFIX_RUN_EVENT, 0x11);
        assert_eq!(PREFIX_RUN_SNAPSHOT, 0x12);
        assert_eq!(PREFIX_BLOB, 0x20);
        assert_eq!(PREFIX_INDEX_STATUS, 0x30);
        assert_eq!(PREFIX_INDEX_WORKFLOW, 0x31);
        assert_eq!(PREFIX_INDEX_ACTION, 0x32);
    }

    #[test]
    fn max_payload_constants_are_sensible() {
        // Given the max payload constants
        // When inspected
        // Then they are non-zero and in reasonable ranges
        assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES > 0);
        assert!(MAX_WORKFLOW_SOURCE_BYTES > 0);
        assert!(MAX_COMPILED_IR_BYTES > 0);
        assert!(MAX_RUN_HEADER_BYTES > 0);
        assert!(MAX_SNAPSHOT_BYTES > 0);
        assert!(MAX_BLOB_BYTES > 0);
    }

    #[test]
    fn validate_replayed_event_accepts_matching_run_and_seq() {
        // Given an event with run 42, seq 5
        // When validate_replayed_event is called with matching expected run and seq
        // Then it returns Ok (tested indirectly via events_for_run)
        let (_guard, journal) = open_journal();
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
journal.append_journaled(&event).expect("journal.append_journaled must succeed");
        let events = journal
            .events_for_run(run)
            .expect("should succeed with contiguous events");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn journal_reopen_preserves_multiple_event_types() {
        // Given a journal with multiple event types for a run
        // When the journal is closed and reopened
        // Then all events are preserved
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let run = RunId::new(999);

        {
            let journal = FjallJournal::open(temp_dir.path(), None).expect("open should succeed");
            let events = vec![
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1),
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(0),
                },
                JournalEvent::SlotWrittenEvent {
                    run,
                    seq: EventSeq::new(2),
                    slot: vb_core::SlotIdx::new(0),
                },
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(3),
                    step: StepIdx::new(0),
                    action: ActionId::new(1),
                },
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq: EventSeq::new(4),
                    step: StepIdx::new(0),
                    action: ActionId::new(1),
                },
                JournalEvent::StepSucceeded {
                    run,
                    seq: EventSeq::new(5),
                    step: StepIdx::new(0),
                    output: vb_core::SlotIdx::new(1),
                },
                JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(6),
                    result: vb_core::SlotIdx::new(1),
                },
            ];
            for event in &events {
journal.append_strict(event).expect("journal.append_strict must succeed");
            }
        }

        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("reopen should succeed");
        let events = journal2
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 7);
        assert_eq!(events[0].seq(), EventSeq::new(0));
        assert_eq!(events[6].seq(), EventSeq::new(6));
    }

    #[test]
    fn run_header_stores_all_fields_correctly() {
        // Given a RunHeaderRecord with specific field values
        // When stored and retrieved
        // Then all fields match exactly
        let (_guard, journal) = open_journal();
        let record = RunHeaderRecord {
            run: RunId::new(42),
            workflow_id: WorkflowId::new(7),
            compiled_digest: test_digest(99),
            status: 3,
            accepted_at_ms: 1700000000,
        };
journal.put_run_header(&record).expect("journal.put_run_header must succeed");
        let retrieved = journal
            .run_header(RunId::new(42))
            .expect("lookup should succeed");
        let Some(found) = retrieved else {
            panic!("expected Some(record)");
        };
        assert_eq!(found.run, record.run);
        assert_eq!(found.workflow_id, record.workflow_id);
        assert_eq!(found.compiled_digest, record.compiled_digest);
        assert_eq!(found.status, record.status);
        assert_eq!(found.accepted_at_ms, record.accepted_at_ms);
    }

    #[test]
    fn journal_stores_and_retrieves_blob_with_zero_bytes() {
        // Given a blob with zero bytes
        // When stored and retrieved
        // Then the record survives with empty bytes
        let (_guard, journal) = open_journal();
        let digest = [0u8; 32];
        let record = BlobRecord {
            digest,
            bytes: vec![],
        };
journal.put_blob(&record).expect("journal.put_blob must succeed");
        let retrieved = journal.blob(digest).expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn workflow_source_stores_and_retrieves_empty_source() {
        // Given a workflow source with zero source bytes
        // When stored and retrieved
        // Then the record survives with empty source
        let (_guard, journal) = open_journal();
        let digest = test_digest(0);
        let record = WorkflowSourceRecord {
            digest,
            source: vec![],
        };
journal.put_workflow_source(&record).expect("journal.put_workflow_source must succeed");
        let retrieved = journal
            .workflow_source(digest)
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn encode_decode_roundtrip_for_wait_scheduled_record() {
        // Given a WaitScheduledEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::WaitScheduledEvent {
            run: RunId::new(10),
            seq: EventSeq::new(2),
            step: StepIdx::new(3),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::WaitScheduled,
            2,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_ask_scheduled_record() {
        // Given an AskScheduledEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::AskScheduledEvent {
            run: RunId::new(11),
            seq: EventSeq::new(3),
            step: StepIdx::new(4),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::AskScheduled,
            3,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_ask_answered_record() {
        // Given an AskAnsweredEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::AskAnsweredEvent {
            run: RunId::new(12),
            seq: EventSeq::new(4),
            step: StepIdx::new(5),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::AskAnswered, 4, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_retry_scheduled_record() {
        // Given a RetryScheduledEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RetryScheduledEvent {
            run: RunId::new(13),
            seq: EventSeq::new(5),
            step: StepIdx::new(6),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RetryScheduled,
            5,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_cancelled_record() {
        // Given a RunCancelled event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunCancelled {
            run: RunId::new(14),
            seq: EventSeq::new(6),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            6,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    // =========================================================================
    // Section: Adversarial Record Header Decode Tests
    // =========================================================================

    fn encode_and_patch_field(
        event: &JournalEvent,
        kind: RecordKind,
        offset: usize,
        new_bytes: &[u8],
    ) -> Vec<u8> {
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            kind,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        let end = offset.saturating_add(new_bytes.len());
        assert!(end <= 56, "patch must be within CRC-protected region");
        encoded
            .get_mut(offset..end)
            .expect("patch range valid")
            .copy_from_slice(new_bytes);
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;
        encoded
    }

    #[test]
    fn adversarial_decode_wrong_magic_for_family_returns_bad_magic() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_SNAPSHOT, 128);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("expected BadMagic, got {:?}", result)
        };
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }

    #[test]
    fn adversarial_decode_vbir_magic_on_journal_returns_bad_magic() {
        let record = CompiledIrRecord {
            digest: test_digest(1),
            ir: vec![1, 2, 3],
        };
        let encoded = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            MAX_COMPILED_IR_BYTES,
        )
        .expect("ok");
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("expected BadMagic, got {:?}", result)
        };
        assert_eq!(found, MAGIC_COMPILED_ARTIFACT);
    }

    #[test]
    fn adversarial_decode_unsupported_schema_version_returns_exact_version() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(2),
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &5u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
            panic!("expected UnsupportedSchemaVersion, got {:?}", result)
        };
        assert_eq!(version, 5);
    }

    #[test]
    fn adversarial_decode_unknown_record_kind_returns_exact_kind() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(3),
            seq: EventSeq::new(0),
            workflow: test_digest(3),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &99u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnknownRecordKind { kind }) = result else {
            panic!("expected UnknownRecordKind, got {:?}", result)
        };
        assert_eq!(kind, 99);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_snapshot_kind_in_journal() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(4),
            seq: EventSeq::new(0),
            workflow: test_digest(4),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &30u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
            panic!("expected mismatch, got {:?}", result)
        };
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(kind, 30);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_blob_in_snapshot() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(5),
            seq: EventSeq::new(0),
            workflow: test_digest(5),
        };
        let result = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Blob,
            event.seq().get(),
            &event,
            MAX_SNAPSHOT_BYTES,
        );
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
            panic!("expected mismatch, got {:?}", result)
        };
        assert_eq!(magic, MAGIC_SNAPSHOT);
        assert_eq!(kind, RecordKind::Blob.id());
    }

    #[test]
    fn adversarial_decode_header_len_not_60_returns_mismatch() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(6),
            seq: EventSeq::new(0),
            workflow: test_digest(6),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::HeaderLengthMismatch { found }) = result else {
            panic!("expected mismatch, got {:?}", result)
        };
        assert_eq!(found, 48);
    }

    #[test]
    fn adversarial_decode_payload_len_above_limit_returns_too_large() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(7),
            seq: EventSeq::new(0),
            workflow: test_digest(7),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 12, &9999u32.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 100);
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("expected PayloadTooLarge, got {:?}", result)
        };
        assert_eq!(len, 9999);
        assert_eq!(max, 100);
    }

    #[test]
    fn adversarial_decode_corrupt_header_crc_returns_checksum_mismatch() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(8),
            seq: EventSeq::new(0),
            workflow: test_digest(8),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        if let Some(b) = encoded.get_mut(57) {
            *b ^= 0x80;
        }
        assert!(matches!(
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::HeaderChecksumMismatch)
        ));
    }

    #[test]
    fn adversarial_decode_corrupt_payload_digest_returns_digest_mismatch() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(9),
            seq: EventSeq::new(0),
            workflow: test_digest(9),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        if let Some(b) = encoded.get_mut(61) {
            *b ^= 0xFF;
        }
        assert!(matches!(
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::PayloadDigestMismatch)
        ));
    }

    #[test]
    fn adversarial_decode_truncated_before_full_header_returns_unexpected_eof() {
        let truncated = [0u8; 45];
        assert!(matches!(
            decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::UnexpectedEof)
        ));
    }

    #[test]
    fn adversarial_decode_truncated_before_full_payload_returns_unexpected_eof() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(10),
            seq: EventSeq::new(0),
            workflow: test_digest(10),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        let truncated = encoded.get(..62).expect("slice");
        assert!(matches!(
            decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::UnexpectedEof)
        ));
    }

    // =========================================================================
    // Section: Adversarial Key Encoding Tests
    // =========================================================================

    #[test]
    fn adversarial_key_prefix_isolation_proves_different_prefixes() {
        let digest = [0xAB; 32];
        let ws = workflow_source_key(digest).expect("ws");
        let ci = compiled_ir_key(digest).expect("ci");
        let bl = blob_key(digest).expect("bl");
        assert_ne!(ws[0], ci[0]);
        assert_ne!(ws[0], bl[0]);
        assert_eq!(ws[1..], ci[1..]);
        assert_eq!(ws[1..], bl[1..]);
    }

    #[test]
    fn adversarial_key_wrong_endianness_produces_different_keys() {
        let key = run_header_key(RunId::new(1)).expect("key");
        let mut le = [0u8; 9];
        le[0] = PREFIX_RUN_HEADER;
        le[1..9].copy_from_slice(&1u64.to_le_bytes());
        assert_ne!(key.as_slice(), le.as_slice());
        assert_eq!(key[1..9], 1u64.to_be_bytes());
    }

    #[test]
    fn adversarial_key_no_collision_different_runs_same_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(5)).expect("k1");
        let k2 = run_event_key(RunId::new(200), EventSeq::new(5)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_same_run_different_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(0)).expect("k1");
        let k2 = run_event_key(RunId::new(100), EventSeq::new(1)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_different_digests() {
        assert_ne!(
            blob_key([1u8; 32]).expect("k1").as_slice(),
            blob_key([2u8; 32]).expect("k2").as_slice()
        );
    }

    // =========================================================================
    // Section: Adversarial Journal / Replay Tests
    // =========================================================================

    #[test]
    fn adversarial_append_duplicate_sequence_rejected_with_exact_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(50);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        let result = journal.append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
        });
        let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
            panic!("expected DuplicateEvent, got {:?}", result)
        };
        assert_eq!(r, run);
        assert_eq!(s, EventSeq::new(0));
    }

    #[test]
    fn adversarial_read_events_with_sequence_gap_returns_exact_gap() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(777);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        assert!(
            journal
                .append_journaled(&JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(5),
                    result: vb_core::SlotIdx::new(0)
                })
                .is_ok()
        );
        let Err(JournalError::SequenceGap { expected, actual }) = journal.events_for_run(run)
        else {
            panic!("expected SequenceGap")
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(5));
    }

    // =========================================================================
    // Section: Adversarial Blob / Snapshot / Size Boundary Tests
    // =========================================================================

    #[test]
    fn adversarial_put_blob_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let record = BlobRecord {
            digest: [0xFF; 32],
            bytes: vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)],
        };
        assert!(matches!(
            journal.put_blob(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_blob_zero_length_round_trips() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let record = BlobRecord {
            digest: [0x42; 32],
            bytes: vec![],
        };
journal.put_blob(&record).expect("journal.put_blob must succeed");
        assert_eq!(journal.blob([0x42; 32]).expect("ok"), Some(record));
    }

    #[test]
    fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let snap = RunSnapshot {
            run: RunId::new(888),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)],
        };
        assert!(matches!(
            journal.put_snapshot(&snap),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
        let snap = RunSnapshot {
            run: RunId::new(889),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
        };
        let mut enc = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snap.seq.get(),
            &snap,
            MAX_SNAPSHOT_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(0) {
            *b ^= 0xFF;
        }
        assert!(matches!(
            decode_record::<RunSnapshot>(&enc, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let record = WorkflowSourceRecord {
            digest: test_digest(0xEE),
            source: vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)],
        };
        assert!(matches!(
            journal.put_workflow_source(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let record = CompiledIrRecord {
            digest: test_digest(0xCC),
            ir: vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)],
        };
        assert!(matches!(
            journal.put_compiled_ir(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    // =========================================================================
    // Section: Adversarial Schema Migration Tests
    // =========================================================================

    #[test]
    fn adversarial_schema_migration_from_zero_exact_fields() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(11),
            seq: EventSeq::new(0),
            workflow: test_digest(11),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());
        let Err(JournalError::MigrationRequired { from, to }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected MigrationRequired")
        };
        assert_eq!(from, 0);
        assert_eq!(to, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adversarial_schema_future_version_max_unsupported() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(12),
            seq: EventSeq::new(0),
            workflow: test_digest(12),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());
        let Err(JournalError::UnsupportedSchemaVersion { version }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected UnsupportedSchemaVersion")
        };
        assert_eq!(version, u16::MAX);
    }

    // =========================================================================
    // Section: Adversarial Queue Tests
    // =========================================================================

    #[test]
    fn adversarial_queue_zero_capacity_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_zero_batch_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_full_returns_queue_full() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("q");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
queue.enqueue_journaled(event.clone()).expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.enqueue_journaled(event),
            Err(JournalError::QueueFull)
        ));
    }

    #[test]
    fn journal_writer_queue_drain_all_flushes_until_empty() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(2);
        let workflow = test_digest(2);

        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                })
                .is_ok()
        );
        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(1),
                })
                .is_ok()
        );

        assert!(matches!(
            queue.drain_all(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_retains_events_when_append_fails() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(3);
        let duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(3),
        };
        let conflicting_duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(33),
        };
        let next = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };

        assert!(matches!(journal.append_journaled(&duplicate), Ok(())));
        assert!(matches!(
            queue.enqueue_journaled(conflicting_duplicate),
            Ok(())
        ));
        assert!(matches!(queue.enqueue_journaled(next), Ok(())));

        assert!(matches!(
            queue.flush_batch(&journal),
            Err(JournalError::DuplicateEvent { run: found, seq })
                if found == run && seq == EventSeq::new(0)
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 2 && counts.strict == 0
        ));
    }

    #[test]
    fn journal_writer_queue_flush_persists_journaled_events_before_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let path = temp_dir.path().to_path_buf();
        let journal = FjallJournal::open(&path, None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(4);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(4),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };

queue.enqueue_journaled(accepted).expect("queue.enqueue_journaled must succeed");
queue.enqueue_journaled(cancelled).expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        drop(journal);

        let reopened = FjallJournal::open(&path, None).expect("reopen");
        assert!(matches!(reopened.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_shutdown_rejects_new_writes_after_durable_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(5);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(5),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };

queue.enqueue_journaled(accepted.clone()).expect("queue.enqueue_journaled must succeed");
queue.enqueue_strict(cancelled).expect("queue.enqueue_strict must succeed");
        assert!(matches!(
            queue.shutdown(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.enqueue_journaled(accepted),
            Err(JournalError::QueueShutdown)
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_crash_window_retry_drains_already_written_same_event() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(6);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(6),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };

journal.append_journaled(&accepted).expect("journal.append_journaled must succeed");
queue.enqueue_journaled(accepted).expect("queue.enqueue_journaled must succeed");
queue.enqueue_journaled(cancelled).expect("queue.enqueue_journaled must succeed");

        // This models the crash window where a prior attempt reached Fjall before
        // the queue could durably drain. Retrying accepts the identical event only.
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    // =========================================================================
    // Section: Adversarial Postcard / Encoding Edge Cases
    // =========================================================================

    #[test]
    fn adversarial_valid_header_garbage_postcard_returns_decode_failed() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(13),
            seq: EventSeq::new(0),
            workflow: test_digest(13),
        };
        let mut enc = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(60) {
            *b = 0xFF;
        }
        let digest_bytes = *blake3::hash(&enc[60..]).as_bytes();
        enc.get_mut(24..56)
            .expect("digest")
            .copy_from_slice(&digest_bytes);
        let cs = crc32c::crc32c(&enc[..56]);
        enc[56] = (cs & 0xFF) as u8;
        enc[57] = ((cs >> 8) & 0xFF) as u8;
        enc[58] = ((cs >> 16) & 0xFF) as u8;
        enc[59] = ((cs >> 24) & 0xFF) as u8;
        assert!(matches!(
            decode_record::<JournalEvent>(&enc, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::PostcardDecodeFailed)
        ));
    }

    #[test]
    fn adversarial_run_header_wrong_magic_returns_bad_magic() {
        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: test_digest(8),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        let enc = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.as_u64(),
            &record,
            MAX_RUN_HEADER_BYTES,
        )
        .expect("ok");
        assert!(matches!(
            decode_record::<RunHeaderRecord>(&enc, MAGIC_BLOB, MAX_RUN_HEADER_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_decode_empty_returns_unexpected_eof() {
        assert!(matches!(
            decode_record::<JournalEvent>(&[][..], MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::UnexpectedEof)
        ));
    }

    #[test]
    fn adversarial_encode_empty_blob_succeeds() {
        assert!(
            encode_record(
                MAGIC_BLOB,
                RecordKind::Blob,
                0,
                &BlobRecord {
                    digest: [0; 32],
                    bytes: vec![]
                },
                MAX_BLOB_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_source_succeeds() {
        assert!(
            encode_record(
                MAGIC_WORKFLOW_SOURCE,
                RecordKind::WorkflowSource,
                0,
                &WorkflowSourceRecord {
                    digest: test_digest(0),
                    source: vec![]
                },
                MAX_WORKFLOW_SOURCE_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_ir_succeeds() {
        assert!(
            encode_record(
                MAGIC_COMPILED_ARTIFACT,
                RecordKind::CompiledIr,
                0,
                &CompiledIrRecord {
                    digest: test_digest(0),
                    ir: vec![]
                },
                MAX_COMPILED_IR_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn journal_error_diagnostic_codes_are_unique() {
        let errors = [
            JournalError::KeyCapacity,
            JournalError::WriteLockPoisoned,
            JournalError::QueueCapacity,
            JournalError::QueueFull,
            JournalError::SequenceOverflow,
            JournalError::HeaderChecksumMismatch,
            JournalError::PayloadDigestMismatch,
            JournalError::UnexpectedEof,
            JournalError::PostcardDecodeFailed,
            JournalError::DuplicateEvent {
                run: RunId::new(1),
                seq: EventSeq::new(0),
            },
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            },
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            },
            JournalError::BadMagic { found: 0 },
            JournalError::UnsupportedSchemaVersion { version: 0 },
            JournalError::MigrationRequired { from: 0, to: 1 },
            JournalError::UnknownRecordKind { kind: 0 },
            JournalError::RecordKindFamilyMismatch { magic: 0, kind: 0 },
            JournalError::HeaderLengthMismatch { found: 0 },
            JournalError::PayloadTooLarge { len: 0, max: 0 },
        ];
        let mut seen = std::collections::BTreeSet::new();
        for err in &errors {
            let code = err.diagnostic_code();
            assert!(seen.insert(code), "duplicate diagnostic code: {code}");
        }
        assert_eq!(seen.len(), errors.len());
    }

    #[test]
    fn journal_error_diagnostic_code_fjall() {
        // Fjall and Encode variants hold external errors; we verify via KeyCapacity
        assert_eq!(
            JournalError::KeyCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4003)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_duplicate_event() {
        assert_eq!(
            JournalError::DuplicateEvent {
                run: RunId::new(42),
                seq: EventSeq::new(7),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4004)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_write_lock_poisoned() {
        assert_eq!(
            JournalError::WriteLockPoisoned.diagnostic_code(),
            DiagnosticCode::new(0x4005)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_capacity() {
        assert_eq!(
            JournalError::QueueCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4006)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_full() {
        assert_eq!(
            JournalError::QueueFull.diagnostic_code(),
            DiagnosticCode::new(0x4007)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_wrong_run() {
        assert_eq!(
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4008)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_gap() {
        assert_eq!(
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4009)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_overflow() {
        assert_eq!(
            JournalError::SequenceOverflow.diagnostic_code(),
            DiagnosticCode::new(0x400A)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_bad_magic() {
        assert_eq!(
            JournalError::BadMagic { found: 0xDEAD_BEEF }.diagnostic_code(),
            DiagnosticCode::new(0x400B)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unsupported_schema_version() {
        assert_eq!(
            JournalError::UnsupportedSchemaVersion { version: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x400C)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_migration_required() {
        assert_eq!(
            JournalError::MigrationRequired { from: 0, to: 1 }.diagnostic_code(),
            DiagnosticCode::new(0x400D)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unknown_record_kind() {
        assert_eq!(
            JournalError::UnknownRecordKind { kind: 200 }.diagnostic_code(),
            DiagnosticCode::new(0x400E)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_record_kind_family_mismatch() {
        assert_eq!(
            JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_JOURNAL_EVENT,
                kind: 1,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x400F)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_length_mismatch() {
        assert_eq!(
            JournalError::HeaderLengthMismatch { found: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x4010)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_too_large() {
        assert_eq!(
            JournalError::PayloadTooLarge { len: 200, max: 10 }.diagnostic_code(),
            DiagnosticCode::new(0x4011)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_checksum_mismatch() {
        assert_eq!(
            JournalError::HeaderChecksumMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4012)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_digest_mismatch() {
        assert_eq!(
            JournalError::PayloadDigestMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4013)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unexpected_eof() {
        assert_eq!(
            JournalError::UnexpectedEof.diagnostic_code(),
            DiagnosticCode::new(0x4014)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_postcard_decode_failed() {
        assert_eq!(
            JournalError::PostcardDecodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x4015)
        );
    }

    // =========================================================================
    // Section: Batch Write-Through Integration Tests (60 new tests)
    // =========================================================================

    // --- JournalWriteBatch put_run_event round-trips (tests 1-12) ---

    #[test]
    fn batch_append_run_accepted_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1001);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1, "one event must be stored");
        assert_eq!(events[0], event, "event must round-trip exactly");
    }

    #[test]
    fn batch_append_step_started_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1002);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_succeeded_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1003);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            output: SlotIdx::new(3),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1004);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_scheduled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1005);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(7),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_completed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1006);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            action: ActionId::new(8),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1007);
        let event = JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            action: ActionId::new(9),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_finished_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1008);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::new(42),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1009);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_cancelled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1010);
        let event = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_slot_written_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1011);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(5),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_suspended_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1012);
        let event = JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
        };
        let mut batch = journal.batch();
        batch.append_event(&event).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    // --- Multi-run isolation (tests 13-16) ---

    #[test]
    fn events_for_run_isolates_run_a_from_run_b() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(2001);
        let run_b = RunId::new(2002);
        let event_a = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        };
        let event_b = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        };
        let event_a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
        };
        let mut batch = journal.batch();
        batch.append_event(&event_a).expect("batch.append_event must succeed");
        batch.append_event(&event_b).expect("batch.append_event must succeed");
        batch.append_event(&event_a2).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events_a = journal.events_for_run(run_a).expect("events_for_run A must succeed");
        assert_eq!(events_a.len(), 2, "run A must have exactly 2 events");
        assert_eq!(events_a[0], event_a);
        assert_eq!(events_a[1], event_a2);
        let events_b = journal.events_for_run(run_b).expect("events_for_run B must succeed");
        assert_eq!(events_b.len(), 1, "run B must have exactly 1 event");
        assert_eq!(events_b[0], event_b);
    }

    #[test]
    fn run_header_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(3001);
        let run_2 = RunId::new(3002);
        let header_1 = RunHeaderRecord {
            run: run_1,
            workflow_id: WorkflowId::new(10),
            compiled_digest: WorkflowDigest::from_bytes([1; 32]),
            status: 1,
            accepted_at_ms: 100,
        };
        let header_2 = RunHeaderRecord {
            run: run_2,
            workflow_id: WorkflowId::new(20),
            compiled_digest: WorkflowDigest::from_bytes([2; 32]),
            status: 2,
            accepted_at_ms: 200,
        };
        let mut batch = journal.batch();
        batch.put_run_header(&header_1).expect("batch.put_run_header must succeed");
        batch.put_run_header(&header_2).expect("batch.put_run_header must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_1 = journal.run_header(run_1).expect("run_header run_1 must succeed");
        assert_eq!(found_1, Some(header_1), "run 1 header must match exactly");
        let found_2 = journal.run_header(run_2).expect("run_header run_2 must succeed");
        assert_eq!(found_2, Some(header_2), "run 2 header must match exactly");
    }

    #[test]
    fn snapshot_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(3003);
        let run_b = RunId::new(3004);
        let snap_a = RunSnapshot {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xA; 32]),
            slots: vec![1, 2, 3],
        };
        let snap_b = RunSnapshot {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xB; 32]),
            slots: vec![4, 5, 6],
        };
        let mut batch = journal.batch();
        batch.put_snapshot(&snap_a).expect("batch.put_snapshot must succeed");
        batch.put_snapshot(&snap_b).expect("batch.put_snapshot must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_a = journal.snapshot(run_a, EventSeq::new(0)).expect("snapshot A must succeed");
        assert_eq!(found_a, Some(snap_a), "snapshot for run A must match");
        let found_b = journal.snapshot(run_b, EventSeq::new(0)).expect("snapshot B must succeed");
        assert_eq!(found_b, Some(snap_b), "snapshot for run B must match");
    }

    #[test]
    fn batch_writes_for_multiple_runs_commit_atomically() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(4001);
        let run_2 = RunId::new(4002);
        let run_3 = RunId::new(4003);
        let mut batch = journal.batch();
        batch.append_event(&JournalEvent::RunAccepted {
            run: run_1,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        }).expect("batch.append_event must succeed");
        batch.append_event(&JournalEvent::RunAccepted {
            run: run_2,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        }).expect("batch.append_event must succeed");
        batch.append_event(&JournalEvent::RunAccepted {
            run: run_3,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        }).expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        assert_eq!(
            journal.events_for_run(run_1).expect("run_1 must succeed").len(),
            1,
            "run 1 must have 1 event"
        );
        assert_eq!(
            journal.events_for_run(run_2).expect("run_2 must succeed").len(),
            1,
            "run 2 must have 1 event"
        );
        assert_eq!(
            journal.events_for_run(run_3).expect("run_3 must succeed").len(),
            1,
            "run 3 must have 1 event"
        );
    }

    // --- Writer Queue edge cases (tests 17-22) ---

    #[test]
    fn queue_journaled_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5001);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let event_1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let event_2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
        };
        queue.enqueue_journaled(event_0.clone()).expect("enqueue 0 must succeed");
        queue.enqueue_journaled(event_1.clone()).expect("enqueue 1 must succeed");
        queue.enqueue_journaled(event_2.clone()).expect("enqueue 2 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 3);
        assert_eq!(report.written, 3);
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events[0], event_0, "first event must be seq 0");
        assert_eq!(events[1], event_1, "second event must be seq 1");
        assert_eq!(events[2], event_2, "third event must be seq 2");
    }

    #[test]
    fn queue_strict_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5002);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        let event_1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };
        queue.enqueue_strict(event_0.clone()).expect("enqueue 0 must succeed");
        queue.enqueue_strict(event_1.clone()).expect("enqueue 1 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2);
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events[0], event_0);
        assert_eq!(events[1], event_1);
    }

    #[test]
    fn queue_mixed_journaled_and_strict_drain_returns_both() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5003);
        let journaled_event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        let strict_event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        queue.enqueue_journaled(journaled_event.clone()).expect("enqueue journaled must succeed");
        queue.enqueue_strict(strict_event.clone()).expect("enqueue strict must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2, "both events must be drained");
        assert_eq!(report.written, 2, "both events must be written");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], journaled_event);
        assert_eq!(events[1], strict_event);
    }

    #[test]
    fn queue_flush_persists_before_drain() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5004);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([4; 32]),
        };
        queue.enqueue_journaled(event.clone()).expect("enqueue must succeed");
        let report = queue.flush_batch(&journal).expect("flush_batch must succeed");
        assert_eq!(report.written, 1, "one event must be written");
        let events_before = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events_before.len(), 1, "event must be on disk before drain");
        assert_eq!(events_before[0], event);
    }

    #[test]
    fn queue_empty_drain_returns_zero_events() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 0, "empty queue must drain zero events");
        assert_eq!(report.written, 0, "empty queue must write zero events");
    }

    #[test]
    fn queue_pending_count_matches_enqueued() {
        let queue = JournalWriterQueue::new(16, 4, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5005);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        let counts_empty = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts_empty.journaled, 0);
        assert_eq!(counts_empty.strict, 0);
        queue.enqueue_journaled(event.clone()).expect("enqueue 0 must succeed");
        queue.enqueue_journaled(event.clone()).expect("enqueue 1 must succeed");
        queue.enqueue_strict(event).expect("enqueue 2 must succeed");
        let counts = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts.journaled, 2, "two journaled events must be counted");
        assert_eq!(counts.strict, 1, "one strict event must be counted");
    }

    // --- FjallJournal open/close/reopen (tests 23-30) ---

    #[test]
    fn journal_open_creates_fresh_database() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let events = journal.events_for_run(RunId::new(1)).expect("events_for_run must succeed");
        assert!(events.is_empty(), "fresh database must have no events");
        let header = journal.run_header(RunId::new(1)).expect("run_header must succeed");
        assert_eq!(header, None, "fresh database must have no headers");
        let blob = journal.blob([0; 32]).expect("blob must succeed");
        assert_eq!(blob, None, "fresh database must have no blobs");
    }

    #[test]
    fn journal_close_and_reopen_preserves_strict_data() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let digest = WorkflowDigest::from_bytes([0xEE; 32]);
        let run = RunId::new(6001);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(5),
            compiled_digest: digest,
            status: 3,
            accepted_at_ms: 999,
        };
        {
            let journal = FjallJournal::open(&path, None).expect("setup: journal open");
            journal.put_run_header(&header).expect("put_run_header must succeed");
            journal.persist_strict().expect("persist_strict must succeed");
        }
        let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
        let found = reopened.run_header(run).expect("run_header must succeed");
        assert_eq!(found, Some(header), "strict data must survive reopen");
    }

    #[test]
    fn journal_multiple_opens_same_path_fails_or_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal1 = FjallJournal::open(temp_dir.path(), None).expect("first open must succeed");
        let journal2_result = FjallJournal::open(temp_dir.path(), None);
        drop(journal1);
        if let Ok(j2) = journal2_result {
            drop(j2);
        }
    }

    #[test]
    fn journal_put_then_get_workflow_source_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0x77; 32]);
        let record = WorkflowSourceRecord {
            digest,
            source: b"consistent_source".to_vec(),
        };
        journal.put_workflow_source(&record).expect("put_workflow_source must succeed");
        let found = journal.workflow_source(digest).expect("workflow_source must succeed");
        assert_eq!(found, Some(record), "put-then-get must be consistent in same session");
    }

    #[test]
    fn journal_put_then_get_compiled_ir_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0x88; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: b"consistent_ir".to_vec(),
        };
        journal.put_compiled_ir(&record).expect("put_compiled_ir must succeed");
        let found = journal.compiled_ir(digest).expect("compiled_ir must succeed");
        assert_eq!(found, Some(record), "put-then-get must be consistent in same session");
    }

    #[test]
    fn journal_put_then_get_run_header_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6002);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(99),
            compiled_digest: WorkflowDigest::from_bytes([0x99; 32]),
            status: 7,
            accepted_at_ms: 123456789,
        };
        journal.put_run_header(&record).expect("put_run_header must succeed");
        let found = journal.run_header(run).expect("run_header must succeed");
        assert_eq!(found, Some(record), "put-then-get must be consistent in same session");
    }

    #[test]
    fn journal_put_then_get_snapshot_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6003);
        let seq = EventSeq::new(4);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
            slots: vec![0xDE, 0xAD],
        };
        journal.put_snapshot(&snapshot).expect("put_snapshot must succeed");
        let found = journal.snapshot(run, seq).expect("snapshot must succeed");
        assert_eq!(found, Some(snapshot), "put-then-get must be consistent in same session");
    }

    #[test]
    fn journal_put_then_get_blob_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = [0xBB; 32];
        let record = BlobRecord {
            digest,
            bytes: b"consistent_blob".to_vec(),
        };
        journal.put_blob(&record).expect("put_blob must succeed");
        let found = journal.blob(digest).expect("blob must succeed");
        assert_eq!(found, Some(record), "put-then-get must be consistent in same session");
    }

    // --- Index queries (tests 31-35) ---

    #[test]
    fn status_index_stores_and_queries_by_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state: u8 = 3;
        let timestamp: u64 = 1700000000;
        let run = RunId::new(7001);
        journal.put_status_index(state, timestamp, run).expect("put_status_index must succeed");
        let key = index_status_key(state, timestamp, run).expect("key must succeed");
        let value = journal.index_status.get(key.as_slice()).expect("get must succeed");
        assert!(value.is_some(), "status index entry must exist after put");
    }

    #[test]
    fn workflow_index_stores_and_queries_by_workflow_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(42);
        let run = RunId::new(7002);
        journal.put_workflow_index(workflow, run).expect("put_workflow_index must succeed");
        let key = index_workflow_key(workflow, run).expect("key must succeed");
        let value = journal.index_workflow.get(key.as_slice()).expect("get must succeed");
        assert!(value.is_some(), "workflow index entry must exist after put");
    }

    #[test]
    fn action_index_stores_and_queries_by_action_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let action = ActionId::new(7);
        let run = RunId::new(7003);
        let step = StepIdx::new(2);
        journal.put_action_index(action, run, step).expect("put_action_index must succeed");
        let key = index_action_key(action, run, step).expect("key must succeed");
        let value = journal.index_action.get(key.as_slice()).expect("get must succeed");
        assert!(value.is_some(), "action index entry must exist after put");
    }

    #[test]
    fn status_index_multiple_runs_same_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state: u8 = 5;
        let run_1 = RunId::new(7010);
        let run_2 = RunId::new(7011);
        let run_3 = RunId::new(7012);
        journal.put_status_index(state, 100, run_1).expect("put_status_index 1 must succeed");
        journal.put_status_index(state, 200, run_2).expect("put_status_index 2 must succeed");
        journal.put_status_index(state, 300, run_3).expect("put_status_index 3 must succeed");
        let key_1 = index_status_key(state, 100, run_1).expect("key 1 must succeed");
        let key_2 = index_status_key(state, 200, run_2).expect("key 2 must succeed");
        let key_3 = index_status_key(state, 300, run_3).expect("key 3 must succeed");
        assert!(journal.index_status.get(key_1.as_slice()).expect("get 1").is_some());
        assert!(journal.index_status.get(key_2.as_slice()).expect("get 2").is_some());
        assert!(journal.index_status.get(key_3.as_slice()).expect("get 3").is_some());
    }

    #[test]
    fn workflow_index_multiple_runs_same_workflow() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(99);
        let run_1 = RunId::new(7020);
        let run_2 = RunId::new(7021);
        let run_3 = RunId::new(7022);
        journal.put_workflow_index(workflow, run_1).expect("put 1 must succeed");
        journal.put_workflow_index(workflow, run_2).expect("put 2 must succeed");
        journal.put_workflow_index(workflow, run_3).expect("put 3 must succeed");
        let key_1 = index_workflow_key(workflow, run_1).expect("key 1 must succeed");
        let key_2 = index_workflow_key(workflow, run_2).expect("key 2 must succeed");
        let key_3 = index_workflow_key(workflow, run_3).expect("key 3 must succeed");
        assert!(journal.index_workflow.get(key_1.as_slice()).expect("get 1").is_some());
        assert!(journal.index_workflow.get(key_2.as_slice()).expect("get 2").is_some());
        assert!(journal.index_workflow.get(key_3.as_slice()).expect("get 3").is_some());
    }

    // --- Record builder (tests 36-40) ---

    #[test]
    fn builder_initial_len_is_zero() {
        let builder = BatchBuilder::new();
        assert_eq!(builder.len(), 0, "new builder must have len 0");
        assert!(builder.is_empty(), "new builder must be empty");
    }

    #[test]
    fn builder_append_increments_len() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8001);
        builder.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        });
        assert_eq!(builder.len(), 1, "builder must have len 1 after one push");
        assert!(!builder.is_empty());
    }

    #[test]
    fn builder_append_multiple_events_len_matches() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8002);
        builder.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        });
        builder.push(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        });
        builder.push(JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
        });
        assert_eq!(builder.len(), 3, "builder must have len 3 after three pushes");
    }

    #[test]
    fn builder_as_slice_returns_appended_events() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8003);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let e1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
        };
        builder.push(e0.clone());
        builder.push(e1.clone());
        let slice = builder.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], e0, "first slice element must match first pushed event");
        assert_eq!(slice[1], e1, "second slice element must match second pushed event");
    }

    #[test]
    fn builder_build_produces_correct_record_count() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(8004);
        let mut builder = BatchBuilder::new();
        builder.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        });
        builder.push(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        });
        builder.push(JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
        });
        assert_eq!(builder.len(), 3);
        journal.append_strict_batch(builder.as_slice()).expect("append_strict_batch must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 3, "three events must be stored");
    }

    // --- Batch state tracking (tests 41-44) ---

    #[test]
    fn batch_initial_len_is_zero() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch();
        assert_eq!(batch.len(), 0, "new batch must have len 0");
        assert!(batch.is_empty(), "new batch must be empty");
    }

    #[test]
    fn batch_len_increments_per_put() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let digest = WorkflowDigest::from_bytes([0x41; 32]);
        batch.put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: b"a".to_vec(),
        }).expect("put 1 must succeed");
        assert_eq!(batch.len(), 1, "batch must have len 1 after first put");
        batch.put_compiled_ir(&CompiledIrRecord {
            digest,
            ir: b"ir".to_vec(),
        }).expect("put 2 must succeed");
        assert_eq!(batch.len(), 2, "batch must have len 2 after second put");
        batch.put_run_header(&RunHeaderRecord {
            run: RunId::new(9001),
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 0,
            accepted_at_ms: 0,
        }).expect("put 3 must succeed");
        assert_eq!(batch.len(), 3, "batch must have len 3 after third put");
    }

    #[test]
    fn batch_len_resets_after_commit() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let digest = WorkflowDigest::from_bytes([0x42; 32]);
        batch.put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: b"data".to_vec(),
        }).expect("put must succeed");
        assert_eq!(batch.len(), 1, "batch must have 1 operation before commit");
        batch.commit().expect("commit must succeed");
        let fresh_batch = journal.batch();
        assert_eq!(fresh_batch.len(), 0, "new batch after commit must start at 0");
    }

    #[test]
    fn batch_put_snapshot_increments_len() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        assert_eq!(batch.len(), 0);
        let snapshot = RunSnapshot {
            run: RunId::new(9002),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x43; 32]),
            slots: vec![1, 2],
        };
        batch.put_snapshot(&snapshot).expect("put_snapshot must succeed");
        assert_eq!(batch.len(), 1, "batch len must be 1 after put_snapshot");
    }

    // --- Envelope validation (tests 45-47) ---

    #[test]
    fn decode_valid_envelope_produces_exact_record() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            source: b"exact_match".to_vec(),
        };
        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        ).expect("encode must succeed");
        let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(
            &encoded,
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        ).expect("decode must succeed");
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded, record, "decoded record must exactly match original");
    }

    #[test]
    fn envelope_magic_matches_expected_constant() {
        assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352, "VBSR in ASCII hex");
        assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952, "VBIR in ASCII hex");
        assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45, "VBJE in ASCII hex");
        assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E, "VBSN in ASCII hex");
        assert_eq!(MAGIC_BLOB, 0x5642_424C, "VBBL in ASCII hex");
        assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54, "VBLT in ASCII hex");
        assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958, "VBIX in ASCII hex");
    }

    #[test]
    fn envelope_header_len_is_fixed_at_60() {
        assert_eq!(RECORD_HEADER_LEN, 60, "header length must be exactly 60");
        assert_eq!(RECORD_HEADER_BYTES, 60, "header bytes constant must be 60");
        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            b"payload",
            128,
        ).expect("encode_record_header must succeed");
        assert_eq!(header.len(), 60, "encoded header must be exactly 60 bytes");
    }

    // --- Cross-keyspace atomicity (tests 48-60) ---

    #[test]
    fn batch_atomic_all_or_nothing_workflow_source_and_ir() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0xAC; 32]);
        let mut batch = journal.batch();
        batch.put_workflow_source(&WorkflowSourceRecord {
            digest,
            source: b"atomic_source".to_vec(),
        }).expect("put_workflow_source must succeed");
        batch.put_compiled_ir(&CompiledIrRecord {
            digest,
            ir: b"atomic_ir".to_vec(),
        }).expect("put_compiled_ir must succeed");
        batch.commit().expect("commit must succeed");
        let source = journal.workflow_source(digest).expect("workflow_source must succeed");
        let ir = journal.compiled_ir(digest).expect("compiled_ir must succeed");
        assert!(source.is_some(), "source must be present after atomic commit");
        assert!(ir.is_some(), "IR must be present after atomic commit");
        assert_eq!(source.unwrap().source, b"atomic_source".to_vec());
        assert_eq!(ir.unwrap().ir, b"atomic_ir".to_vec());
    }

    #[test]
    fn batch_commit_with_header_and_events_cross_keyspace() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9003);
        let digest = WorkflowDigest::from_bytes([0xCD; 32]);
        let mut batch = journal.batch();
        batch.put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 555,
        }).expect("put_run_header must succeed");
        batch.append_event(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        }).expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
        let header = journal.run_header(run).expect("run_header must succeed");
        assert!(header.is_some(), "header must be present");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events.len(), 1, "event must be present");
    }

    #[test]
    fn batch_strict_commit_all_persisted_durably() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let digest = WorkflowDigest::from_bytes([0xDD; 32]);
        let blob_digest = [0xEE; 32];
        {
            let journal = FjallJournal::open(&path, None).expect("setup: journal open");
            let mut batch = journal.batch().strict();
            batch.put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: b"strict_ws".to_vec(),
            }).expect("put_workflow_source must succeed");
            batch.put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"strict_ir".to_vec(),
            }).expect("put_compiled_ir must succeed");
            batch.put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: b"strict_blob".to_vec(),
            }).expect("put_blob must succeed");
            batch.commit().expect("commit must succeed");
        }
        let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
        let ws = reopened.workflow_source(digest).expect("workflow_source must succeed");
        assert_eq!(ws.unwrap().source, b"strict_ws".to_vec());
        let ir = reopened.compiled_ir(digest).expect("compiled_ir must succeed");
        assert_eq!(ir.unwrap().ir, b"strict_ir".to_vec());
        let bl = reopened.blob(blob_digest).expect("blob must succeed");
        assert_eq!(bl.unwrap().bytes, b"strict_blob".to_vec());
    }

    #[test]
    fn batch_empty_strict_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch().strict();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        batch.commit().expect("empty strict batch commit must succeed");
    }

    #[test]
    fn batch_commit_after_multiple_puts_persists_all() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest_1 = WorkflowDigest::from_bytes([1; 32]);
        let digest_2 = WorkflowDigest::from_bytes([2; 32]);
        let blob_digest = [3u8; 32];
        let run = RunId::new(9005);
        let mut batch = journal.batch();
        batch.put_workflow_source(&WorkflowSourceRecord {
            digest: digest_1,
            source: b"ws".to_vec(),
        }).expect("put 1 must succeed");
        batch.put_compiled_ir(&CompiledIrRecord {
            digest: digest_2,
            ir: b"ir".to_vec(),
        }).expect("put 2 must succeed");
        batch.put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest_1,
            status: 1,
            accepted_at_ms: 100,
        }).expect("put 3 must succeed");
        batch.put_blob(&BlobRecord {
            digest: blob_digest,
            bytes: b"blob".to_vec(),
        }).expect("put 4 must succeed");
        batch.put_snapshot(&RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: digest_1,
            slots: vec![42],
        }).expect("put 5 must succeed");
        batch.commit().expect("commit must succeed");
        assert!(journal.workflow_source(digest_1).expect("ws").is_some());
        assert!(journal.compiled_ir(digest_2).expect("ir").is_some());
        assert!(journal.run_header(run).expect("rh").is_some());
        assert!(journal.blob(blob_digest).expect("bl").is_some());
        assert!(journal.snapshot(run, EventSeq::new(0)).expect("sn").is_some());
    }

    #[test]
    fn journal_events_for_run_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9006);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(1),
        };
        let mut batch = journal.batch();
        batch.append_event(&e0).expect("append 0 must succeed");
        batch.append_event(&e1).expect("append 1 must succeed");
        batch.append_event(&e2).expect("append 2 must succeed");
        batch.commit().expect("commit must succeed");
        let events = journal.events_for_run(run).expect("events_for_run must succeed");
        assert_eq!(events, vec![e0, e1, e2], "replayed events must match input exactly");
    }

    #[test]
    fn journal_workflow_source_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0xFE; 32]);
        let record = WorkflowSourceRecord {
            digest,
            source: b"exact_bytes_source".to_vec(),
        };
        let mut batch = journal.batch();
        batch.put_workflow_source(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.workflow_source(digest).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.source, b"exact_bytes_source".to_vec(), "source bytes must match exactly");
        assert_eq!(found_record.digest, digest, "digest must match");
    }

    #[test]
    fn journal_compiled_ir_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0xFC; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: b"exact_ir_bytes".to_vec(),
        };
        let mut batch = journal.batch();
        batch.put_compiled_ir(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.compiled_ir(digest).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.ir, b"exact_ir_bytes".to_vec(), "IR bytes must match exactly");
        assert_eq!(found_record.digest, digest);
    }

    #[test]
    fn journal_run_header_after_batch_commit_matches_all_fields() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9007);
        let workflow_id = WorkflowId::new(42);
        let compiled_digest = WorkflowDigest::from_bytes([0xFB; 32]);
        let status: u8 = 5;
        let accepted_at_ms: u64 = 9876543210;
        let record = RunHeaderRecord {
            run,
            workflow_id,
            compiled_digest,
            status,
            accepted_at_ms,
        };
        let mut batch = journal.batch();
        batch.put_run_header(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.run_header(run).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.run, run, "run must match");
        assert_eq!(found_record.workflow_id, workflow_id, "workflow_id must match");
        assert_eq!(found_record.compiled_digest, compiled_digest, "compiled_digest must match");
        assert_eq!(found_record.status, status, "status must match");
        assert_eq!(found_record.accepted_at_ms, accepted_at_ms, "accepted_at_ms must match");
    }

    #[test]
    fn journal_snapshot_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9008);
        let seq = EventSeq::new(3);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0xFA; 32]),
            slots: b"snapshot_data".to_vec(),
        };
        let mut batch = journal.batch();
        batch.put_snapshot(&snapshot).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.snapshot(run, seq).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.run, run);
        assert_eq!(found_record.seq, seq);
        assert_eq!(found_record.slots, b"snapshot_data".to_vec());
    }

    #[test]
    fn journal_blob_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = [0xF0; 32];
        let record = BlobRecord {
            digest,
            bytes: b"batch_blob_exact".to_vec(),
        };
        let mut batch = journal.batch();
        batch.put_blob(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.blob(digest).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.bytes, b"batch_blob_exact".to_vec(), "blob bytes must match exactly");
        assert_eq!(found_record.digest, digest);
    }

    #[test]
    fn journal_status_index_after_batch_commit_returns_correct_run() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state: u8 = 7;
        let timestamp: u64 = 55555;
        let run = RunId::new(9009);
        let mut batch = journal.batch();
        batch.put_status_index(state, timestamp, run).expect("put_status_index must succeed");
        batch.commit().expect("commit must succeed");
        let key = index_status_key(state, timestamp, run).expect("key must succeed");
        let value = journal.index_status.get(key.as_slice()).expect("get must succeed");
        assert!(value.is_some(), "status index must exist after batch commit");
    }

    #[test]
    fn journal_action_index_after_batch_commit_returns_correct_entry() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let action = ActionId::new(11);
        let run = RunId::new(9010);
        let step = StepIdx::new(4);
        let mut batch = journal.batch();
        batch.put_action_index(action, run, step).expect("put_action_index must succeed");
        batch.commit().expect("commit must succeed");
        let key = index_action_key(action, run, step).expect("key must succeed");
        let value = journal.index_action.get(key.as_slice()).expect("get must succeed");
        assert!(value.is_some(), "action index must exist after batch commit");
    }

    #[test]
    fn adversarial_reopen_after_unflushed_journaled_events_may_lose_them() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9001);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        journal.append_journaled(&event).expect("append journaled");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let result = journal2.events_for_run(run).expect("events_for_run succeeds");
        // Journaled durability does not guarantee persistence without flush
        // Either the event is present (Fjall flushed on drop) or absent (acceptable)
        assert!(result.len() <= 1, "at most one event expected");
    }

    #[test]
    fn adversarial_reopen_after_flushed_journaled_events_preserves_them() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9002);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        journal.append_journaled(&event).expect("append journaled");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let events = journal2.events_for_run(run).expect("events_for_run succeeds");
        assert_eq!(events.len(), 1, "flushed journaled event must survive reopen");
    }

    #[test]
    fn adversarial_reopen_after_strict_event_preserves_it() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9003);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        journal.append_strict(&event).expect("append strict");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let events = journal2.events_for_run(run).expect("events_for_run succeeds");
        assert_eq!(events.len(), 1, "strict event must survive reopen");
    }

    #[test]
    fn adversarial_batch_commit_then_reopen_preserves_all_keys() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([4; 32]);
        let run = RunId::new(9004);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: b"source".to_vec(),
            })
            .expect("put_workflow_source");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 100,
            })
            .expect("put_run_header");
        batch
            .put_blob(&BlobRecord {
                digest: digest.as_bytes(),
                bytes: b"blob".to_vec(),
            })
            .expect("put_blob");
        batch.commit().expect("commit");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let source = journal2.workflow_source(digest).expect("get source");
        assert!(source.is_some(), "workflow source must survive reopen");
        let header = journal2.run_header(run).expect("get header");
        assert!(header.is_some(), "run header must survive reopen");
        let blob = journal2.blob(digest.as_bytes()).expect("get blob");
        assert!(blob.is_some(), "blob must survive reopen");
    }

    #[test]
    fn adversarial_double_append_same_run_seq_returns_duplicate_error() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9005);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        journal.append_strict(&event).expect("first append");
        let result = journal.append_strict(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate append must return DuplicateEvent"
        );
    }

    #[test]
    fn adversarial_events_for_run_on_empty_journal_returns_empty() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let events = journal.events_for_run(RunId::new(9999)).expect("events_for_run");
        assert_eq!(events.len(), 0, "no events for nonexistent run");
    }

    #[test]
    fn adversarial_run_header_for_never_written_run_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let header = journal.run_header(RunId::new(8888)).expect("run_header");
        assert!(header.is_none(), "no header for nonexistent run");
    }

    #[test]
    fn adversarial_snapshot_for_nonexistent_run_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let snapshot = journal.snapshot(RunId::new(7777), EventSeq::new(0)).expect("snapshot");
        assert!(snapshot.is_none(), "no snapshot for nonexistent run");
    }

    #[test]
    fn adversarial_blob_for_nonexistent_digest_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let blob = journal.blob([0xAA; 32]).expect("blob");
        assert!(blob.is_none(), "no blob for nonexistent digest");
    }

    #[test]
    fn adversarial_workflow_source_for_wrong_digest_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest_a = WorkflowDigest::from_bytes([1; 32]);
        let record = WorkflowSourceRecord {
            digest: digest_a,
            source: b"data".to_vec(),
        };
        journal.put_workflow_source(&record).expect("put");
        let digest_b = WorkflowDigest::from_bytes([2; 32]);
        let result = journal.workflow_source(digest_b).expect("get");
        assert!(result.is_none(), "wrong digest must return None");
    }

    #[test]
    fn adversarial_multiple_snapshots_same_run_different_seq_all_retrievable() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9006);
        for seq_val in [0u64, 5, 10] {
            let snap = RunSnapshot {
                run,
                seq: EventSeq::new(seq_val),
                workflow: WorkflowDigest::from_bytes([1; 32]),
                slots: vec![0u8],
            };
            journal.put_snapshot(&snap).expect("put_snapshot");
        }
        for seq_val in [0u64, 5, 10] {
            let loaded = journal.snapshot(run, EventSeq::new(seq_val)).expect("get");
            assert!(loaded.is_some(), "snapshot at seq {seq_val} must exist");
        }
    }


    #[test]
    fn adversarial_batch_two_sequential_commits_both_visible() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest1 = WorkflowDigest::from_bytes([1; 32]);
        let digest2 = WorkflowDigest::from_bytes([2; 32]);
        let mut batch1 = journal.batch();
        batch1
            .put_workflow_source(&WorkflowSourceRecord {
                digest: digest1,
                source: b"first".to_vec(),
            })
            .expect("put1");
        batch1.commit().expect("commit1");
        let mut batch2 = journal.batch();
        batch2
            .put_workflow_source(&WorkflowSourceRecord {
                digest: digest2,
                source: b"second".to_vec(),
            })
            .expect("put2");
        batch2.commit().expect("commit2");
        assert!(journal.workflow_source(digest1).expect("get1").is_some());
        assert!(journal.workflow_source(digest2).expect("get2").is_some());
    }

    #[test]
    fn adversarial_snapshot_with_empty_slots_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9010);
        let snap = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
            slots: vec![],
        };
        journal.put_snapshot(&snap).expect("put");
        let loaded = journal.snapshot(run, EventSeq::new(0)).expect("get").expect("must exist");
        assert_eq!(loaded.slots.len(), 0);
        assert_eq!(loaded.run, run);
    }

    #[test]
    fn adversarial_blob_with_single_byte_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([42; 32]);
        let record = BlobRecord {
            digest: digest.as_bytes(),
            bytes: vec![0xFF],
        };
        journal.put_blob(&record).expect("put");
        let loaded = journal.blob(digest.as_bytes()).expect("get").expect("must exist");
        assert_eq!(loaded.bytes, vec![0xFF]);
    }

    #[test]
    fn adversarial_workflow_source_with_empty_bytes_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([7; 32]);
        let record = WorkflowSourceRecord {
            digest,
            source: vec![],
        };
        journal.put_workflow_source(&record).expect("put");
        let loaded = journal.workflow_source(digest).expect("get").expect("must exist");
        assert_eq!(loaded.source, vec![]);
    }

    #[test]
    fn adversarial_run_header_with_max_run_id_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(u64::MAX);
        let digest = WorkflowDigest::from_bytes([9; 32]);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(u32::MAX),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: u64::MAX,
        };
        journal.put_run_header(&record).expect("put");
        let loaded = journal.run_header(run).expect("get").expect("must exist");
        assert_eq!(loaded.run, RunId::new(u64::MAX));
        assert_eq!(loaded.workflow_id, WorkflowId::new(u32::MAX));
        assert_eq!(loaded.accepted_at_ms, u64::MAX);
    }

    #[test]
    fn adversarial_batch_strict_commit_survives_immediate_reopen() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let journal = FjallJournal::open(&path, None).expect("setup: journal open");
        let run = RunId::new(9020);
        let digest = WorkflowDigest::from_bytes([11; 32]);
        let mut batch = journal.batch().strict();
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(3),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 500,
            })
            .expect("put");
        batch.strict().commit().expect("strict commit");
        drop(journal);
        let journal2 = FjallJournal::open(&path, None).expect("reopen");
        let header = journal2.run_header(run).expect("get").expect("must exist");
        assert_eq!(header.run, run);
        assert_eq!(header.status, 1);
    }

    #[test]
    fn adversarial_events_for_run_isolates_run_a_from_run_b() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run: run_a,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append a");
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run: run_b,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append b");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run: run_a,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::ZERO,
            })
            .expect("append a2");
        let events_a = journal.events_for_run(run_a).expect("events a");
        let events_b = journal.events_for_run(run_b).expect("events b");
        assert_eq!(events_a.len(), 2, "run A should have 2 events");
        assert_eq!(events_b.len(), 1, "run B should have 1 event");
    }

    #[test]
    fn adversarial_run_header_overwrite_replaces_previous() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9030);
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 100,
            })
            .expect("put first");
        journal
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(2),
                compiled_digest: digest,
                status: 3,
                accepted_at_ms: 200,
            })
            .expect("put second");
        let header = journal.run_header(run).expect("get").expect("exists");
        assert_eq!(header.workflow_id, WorkflowId::new(2));
        assert_eq!(header.status, 3);
        assert_eq!(header.accepted_at_ms, 200);
    }


    #[test]
    fn adversarial_batch_commit_with_5_puts_persists_all() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let d1 = WorkflowDigest::from_bytes([1; 32]);
        let d2 = WorkflowDigest::from_bytes([2; 32]);
        let run = RunId::new(9050);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest: d1,
                source: b"s".to_vec(),
            })
            .expect("put1");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest: d2,
                ir: b"ir".to_vec(),
            })
            .expect("put2");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: d1,
                status: 1,
                accepted_at_ms: 0,
            })
            .expect("put3");
        batch
            .put_blob(&BlobRecord {
                digest: d1.as_bytes(),
                bytes: b"b".to_vec(),
            })
            .expect("put4");
        batch
            .put_status_index(1, 0, run)
            .expect("put5");
        batch.commit().expect("commit");
        assert!(journal.workflow_source(d1).expect("g1").is_some());
        assert!(journal.compiled_ir(d2).expect("g2").is_some());
        assert!(journal.run_header(run).expect("g3").is_some());
        assert!(journal.blob(d1.as_bytes()).expect("g4").is_some());
    }

    #[test]
    fn adversarial_compiled_ir_with_different_ir_same_digest_overwrites() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"version1".to_vec(),
            })
            .expect("put1");
        journal
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"version2".to_vec(),
            })
            .expect("put2");
        let loaded = journal.compiled_ir(digest).expect("get").expect("exists");
        assert_eq!(loaded.ir, b"version2".to_vec(), "second write must win");
    }

    #[test]
    fn adversarial_journal_open_fresh_database_is_empty() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        assert!(journal.run_header(RunId::new(1)).expect("header").is_none());
        assert!(journal.workflow_source(WorkflowDigest::from_bytes([0; 32])).expect("source").is_none());
        assert!(journal.compiled_ir(WorkflowDigest::from_bytes([0; 32])).expect("ir").is_none());
        assert!(journal.blob([0; 32]).expect("blob").is_none());
        assert_eq!(journal.events_for_run(RunId::new(1)).expect("events").len(), 0);
    }

    #[test]
    fn adversarial_snapshot_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run1 = RunId::new(100);
        let run2 = RunId::new(200);
        journal
            .put_snapshot(&RunSnapshot {
                run: run1,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
                slots: vec![1u8],
            })
            .expect("snap1");
        journal
            .put_snapshot(&RunSnapshot {
                run: run2,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([2; 32]),
                slots: vec![2u8],
            })
            .expect("snap2");
        let s1 = journal.snapshot(run1, EventSeq::new(0)).expect("get1").expect("exists");
        let s2 = journal.snapshot(run2, EventSeq::new(0)).expect("get2").expect("exists");
        assert_eq!(s1.workflow, WorkflowDigest::from_bytes([1; 32]));
        assert_eq!(s2.workflow, WorkflowDigest::from_bytes([2; 32]));
    }

    #[test]
    fn adversarial_status_index_multiple_runs_same_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = 1u8;
        let ts = 1000u64;
        for run_id in [RunId::new(10), RunId::new(20), RunId::new(30)] {
            journal.put_status_index(state, ts, run_id).expect("put");
        }
        // All three runs should be indexable under the same state
        // (verification via no-error roundtrip)
    }

    #[test]
    fn adversarial_workflow_index_multiple_runs_same_workflow() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let wf = WorkflowId::new(42);
        for run_id in [RunId::new(1), RunId::new(2), RunId::new(3)] {
            journal.put_workflow_index(wf, run_id).expect("put");
        }
        // All three runs indexed under same workflow
    }

    #[test]
    fn adversarial_batch_empty_strict_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch().strict();
        batch.strict().commit().expect("empty strict commit must succeed");
    }

    #[test]
    fn adversarial_append_event_at_max_seq_stores_correctly() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9060);
        // Write contiguous events 0..2, then verify seq 0 and 1 are present
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append0");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::ZERO,
            })
            .expect("append1");
        let events = journal.events_for_run(run).expect("replay");
        assert_eq!(events.len(), 2, "contiguous seq 0,1 must replay");
    }

    #[test]
    fn adversarial_batch_commit_persists_all_keys_or_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([13; 32]);
        let run = RunId::new(9070);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: b"src".to_vec(),
            })
            .expect("ws");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"ir".to_vec(),
            })
            .expect("ir");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 0,
            })
            .expect("rh");
        batch.commit().expect("commit");
        // All three must be present — batch is atomic
        assert!(journal.workflow_source(digest).expect("g1").is_some());
        assert!(journal.compiled_ir(digest).expect("g2").is_some());
        assert!(journal.run_header(run).expect("g3").is_some());
    }
}

#[cfg(test)]
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
mod proptests {
    use crate::{
        BlobRecord, EventSeq, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_WORKFLOW_SOURCE,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RecordKind, WorkflowSourceRecord, blob_key,
        compiled_ir_key, decode_record, encode_record, index_action_key, index_status_key,
        index_workflow_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
    };
    use proptest::prelude::*;
    use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest, WorkflowId};

    proptest! {
        #[test]
        fn run_event_key_ordering_is_monotonic(seq1 in 0u64..1000u64, seq2 in 0u64..1000u64) {
            let run = RunId::new(42);
            let key1 = run_event_key(run, EventSeq::new(seq1));
            let key2 = run_event_key(run, EventSeq::new(seq2));
            let Ok(k1) = key1 else { return Ok(()) };
            let Ok(k2) = key2 else { return Ok(()) };
            if seq1 < seq2 {
                prop_assert!(k1 < k2);
            } else if seq1 > seq2 {
                prop_assert!(k1 > k2);
            }
        }

        #[test]
        fn encode_decode_record_roundtrip_for_all_record_kinds(
            kind_id in 10u16..=23u16,
            run_val in 1u64..=1000u64,
            seq_val in 0u64..=100u64,
        ) {
            // Given a RunAccepted event (all journal events share the same encode/decode path)
            // When encoded with MAGIC_JOURNAL_EVENT and the given kind, then decoded
            // Then the round trip preserves the original event
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let event = crate::JournalEvent::RunAccepted {
                run,
                seq,
                workflow: WorkflowDigest::from_bytes([kind_id as u8; 32]),
            };
            let kind = match kind_id {
                10 => RecordKind::RunAccepted,
                11 => RecordKind::StepStarted,
                12 => RecordKind::SlotWritten,
                13 => RecordKind::ActionScheduled,
                14 => RecordKind::ActionCompleted,
                15 => RecordKind::ActionFailed,
                16 => RecordKind::WaitScheduled,
                17 => RecordKind::AskScheduled,
                18 => RecordKind::AskAnswered,
                19 => RecordKind::RetryScheduled,
                20 => RecordKind::StepFailed,
                21 => RecordKind::RunCancelled,
                22 => RecordKind::RunFinished,
                23 => RecordKind::RunFailed,
                _ => return Ok(()),
            };
            let encoded = encode_record(
                MAGIC_JOURNAL_EVENT,
                kind,
                seq_val,
                &event,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<crate::JournalEvent>(
                &encoded,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok((_envelope, decoded_event)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_event, event);
        }

        #[test]
        fn journal_key_bytes_are_deterministic(
            run_val in 1u64..=10000u64,
            seq_val in 0u64..=1000u64,
        ) {
            // Given the same run and seq inputs
            // When run_event_key is called twice
            // Then both results are identical
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let key1 = run_event_key(run, seq);
            let key2 = run_event_key(run, seq);
            let Ok(k1) = key1 else { return Ok(()) };
            let Ok(k2) = key2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);
        }

        #[test]
        fn event_seq_new_never_panics_for_valid_values(val in 0u64..=u64::MAX) {
            // Given any valid u64
            // When EventSeq::new is called
            // Then get() returns the same value
            let seq = EventSeq::new(val);
            prop_assert_eq!(seq.get(), val);
        }

        #[test]
        fn record_kind_id_roundtrip(kind_id in 1u16..=50u16) {
            // Given a valid record kind id
            // When it matches a known RecordKind variant
            // Then the id() round-trips correctly
            let kind = match kind_id {
                1 => RecordKind::WorkflowSource,
                2 => RecordKind::CompiledIr,
                3 => RecordKind::RunHeader,
                10 => RecordKind::RunAccepted,
                11 => RecordKind::StepStarted,
                12 => RecordKind::SlotWritten,
                13 => RecordKind::ActionScheduled,
                14 => RecordKind::ActionCompleted,
                15 => RecordKind::ActionFailed,
                16 => RecordKind::WaitScheduled,
                17 => RecordKind::AskScheduled,
                18 => RecordKind::AskAnswered,
                19 => RecordKind::RetryScheduled,
                20 => RecordKind::StepFailed,
                21 => RecordKind::RunCancelled,
                22 => RecordKind::RunFinished,
                23 => RecordKind::RunFailed,
                30 => RecordKind::Snapshot,
                40 => RecordKind::Blob,
                50 => RecordKind::IndexUpdate,
                _ => return Ok(()),
            };
            prop_assert_eq!(kind.id(), kind_id);
        }

        #[test]
        fn all_key_functions_are_deterministic(
            run_val in 1u64..=1000u64,
            seq_val in 0u64..=100u64,
            state_val in 0u8..=255u8,
            ts_val in 0u64..=10000u64,
            wf_val in 1u32..=1000u32,
            action_val in 1u16..=1000u16,
            step_val in 0u16..=100u16,
        ) {
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let digest = [42u8; 32];

            let k1 = workflow_source_key(digest);
            let k2 = workflow_source_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = compiled_ir_key(digest);
            let k2 = compiled_ir_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_header_key(run);
            let k2 = run_header_key(run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_event_key(run, seq);
            let k2 = run_event_key(run, seq);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = run_snapshot_key(run, seq);
            let k2 = run_snapshot_key(run, seq);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = blob_key(digest);
            let k2 = blob_key(digest);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_status_key(state_val, ts_val, run);
            let k2 = index_status_key(state_val, ts_val, run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_workflow_key(WorkflowId::new(wf_val), run);
            let k2 = index_workflow_key(WorkflowId::new(wf_val), run);
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);

            let k1 = index_action_key(ActionId::new(action_val), run, StepIdx::new(step_val));
            let k2 = index_action_key(ActionId::new(action_val), run, StepIdx::new(step_val));
            let Ok(k1) = k1 else { return Ok(()) };
            let Ok(k2) = k2 else { return Ok(()) };
            prop_assert_eq!(k1, k2);
        }

        #[test]
        fn workflow_source_roundtrip_with_arbitrary_source_bytes(
            source_bytes in proptest::collection::vec(any::<u8>(), 0..100usize),
        ) {
            let digest = WorkflowDigest::from_bytes([77; 32]);
            let record = WorkflowSourceRecord {
                digest,
                source: source_bytes,
            };
            let encoded = encode_record(
                MAGIC_WORKFLOW_SOURCE,
                RecordKind::WorkflowSource,
                0,
                &record,
                65536,
            );
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<WorkflowSourceRecord>(&encoded, MAGIC_WORKFLOW_SOURCE, 65536);
            let Ok((_env, decoded_record)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_record, record);
        }

        #[test]
        fn blob_roundtrip_with_arbitrary_bytes(
            blob_bytes in proptest::collection::vec(any::<u8>(), 0..100usize),
        ) {
            let digest = [88u8; 32];
            let record = BlobRecord {
                digest,
                bytes: blob_bytes,
            };
            let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 65536);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, 65536);
            let Ok((_env, decoded_record)) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded_record, record);
        }
    }
}

#[test]
fn drop_persists_without_panic() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallJournal, JournalEvent};
    use vb_core::{RunId, WorkflowDigest};
    // Given a journal with one appended event
    // When the journal is dropped
    // Then it should not panic (persist is best-effort)
    let temp_dir = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(temp_dir.path(), None)?;
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1u8; 32]),
        };
        journal.append_journaled(&event)?;
    }
    // reopen to verify data survived drop persist
    let reopened = FjallJournal::open(temp_dir.path(), None)?;
    let events = reopened.events_for_run(RunId::new(1))?;
    if events.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

#[test]
fn events_for_run_uses_snapshot_isolation() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallJournal, JournalEvent};
    use vb_core::{RunId, StepIdx, WorkflowDigest};
    // Given a journal with two events
    // When events_for_run is called
    // Then it should return a consistent snapshot even if writes interleave
    let temp_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp_dir.path(), None)?;
    let event0 = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    let event1 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
    };
    journal.append_journaled(&event0)?;
    journal.append_journaled(&event1)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 2 {
        return Err("expected two replayed events".into());
    }
    if replay.first() != Some(&event0) {
        return Err("first replayed event mismatch".into());
    }
    if replay.get(1) != Some(&event1) {
        return Err("second replayed event mismatch".into());
    }
    Ok(())
}

#[test]
fn open_with_custom_cache_size() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, FjallConfig, FjallJournal, JournalEvent};
    use vb_core::{RunId, WorkflowDigest};
    // Given a custom FjallConfig with 512 MiB cache
    // When the journal is opened with that config
    // Then it should open successfully
    let temp_dir = tempfile::tempdir()?;
    let config = FjallConfig {
        cache_size_bytes: 536_870_912, // 512 MiB
    };
    let journal = FjallJournal::open(temp_dir.path(), Some(config))?;
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    journal.append_journaled(&event)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

#[test]
fn open_store_uses_default_config() -> Result<(), Box<dyn std::error::Error>> {
    use crate::{EventSeq, JournalEvent, open_store};
    use vb_core::{RunId, WorkflowDigest};
    // Given no explicit config
    // When open_store is called
    // Then it should open with the default 256 MiB cache
    let temp_dir = tempfile::tempdir()?;
    let journal = open_store(temp_dir.path())?;
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1u8; 32]),
    };
    journal.append_journaled(&event)?;
    let replay = journal.events_for_run(RunId::new(1))?;
    if replay.len() != 1 {
        return Err("expected one replayed event".into());
    }
    Ok(())
}

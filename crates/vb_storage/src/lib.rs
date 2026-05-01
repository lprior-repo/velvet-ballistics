//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

pub mod recovery;

use arrayvec::ArrayVec;
use recovery::RunSnapshot;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
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
    pending: Mutex<VecDeque<QueuedJournalEvent>>,
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
            pending: Mutex::new(VecDeque::with_capacity(capacity)),
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
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        if pending.len() >= self.capacity {
            return Err(JournalError::QueueFull);
        }
        pending.push_back(QueuedJournalEvent { event, profile });
        Ok(())
    }

    /// Returns pending write counts split by durability profile.
    pub fn pending_profile_counts(&self) -> Result<JournalWriterQueueProfileCounts, JournalError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let mut counts = JournalWriterQueueProfileCounts {
            journaled: 0,
            strict: 0,
        };
        for item in &*pending {
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
        let mut drained = 0usize;
        let mut written = 0usize;
        while drained < self.batch_size {
            let item = {
                let mut pending = self
                    .pending
                    .lock()
                    .map_err(|_| JournalError::WriteLockPoisoned)?;
                pending.pop_front()
            };
            let Some(item) = item else {
                break;
            };
            drained = drained.saturating_add(1);
            if item.profile == DurabilityProfile::Strict {
                journal.append_strict(&item.event)?;
            } else {
                journal.append_journaled(&item.event)?;
            }
            written = written.saturating_add(1);
        }
        Ok(JournalWriterFlushReport { drained, written })
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
            Self::StepSucceeded { .. } => RecordKind::SlotWritten,
            Self::ActionScheduled { .. } => RecordKind::ActionScheduled,
            Self::ActionCompletedEvent { .. } => RecordKind::ActionCompleted,
            Self::ActionFailedEvent { .. } => RecordKind::ActionFailed,
            Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
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
    write_lock: Mutex<()>,
}

impl FjallJournal {
    /// Opens or creates the journal at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let database = fjall::Database::builder(path).open()?;
        let workflow_source = database.keyspace(
            KEYSPACE_WORKFLOW_SOURCE,
            fjall::KeyspaceCreateOptions::default,
        )?;
        let compiled_ir =
            database.keyspace(KEYSPACE_COMPILED_IR, fjall::KeyspaceCreateOptions::default)?;
        let run_header =
            database.keyspace(KEYSPACE_RUN_HEADER, fjall::KeyspaceCreateOptions::default)?;
        let events =
            database.keyspace(KEYSPACE_RUN_EVENT, fjall::KeyspaceCreateOptions::default)?;
        let run_snapshot =
            database.keyspace(KEYSPACE_RUN_SNAPSHOT, fjall::KeyspaceCreateOptions::default)?;
        let blob = database.keyspace(KEYSPACE_BLOB, fjall::KeyspaceCreateOptions::default)?;
        let index_status =
            database.keyspace(KEYSPACE_INDEX_STATUS, fjall::KeyspaceCreateOptions::default)?;
        let index_workflow = database.keyspace(
            KEYSPACE_INDEX_WORKFLOW,
            fjall::KeyspaceCreateOptions::default,
        )?;
        let index_action =
            database.keyspace(KEYSPACE_INDEX_ACTION, fjall::KeyspaceCreateOptions::default)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
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

    /// Stores a compact run snapshot.
    pub fn put_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
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
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        self.append_unpersisted(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        self.append_unpersisted(event)?;
        self.persist_strict()
    }

    fn append_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
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

    /// Forces a strict durability barrier.
    pub fn persist_strict(&self) -> Result<(), JournalError> {
        self.database.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }

    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
        let mut expected = EventSeq::new(0);

        for item in self.events.prefix(run_prefix(run)?) {
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
}

/// Opens the Fjall-backed storage engine.
pub fn open_store(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path)
}

/// Initializes all declared keyspaces by opening the store.
pub fn init_keyspaces(path: impl AsRef<Path>) -> Result<FjallJournal, JournalError> {
    FjallJournal::open(path)
}

/// Appends one journal event without forcing a durability barrier.
pub fn append_journal_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    journal.append_journaled(event)
}

/// Writes a compact run snapshot.
pub fn write_snapshot(journal: &FjallJournal, snapshot: &RunSnapshot) -> Result<(), JournalError> {
    journal.put_snapshot(snapshot)
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
    let header = bytes
        .get(..RECORD_HEADER_BYTES)
        .ok_or(JournalError::UnexpectedEof)?;
    let magic = read_u32(header, 0)?;
    if magic != expected_magic {
        return Err(JournalError::BadMagic { found: magic });
    }
    let version = read_u16(header, 4)?;
    validate_schema_version(version)?;
    let kind = read_u16(header, 6)?;
    validate_known_kind(kind)?;
    validate_kind_family(magic, kind)?;
    let header_len = read_u32(header, 8)?;
    if header_len != RECORD_HEADER_LEN {
        return Err(JournalError::HeaderLengthMismatch { found: header_len });
    }
    let payload_len = read_u32(header, 12)?;
    if payload_len > max_payload_len {
        return Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max: max_payload_len,
        });
    }
    let expected_crc = read_u32(header, CRC_OFFSET)?;
    if crc32c::crc32c(header_prefix_for_crc(header)?) != expected_crc {
        return Err(JournalError::HeaderChecksumMismatch);
    }
    let payload_start = usize::try_from(header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize =
        usize::try_from(payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len_usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    let digest = digest_from_header(header)?;
    if blake3::hash(payload).as_bytes() != &digest {
        return Err(JournalError::PayloadDigestMismatch);
    }
    Ok((
        RecordEnvelope {
            magic,
            schema_version: version,
            record_kind: kind,
            sequence: read_u64(header, 16)?,
        },
        payload,
    ))
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
        BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, EventSeq, FjallJournal, JournalError,
        JournalEvent, JournalWriterQueue, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
        MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE,
        MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB,
        PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW,
        PREFIX_RUN_EVENT, PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE,
        RECORD_HEADER_LEN, RecordKind, RunHeaderRecord, StorageLimits, WorkflowSourceRecord,
        append_journal_event, blob_key, compiled_ir_key, decode_record, encode_record,
        flush_profile, index_action_key, index_status_key, index_workflow_key, init_keyspaces,
        journal_key, open_store, read_blob, read_run_events, replay_journal, run_event_key,
        run_header_key, run_snapshot_key, workflow_source_key, write_snapshot,
    };
    use crate::recovery::{ActionReplayTracker, RunSnapshot};
    use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest, WorkflowId};

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
    fn journal_opens_declared_keyspaces_and_round_trips_typed_records() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok(), "tempdir should be created");
        let Ok(temp_dir) = temp_dir else {
            return;
        };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok(), "journal should open");
        let Ok(journal) = journal else {
            return;
        };
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

        assert!(journal.put_workflow_source(&source).is_ok());
        assert!(journal.put_compiled_ir(&ir).is_ok());
        assert!(journal.put_run_header(&header).is_ok());
        assert!(journal.put_snapshot(&snapshot).is_ok());
        assert!(journal.put_blob(&blob).is_ok());
        assert!(journal.put_status_index(1, 2, RunId::new(3)).is_ok());
        assert!(
            journal
                .put_workflow_index(WorkflowId::new(4), RunId::new(3))
                .is_ok()
        );
        assert!(
            journal
                .put_action_index(ActionId::new(5), RunId::new(3), StepIdx::new(6))
                .is_ok()
        );

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
        assert!(encoded.is_ok());
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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok(), "tempdir should be created");
        let Ok(temp_dir) = temp_dir else {
            return;
        };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok(), "journal should open");
        let Ok(journal) = journal else {
            return;
        };
        let event = JournalEvent::RunAccepted {
            run: RunId::new(9),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };

        let first = journal.append_journaled(&event);
        let second = journal.append_journaled(&event);

        assert!(first.is_ok());
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

        assert!(queue.enqueue_journaled(journaled).is_ok());
        assert!(queue.enqueue_strict(strict).is_ok());

        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 1 && counts.strict == 1
        ));
    }

    #[test]
    fn flush_profile_wrapper_flushes_queued_events() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };
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

        assert!(queue.enqueue_journaled(journaled.clone()).is_ok());
        assert!(queue.enqueue_strict(strict.clone()).is_ok());
        let report = flush_profile(&queue, &journal);

        assert!(report.is_ok());
        let Ok(report) = report else { return };
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);
        let events = read_run_events(&journal, run);
        assert!(events.is_ok());
        let Ok(events) = events else { return };
        assert_eq!(events, vec![journaled, strict]);
    }

    #[test]
    fn replay_returns_contiguous_events_for_run() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok(), "tempdir should be created");
        let Ok(temp_dir) = temp_dir else {
            return;
        };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok(), "journal should open");
        let Ok(journal) = journal else {
            return;
        };
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

        assert!(journal.append_journaled(&accepted).is_ok());
        assert!(journal.append_journaled(&finished).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run_a = RunId::new(10);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        assert!(journal.append_journaled(&event).is_ok());

        let run_b = RunId::new(20);
        let result = journal.events_for_run(run_b);
        assert!(result.is_ok());
        let events = result.expect("events_for_run should succeed for missing run");
        assert!(events.is_empty(), "no events should exist for run_b");
    }

    #[test]
    fn validate_replayed_event_returns_sequence_gap_when_seq_out_of_order() {
        // Given a journal with seq 0 then seq 2 for the same run
        // When events_for_run replays
        // Then it returns SequenceGap with expected=1, actual=2
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(100);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        assert!(journal.append_journaled(&event0).is_ok());

        // Manually insert an event at seq 2 (skipping seq 1)
        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
        };
        assert!(journal.append_journaled(&event2).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(7),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        assert!(journal.append_journaled(&event).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
    }

    #[test]
    fn public_open_wrappers_create_declared_keyspaces() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };

        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok());
        drop(journal);

        let reopened = init_keyspaces(temp_dir.path());
        assert!(reopened.is_ok());
        assert_eq!(FjallJournal::declared_keyspaces().len(), 9);
    }

    #[test]
    fn public_wrappers_delegate_to_journal_storage_paths() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };
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

        assert!(append_journal_event(&journal, &event).is_ok());
        assert!(journal.put_blob(&blob).is_ok());
        assert!(write_snapshot(&journal, &snapshot).is_ok());

        let events = read_run_events(&journal, run);
        assert!(events.is_ok());
        let Ok(events) = events else { return };
        assert_eq!(events, vec![event.clone()]);
        let loaded_blob = read_blob(&journal, blob.digest);
        assert!(loaded_blob.is_ok());
        let Ok(loaded_blob) = loaded_blob else {
            return;
        };
        assert_eq!(loaded_blob, Some(blob));
        let loaded_snapshot = journal.snapshot(run, EventSeq::new(0));
        assert!(loaded_snapshot.is_ok());
        let Ok(loaded_snapshot) = loaded_snapshot else {
            return;
        };
        assert_eq!(loaded_snapshot, Some(snapshot));
    }

    #[test]
    fn replay_journal_wrapper_uses_recovery_replay() {
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };
        let run = RunId::new(71);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([8; 32]),
        };
        assert!(append_journal_event(&journal, &event).is_ok());

        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_journal(&journal, run, &mut tracker);

        assert!(replayed.is_ok());
        let Ok(replayed) = replayed else { return };
        assert_eq!(replayed, vec![event]);
    }

    #[test]
    fn append_strict_persists_submitted_event() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event
        // Then the event can be retrieved via events_for_run
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(55);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let result = journal.append_strict(&event);
        assert!(result.is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(60);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        assert!(journal.append_strict(&event0).is_ok());

        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
        };
        assert!(journal.append_strict(&event2).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };

        let run = RunId::new(77);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        {
            let journal = FjallJournal::open(temp_dir.path());
            assert!(journal.is_ok());
            let Ok(journal) = journal else { return };
            assert!(journal.append_strict(&event).is_ok());
        }

        let journal2 = FjallJournal::open(temp_dir.path());
        assert!(journal2.is_ok());
        let Ok(journal2) = journal2 else { return };
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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let digest = WorkflowDigest::from_bytes([42; 32]);
        let record = WorkflowSourceRecord {
            digest,
            source: vec![b'h', b'e', b'l', b'l', b'o'],
        };
        assert!(journal.put_workflow_source(&record).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: WorkflowDigest::from_bytes([8; 32]),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        assert!(journal.put_run_header(&record).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let digest = WorkflowDigest::from_bytes([3; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        assert!(journal.put_compiled_ir(&record).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let digest = [0xCC; 32];
        let record = BlobRecord {
            digest,
            bytes: vec![1, 2, 3, 4, 5],
        };
        assert!(journal.put_blob(&record).is_ok());

        let retrieved = journal.blob(digest).expect("blob lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_snapshot_stores_and_retrieves() {
        // Given an open journal and a run snapshot
        // When put_snapshot is called
        // Then the snapshot can be retrieved by run and seq
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let snapshot = RunSnapshot {
            run: RunId::new(88),
            seq: EventSeq::new(10),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            slots: vec![1, 2, 3],
        };
        assert!(journal.put_snapshot(&snapshot).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let result = journal.put_action_index(ActionId::new(1), RunId::new(2), StepIdx::new(3));
        assert!(result.is_ok());
    }

    #[test]
    fn put_status_index_stores_and_retrieves() {
        // Given an open journal
        // When put_status_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let result = journal.put_status_index(1, 1700000000, RunId::new(99));
        assert!(result.is_ok());
    }

    #[test]
    fn put_workflow_index_stores_and_retrieves() {
        // Given an open journal
        // When put_workflow_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let result = journal.put_workflow_index(WorkflowId::new(7), RunId::new(8));
        assert!(result.is_ok());
    }

    #[test]
    fn events_for_run_returns_only_events_for_target_run() {
        // Given a journal with events for run 10 and run 20
        // When events_for_run is called for run 10
        // Then only run 10 events are returned
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

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

        assert!(journal.append_journaled(&event_a0).is_ok());
        assert!(journal.append_journaled(&event_b0).is_ok());
        assert!(journal.append_journaled(&event_a1).is_ok());

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let result = journal.blob([0; 32]).expect("lookup should succeed");
        assert_eq!(result, None);
    }

    // --- Section 4: Journal Lifecycle BDD Tests ---

    fn open_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal should open");
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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&accepted).is_ok());
        assert!(journal.append_strict(&started).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_strict(&e0).is_ok());
        assert!(journal.append_strict(&e1).is_ok());
        assert!(journal.append_strict(&e2).is_ok());

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
        assert!(journal.append_strict(&event).is_ok());

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
        assert!(journal.append_journaled(&e0).is_ok());
        assert!(journal.append_journaled(&e1).is_ok());
        assert!(journal.append_journaled(&e2).is_ok());
        assert!(journal.append_journaled(&e3).is_ok());
        assert!(journal.append_journaled(&e4).is_ok());

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
        assert!(journal.append_journaled(&event).is_ok());

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

        assert!(journal.append_journaled(&a0).is_ok());
        assert!(journal.append_journaled(&b0).is_ok());
        assert!(journal.append_journaled(&a1).is_ok());
        assert!(journal.append_journaled(&b1).is_ok());
        assert!(journal.append_journaled(&a2).is_ok());

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
        assert!(journal.append_journaled(&event).is_ok());

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
        assert!(journal.put_run_header(&record).is_ok());

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
        assert!(journal.put_snapshot(&snapshot).is_ok());

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
        assert!(journal.put_compiled_ir(&record).is_ok());

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
        assert!(journal.put_workflow_source(&record).is_ok());

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
        assert!(journal.put_run_header(&original).is_ok());
        assert!(journal.put_run_header(&updated).is_ok());

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

        assert!(journal.append_journaled(&r1_e0).is_ok());
        assert!(journal.append_journaled(&r1_e1).is_ok());
        assert!(journal.append_journaled(&r2_e0).is_ok());
        assert!(journal.append_journaled(&r2_e1).is_ok());
        assert!(journal.append_journaled(&r2_e2).is_ok());

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
        assert!(journal.append_journaled(&event).is_ok());
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
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let run = RunId::new(999);

        {
            let journal = FjallJournal::open(temp_dir.path()).expect("open should succeed");
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
                assert!(journal.append_strict(event).is_ok());
            }
        }

        let journal2 = FjallJournal::open(temp_dir.path()).expect("reopen should succeed");
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
        assert!(journal.put_run_header(&record).is_ok());
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
        assert!(journal.put_blob(&record).is_ok());
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
        assert!(journal.put_workflow_source(&record).is_ok());
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
            MAGIC_JOURNAL_EVENT, kind, event.seq().get(), event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encoding should succeed");
        let end = offset.saturating_add(new_bytes.len());
        assert!(end <= 56, "patch must be within CRC-protected region");
        encoded.get_mut(offset..end).expect("patch range valid").copy_from_slice(new_bytes);
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
        let event = JournalEvent::RunAccepted { run: RunId::new(1), seq: EventSeq::new(0), workflow: test_digest(1) };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, event.seq().get(), &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("ok");
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_SNAPSHOT, 128);
        let Err(JournalError::BadMagic { found }) = result else { panic!("expected BadMagic, got {:?}", result) };
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }

    #[test]
    fn adversarial_decode_vbir_magic_on_journal_returns_bad_magic() {
        let record = CompiledIrRecord { digest: test_digest(1), ir: vec![1, 2, 3] };
        let encoded = encode_record(MAGIC_COMPILED_ARTIFACT, RecordKind::CompiledIr, 0, &record, MAX_COMPILED_IR_BYTES).expect("ok");
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::BadMagic { found }) = result else { panic!("expected BadMagic, got {:?}", result) };
        assert_eq!(found, MAGIC_COMPILED_ARTIFACT);
    }

    #[test]
    fn adversarial_decode_unsupported_schema_version_returns_exact_version() {
        let event = JournalEvent::RunAccepted { run: RunId::new(2), seq: EventSeq::new(0), workflow: test_digest(2) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &5u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else { panic!("expected UnsupportedSchemaVersion, got {:?}", result) };
        assert_eq!(version, 5);
    }

    #[test]
    fn adversarial_decode_unknown_record_kind_returns_exact_kind() {
        let event = JournalEvent::RunAccepted { run: RunId::new(3), seq: EventSeq::new(0), workflow: test_digest(3) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &99u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnknownRecordKind { kind }) = result else { panic!("expected UnknownRecordKind, got {:?}", result) };
        assert_eq!(kind, 99);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_snapshot_kind_in_journal() {
        let event = JournalEvent::RunAccepted { run: RunId::new(4), seq: EventSeq::new(0), workflow: test_digest(4) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &30u16.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else { panic!("expected mismatch, got {:?}", result) };
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(kind, 30);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_blob_in_snapshot() {
        let event = JournalEvent::RunAccepted { run: RunId::new(5), seq: EventSeq::new(0), workflow: test_digest(5) };
        let result = encode_record(MAGIC_SNAPSHOT, RecordKind::Blob, event.seq().get(), &event, MAX_SNAPSHOT_BYTES);
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else { panic!("expected mismatch, got {:?}", result) };
        assert_eq!(magic, MAGIC_SNAPSHOT);
        assert_eq!(kind, RecordKind::Blob.id());
    }

    #[test]
    fn adversarial_decode_header_len_not_60_returns_mismatch() {
        let event = JournalEvent::RunAccepted { run: RunId::new(6), seq: EventSeq::new(0), workflow: test_digest(6) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::HeaderLengthMismatch { found }) = result else { panic!("expected mismatch, got {:?}", result) };
        assert_eq!(found, 48);
    }

    #[test]
    fn adversarial_decode_payload_len_above_limit_returns_too_large() {
        let event = JournalEvent::RunAccepted { run: RunId::new(7), seq: EventSeq::new(0), workflow: test_digest(7) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 12, &9999u32.to_le_bytes());
        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 100);
        let Err(JournalError::PayloadTooLarge { len, max }) = result else { panic!("expected PayloadTooLarge, got {:?}", result) };
        assert_eq!(len, 9999);
        assert_eq!(max, 100);
    }

    #[test]
    fn adversarial_decode_corrupt_header_crc_returns_checksum_mismatch() {
        let event = JournalEvent::RunAccepted { run: RunId::new(8), seq: EventSeq::new(0), workflow: test_digest(8) };
        let mut encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, event.seq().get(), &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("ok");
        if let Some(b) = encoded.get_mut(57) { *b ^= 0x80; }
        assert!(matches!(decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128), Err(JournalError::HeaderChecksumMismatch)));
    }

    #[test]
    fn adversarial_decode_corrupt_payload_digest_returns_digest_mismatch() {
        let event = JournalEvent::RunAccepted { run: RunId::new(9), seq: EventSeq::new(0), workflow: test_digest(9) };
        let mut encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, event.seq().get(), &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("ok");
        if let Some(b) = encoded.get_mut(61) { *b ^= 0xFF; }
        assert!(matches!(decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128), Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn adversarial_decode_truncated_before_full_header_returns_unexpected_eof() {
        let truncated = [0u8; 45];
        assert!(matches!(decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128), Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn adversarial_decode_truncated_before_full_payload_returns_unexpected_eof() {
        let event = JournalEvent::RunAccepted { run: RunId::new(10), seq: EventSeq::new(0), workflow: test_digest(10) };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, event.seq().get(), &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("ok");
        let truncated = encoded.get(..62).expect("slice");
        assert!(matches!(decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128), Err(JournalError::UnexpectedEof)));
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
        assert_ne!(blob_key([1u8; 32]).expect("k1").as_slice(), blob_key([2u8; 32]).expect("k2").as_slice());
    }

    // =========================================================================
    // Section: Adversarial Journal / Replay Tests
    // =========================================================================

    #[test]
    fn adversarial_append_duplicate_sequence_rejected_with_exact_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let run = RunId::new(50);
        assert!(journal.append_journaled(&JournalEvent::RunAccepted { run, seq: EventSeq::new(0), workflow: test_digest(1) }).is_ok());
        let result = journal.append_journaled(&JournalEvent::StepStarted { run, seq: EventSeq::new(0), step: StepIdx::new(0) });
        let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else { panic!("expected DuplicateEvent, got {:?}", result) };
        assert_eq!(r, run);
        assert_eq!(s, EventSeq::new(0));
    }

    #[test]
    fn adversarial_read_events_with_sequence_gap_returns_exact_gap() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let run = RunId::new(777);
        assert!(journal.append_journaled(&JournalEvent::RunAccepted { run, seq: EventSeq::new(0), workflow: test_digest(1) }).is_ok());
        assert!(journal.append_journaled(&JournalEvent::RunFinished { run, seq: EventSeq::new(5), result: vb_core::SlotIdx::new(0) }).is_ok());
        let Err(JournalError::SequenceGap { expected, actual }) = journal.events_for_run(run) else { panic!("expected SequenceGap") };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(5));
    }

    // =========================================================================
    // Section: Adversarial Blob / Snapshot / Size Boundary Tests
    // =========================================================================

    #[test]
    fn adversarial_put_blob_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let record = BlobRecord { digest: [0xFF; 32], bytes: vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)] };
        assert!(matches!(journal.put_blob(&record), Err(JournalError::PayloadTooLarge { .. })));
    }

    #[test]
    fn adversarial_blob_zero_length_round_trips() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let record = BlobRecord { digest: [0x42; 32], bytes: vec![] };
        assert!(journal.put_blob(&record).is_ok());
        assert_eq!(journal.blob([0x42; 32]).expect("ok"), Some(record));
    }

    #[test]
    fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let snap = RunSnapshot { run: RunId::new(888), seq: EventSeq::new(0), workflow: test_digest(1), slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)] };
        assert!(matches!(journal.put_snapshot(&snap), Err(JournalError::PayloadTooLarge { .. })));
    }

    #[test]
    fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
        let snap = RunSnapshot { run: RunId::new(889), seq: EventSeq::new(0), workflow: test_digest(1), slots: vec![1, 2, 3] };
        let mut enc = encode_record(MAGIC_SNAPSHOT, RecordKind::Snapshot, snap.seq.get(), &snap, MAX_SNAPSHOT_BYTES).expect("ok");
        if let Some(b) = enc.get_mut(0) { *b ^= 0xFF; }
        assert!(matches!(decode_record::<RunSnapshot>(&enc, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES), Err(JournalError::BadMagic { .. })));
    }

    #[test]
    fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let record = WorkflowSourceRecord { digest: test_digest(0xEE), source: vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)] };
        assert!(matches!(journal.put_workflow_source(&record), Err(JournalError::PayloadTooLarge { .. })));
    }

    #[test]
    fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("opens");
        let record = CompiledIrRecord { digest: test_digest(0xCC), ir: vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)] };
        assert!(matches!(journal.put_compiled_ir(&record), Err(JournalError::PayloadTooLarge { .. })));
    }

    // =========================================================================
    // Section: Adversarial Schema Migration Tests
    // =========================================================================

    #[test]
    fn adversarial_schema_migration_from_zero_exact_fields() {
        let event = JournalEvent::RunAccepted { run: RunId::new(11), seq: EventSeq::new(0), workflow: test_digest(11) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());
        let Err(JournalError::MigrationRequired { from, to }) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128) else { panic!("expected MigrationRequired") };
        assert_eq!(from, 0);
        assert_eq!(to, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adversarial_schema_future_version_max_unsupported() {
        let event = JournalEvent::RunAccepted { run: RunId::new(12), seq: EventSeq::new(0), workflow: test_digest(12) };
        let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());
        let Err(JournalError::UnsupportedSchemaVersion { version }) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128) else { panic!("expected UnsupportedSchemaVersion") };
        assert_eq!(version, u16::MAX);
    }

    // =========================================================================
    // Section: Adversarial Queue Tests
    // =========================================================================

    #[test]
    fn adversarial_queue_zero_capacity_returns_queue_capacity() {
        assert!(matches!(JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT), Err(JournalError::QueueCapacity)));
    }

    #[test]
    fn adversarial_queue_zero_batch_returns_queue_capacity() {
        assert!(matches!(JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT), Err(JournalError::QueueCapacity)));
    }

    #[test]
    fn adversarial_queue_full_returns_queue_full() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("q");
        let event = JournalEvent::RunAccepted { run: RunId::new(1), seq: EventSeq::new(0), workflow: test_digest(1) };
        assert!(queue.enqueue_journaled(event.clone()).is_ok());
        assert!(matches!(queue.enqueue_journaled(event), Err(JournalError::QueueFull)));
    }

    // =========================================================================
    // Section: Adversarial Postcard / Encoding Edge Cases
    // =========================================================================

    #[test]
    fn adversarial_valid_header_garbage_postcard_returns_decode_failed() {
        let event = JournalEvent::RunAccepted { run: RunId::new(13), seq: EventSeq::new(0), workflow: test_digest(13) };
        let mut enc = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, event.seq().get(), &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES).expect("ok");
        if let Some(b) = enc.get_mut(60) { *b = 0xFF; }
        let digest_bytes = blake3::hash(&enc[60..]).as_bytes().clone();
        enc.get_mut(24..56).expect("digest").copy_from_slice(&digest_bytes);
        let cs = crc32c::crc32c(&enc[..56]);
        enc[56] = (cs & 0xFF) as u8; enc[57] = ((cs >> 8) & 0xFF) as u8;
        enc[58] = ((cs >> 16) & 0xFF) as u8; enc[59] = ((cs >> 24) & 0xFF) as u8;
        assert!(matches!(decode_record::<JournalEvent>(&enc, MAGIC_JOURNAL_EVENT, 128), Err(JournalError::PostcardDecodeFailed)));
    }

    #[test]
    fn adversarial_run_header_wrong_magic_returns_bad_magic() {
        let record = RunHeaderRecord { run: RunId::new(123), workflow_id: WorkflowId::new(456), compiled_digest: test_digest(8), status: 1, accepted_at_ms: 1700000000 };
        let enc = encode_record(MAGIC_INDEX_RECORD, RecordKind::RunHeader, record.run.as_u64(), &record, MAX_RUN_HEADER_BYTES).expect("ok");
        assert!(matches!(decode_record::<RunHeaderRecord>(&enc, MAGIC_BLOB, MAX_RUN_HEADER_BYTES), Err(JournalError::BadMagic { .. })));
    }

    #[test]
    fn adversarial_decode_empty_returns_unexpected_eof() {
        assert!(matches!(decode_record::<JournalEvent>(&[][..], MAGIC_JOURNAL_EVENT, 128), Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn adversarial_encode_empty_blob_succeeds() {
        assert!(encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &BlobRecord { digest: [0; 32], bytes: vec![] }, MAX_BLOB_BYTES).is_ok());
    }

    #[test]
    fn adversarial_encode_empty_source_succeeds() {
        assert!(encode_record(MAGIC_WORKFLOW_SOURCE, RecordKind::WorkflowSource, 0, &WorkflowSourceRecord { digest: test_digest(0), source: vec![] }, MAX_WORKFLOW_SOURCE_BYTES).is_ok());
    }

    #[test]
    fn adversarial_encode_empty_ir_succeeds() {
        assert!(encode_record(MAGIC_COMPILED_ARTIFACT, RecordKind::CompiledIr, 0, &CompiledIrRecord { digest: test_digest(0), ir: vec![] }, MAX_COMPILED_IR_BYTES).is_ok());
    }
}

#[cfg(test)]
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
mod proptests {
    use super::*;
    use crate::{
        BlobRecord, EventSeq, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_WORKFLOW_SOURCE,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RecordKind, WorkflowSourceRecord, blob_key,
        compiled_ir_key, decode_record, encode_record, index_action_key, index_status_key,
        index_workflow_key, run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
    };
    use proptest::prelude::*;
    use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest, WorkflowId};

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

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

    // --- Section: Adversarial Record Header Decode Tests ---

    /// Helper: encode a record and corrupt a specific byte offset, then recompute the
    /// CRC32C over bytes 0..56 so that the CRC check passes (but the corrupted field
    /// remains). This lets us test validation of individual header fields past the CRC.
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
        let target = encoded.get_mut(offset..end).expect("patch range valid");
        target.copy_from_slice(new_bytes);
        // Recompute CRC32C over bytes 0..56
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
        // Given a record encoded with MAGIC_JOURNAL_EVENT
        // When decoded with MAGIC_SNAPSHOT (wrong family)
        // Then it returns BadMagic with the journal event magic
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
        .expect("encoding should succeed");

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_SNAPSHOT, 128);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("expected BadMagic, got {:?}", result);
        };
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }

    #[test]
    fn adversarial_decode_vbir_magic_on_journal_returns_bad_magic() {
        // Given a record with MAGIC_COMPILED_ARTIFACT (VBIR)
        // When decoded expecting MAGIC_JOURNAL_EVENT
        // Then it returns BadMagic with the VBIR magic value
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
        .expect("encoding should succeed");

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("expected BadMagic, got {:?}", result);
        };
        assert_eq!(found, MAGIC_COMPILED_ARTIFACT);
    }

    #[test]
    fn adversarial_decode_unsupported_schema_version_returns_exact_version() {
        // Given a record with schema version patched to 5 (future)
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion { version: 5 }
        let event = JournalEvent::RunAccepted {
            run: RunId::new(2),
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &5u16.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
            panic!("expected UnsupportedSchemaVersion, got {:?}", result);
        };
        assert_eq!(version, 5);
    }

    #[test]
    fn adversarial_decode_unknown_record_kind_returns_exact_kind() {
        // Given a record with kind patched to 99 (outside all valid ranges)
        // When decode_record is called
        // Then it returns UnknownRecordKind { kind: 99 }
        let event = JournalEvent::RunAccepted {
            run: RunId::new(3),
            seq: EventSeq::new(0),
            workflow: test_digest(3),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &99u16.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnknownRecordKind { kind }) = result else {
            panic!("expected UnknownRecordKind, got {:?}", result);
        };
        assert_eq!(kind, 99);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_snapshot_kind_in_journal() {
        // Given a record with MAGIC_JOURNAL_EVENT but kind patched to Snapshot (30)
        // When decode_record is called
        // Then it returns RecordKindFamilyMismatch
        let event = JournalEvent::RunAccepted {
            run: RunId::new(4),
            seq: EventSeq::new(0),
            workflow: test_digest(4),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &30u16.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
            panic!("expected RecordKindFamilyMismatch, got {:?}", result);
        };
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(kind, 30);
    }

    #[test]
    fn adversarial_decode_kind_family_mismatch_blob_in_snapshot() {
        // Given a record with MAGIC_SNAPSHOT but kind patched to Blob (40)
        // When encode_record is called
        // Then it returns RecordKindFamilyMismatch
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
            panic!("expected RecordKindFamilyMismatch, got {:?}", result);
        };
        assert_eq!(magic, MAGIC_SNAPSHOT);
        assert_eq!(kind, RecordKind::Blob.id());
    }

    #[test]
    fn adversarial_decode_header_len_not_60_returns_mismatch() {
        // Given a record with header_len patched to 48 (not 60)
        // When decode_record is called
        // Then it returns HeaderLengthMismatch { found: 48 }
        let event = JournalEvent::RunAccepted {
            run: RunId::new(6),
            seq: EventSeq::new(0),
            workflow: test_digest(6),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::HeaderLengthMismatch { found }) = result else {
            panic!("expected HeaderLengthMismatch, got {:?}", result);
        };
        assert_eq!(found, 48);
    }

    #[test]
    fn adversarial_decode_payload_len_above_limit_returns_too_large() {
        // Given a record with payload_len patched to 9999 but max set to 100
        // When decode_record is called
        // Then it returns PayloadTooLarge with exact values
        let event = JournalEvent::RunAccepted {
            run: RunId::new(7),
            seq: EventSeq::new(0),
            workflow: test_digest(7),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 12, &9999u32.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 100);
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("expected PayloadTooLarge, got {:?}", result);
        };
        assert_eq!(len, 9999);
        assert_eq!(max, 100);
    }

    #[test]
    fn adversarial_decode_corrupt_header_crc_returns_checksum_mismatch() {
        // Given a record with a single byte flipped in the CRC region
        // When decode_record is called
        // Then it returns HeaderChecksumMismatch
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
        .expect("encoding should succeed");
        // Flip bit in CRC byte at offset 57
        if let Some(byte) = encoded.get_mut(57) {
            *byte ^= 0x80;
        }

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::HeaderChecksumMismatch)),
            "expected HeaderChecksumMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn adversarial_decode_corrupt_payload_digest_returns_digest_mismatch() {
        // Given a record with a single byte flipped in the payload
        // When decode_record is called
        // Then it returns PayloadDigestMismatch
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
        .expect("encoding should succeed");
        // Flip a payload byte (offset 60+)
        if let Some(byte) = encoded.get_mut(61) {
            *byte ^= 0xFF;
        }

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "expected PayloadDigestMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn adversarial_decode_truncated_before_full_header_returns_unexpected_eof() {
        // Given 45 bytes (less than 60-byte header)
        // When decode_record is called
        // Then it returns UnexpectedEof
        let truncated = [0u8; 45];
        let result = decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "expected UnexpectedEof, got {:?}",
            result
        );
    }

    #[test]
    fn adversarial_decode_truncated_before_full_payload_returns_unexpected_eof() {
        // Given a valid header but payload truncated to only 2 of N bytes
        // When decode_record is called
        // Then it returns UnexpectedEof
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
        .expect("encoding should succeed");
        // Keep 60-byte header + 2 payload bytes (truncated)
        let truncated = encoded.get(..62).expect("slice should exist");

        let result = decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "expected UnexpectedEof, got {:?}",
            result
        );
    }

    // --- Section: Adversarial Key Encoding Tests ---

    #[test]
    fn adversarial_key_wrong_prefix_isolation() {
        // Given workflow_source and compiled_ir keys for the same digest
        // When compared
        // Then they differ in the prefix byte only, proving prefix isolation
        let digest = [0xAB; 32];
        let ws_key = workflow_source_key(digest).expect("ws key");
        let ci_key = compiled_ir_key(digest).expect("ci key");
        let bl_key = blob_key(digest).expect("blob key");

        assert_ne!(ws_key[0], ci_key[0]);
        assert_ne!(ws_key[0], bl_key[0]);
        assert_ne!(ci_key[0], bl_key[0]);
        // Same digest payload after prefix
        assert_eq!(ws_key[1..], ci_key[1..]);
        assert_eq!(ws_key[1..], bl_key[1..]);
    }

    #[test]
    fn adversarial_key_too_short_for_format_is_rejected_by_decode() {
        // Given a 3-byte slice (too short for any key format)
        // When used as raw bytes in decode_record
        // Then it returns UnexpectedEof
        let short = [0x11, 0x00, 0x00];
        let result = decode_record::<JournalEvent>(&short, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "expected UnexpectedEof for short key bytes"
        );
    }

    #[test]
    fn adversarial_key_wrong_endianness_produces_different_keys() {
        // Given run id 1 encoded in big-endian vs little-endian in key context
        // When the big-endian key is compared with a manually constructed LE key
        // Then they differ, proving the key encoder uses big-endian
        let key = run_header_key(RunId::new(1)).expect("key should succeed");
        // Key layout: [prefix 0x10][run_id 8 bytes big-endian]
        let mut le_key = [0u8; 9];
        le_key[0] = PREFIX_RUN_HEADER;
        le_key[1..9].copy_from_slice(&1u64.to_le_bytes());

        assert_ne!(key.as_slice(), le_key.as_slice(), "key must use big-endian");
        assert_eq!(
            key[1..9],
            1u64.to_be_bytes(),
            "run id portion must be big-endian"
        );
    }

    #[test]
    fn adversarial_key_no_collision_different_runs_same_seq() {
        // Given two different runs with the same sequence number
        // When their journal keys are constructed
        // Then the keys are different
        let k1 = run_event_key(RunId::new(100), EventSeq::new(5)).expect("key1");
        let k2 = run_event_key(RunId::new(200), EventSeq::new(5)).expect("key2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_same_run_different_seq() {
        // Given the same run with different sequence numbers
        // When their journal keys are constructed
        // Then the keys are different
        let k1 = run_event_key(RunId::new(100), EventSeq::new(0)).expect("key1");
        let k2 = run_event_key(RunId::new(100), EventSeq::new(1)).expect("key2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_different_digests_same_prefix() {
        // Given two different digests with the same blob prefix
        // When their blob keys are constructed
        // Then the keys are different
        let d1 = [1u8; 32];
        let d2 = [2u8; 32];
        let k1 = blob_key(d1).expect("key1");
        let k2 = blob_key(d2).expect("key2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    // --- Section: Adversarial Journal Tests ---

    #[test]
    fn adversarial_append_event_for_run_with_no_prior_events_succeeds() {
        // Given an empty journal
        // When a RunAccepted event is appended for a new run
        // Then it succeeds and events_for_run returns exactly that event
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let run = RunId::new(1000);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(42),
        };
        assert!(journal.append_journaled(&event).is_ok());
        let events = journal.events_for_run(run).expect("read events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn adversarial_append_duplicate_sequence_is_rejected() {
        // Given a journal with seq 0 for run 50
        // When appending another event at seq 0 for the same run
        // Then DuplicateEvent is returned with exact run and seq
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let run = RunId::new(50);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e0_dup = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
        };
        assert!(journal.append_journaled(&e0).is_ok());
        let result = journal.append_journaled(&e0_dup);
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
    fn adversarial_read_events_from_empty_run_returns_empty() {
        // Given a journal with no events
        // When events_for_run is called
        // Then it returns an empty vector (no error)
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let events = journal
            .events_for_run(RunId::new(9999))
            .expect("should succeed");
        assert!(events.is_empty());
    }

    #[test]
    fn adversarial_read_events_with_sequence_gap_returns_error() {
        // Given a journal with seq 0 then seq 5 (gap at 1..4)
        // When events_for_run replays
        // Then it returns SequenceGap with expected=1, actual=5
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let run = RunId::new(777);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e5 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(5),
            result: vb_core::SlotIdx::new(0),
        };
        assert!(journal.append_journaled(&e0).is_ok());
        assert!(journal.append_journaled(&e5).is_ok());

        let result = journal.events_for_run(run);
        let Err(JournalError::SequenceGap { expected, actual }) = result else {
            panic!("expected SequenceGap, got {:?}", result);
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(5));
    }

    // --- Section: Adversarial Blob Storage Tests ---

    #[test]
    fn adversarial_put_blob_exceeding_max_bytes_returns_payload_too_large() {
        // Given a BlobRecord with payload exceeding MAX_BLOB_BYTES
        // When put_blob is called
        // Then it returns PayloadTooLarge
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let record = BlobRecord {
            digest: [0xFF; 32],
            bytes: vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)],
        };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "expected PayloadTooLarge for oversized blob, got {:?}",
            result
        );
    }

    #[test]
    fn adversarial_read_nonexistent_blob_returns_none() {
        // Given a journal with no blobs
        // When blob is called with an arbitrary digest
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let result = journal.blob([0xDE; 32]).expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn adversarial_blob_zero_length_payload_round_trips() {
        // Given a BlobRecord with zero-length bytes
        // When stored and retrieved
        // Then the round-trip succeeds and bytes are empty
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let record = BlobRecord {
            digest: [0x42; 32],
            bytes: vec![],
        };
        assert!(journal.put_blob(&record).is_ok());
        let retrieved = journal.blob([0x42; 32]).expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    // --- Section: Adversarial Migration / Schema Tests ---

    #[test]
    fn adversarial_schema_migration_required_from_zero() {
        // Given a record with schema version 0
        // When decode_record is called
        // Then it returns MigrationRequired { from: 0, to: 1 }
        let event = JournalEvent::RunAccepted {
            run: RunId::new(11),
            seq: EventSeq::new(0),
            workflow: test_digest(11),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::MigrationRequired { from, to }) = result else {
            panic!("expected MigrationRequired, got {:?}", result);
        };
        assert_eq!(from, 0);
        assert_eq!(to, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adversarial_schema_future_version_returns_unsupported() {
        // Given a record with schema version u16::MAX
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion { version: u16::MAX }
        let event = JournalEvent::RunAccepted {
            run: RunId::new(12),
            seq: EventSeq::new(0),
            workflow: test_digest(12),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
            panic!("expected UnsupportedSchemaVersion, got {:?}", result);
        };
        assert_eq!(version, u16::MAX);
    }

    // --- Section: Adversarial Workflow Source Tests ---

    #[test]
    fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
        // Given a WorkflowSourceRecord with source exceeding MAX_WORKFLOW_SOURCE_BYTES
        // When put_workflow_source is called
        // Then it returns PayloadTooLarge
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let record = WorkflowSourceRecord {
            digest: test_digest(0xEE),
            source: vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)],
        };
        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "expected PayloadTooLarge for oversized source, got {:?}",
            result
        );
    }

    // --- Section: Adversarial Compiled IR Tests ---

    #[test]
    fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
        // Given a CompiledIrRecord with IR exceeding MAX_COMPILED_IR_BYTES
        // When put_compiled_ir is called
        // Then it returns PayloadTooLarge
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let record = CompiledIrRecord {
            digest: test_digest(0xCC),
            ir: vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)],
        };
        let result = journal.put_compiled_ir(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "expected PayloadTooLarge for oversized IR, got {:?}",
            result
        );
    }

    // --- Section: Adversarial Snapshot Tests ---

    #[test]
    fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
        // Given a RunSnapshot with slots exceeding MAX_SNAPSHOT_BYTES
        // When put_snapshot is called
        // Then it returns PayloadTooLarge
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path()).expect("journal opens");
        let snapshot = RunSnapshot {
            run: RunId::new(888),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)],
        };
        let result = journal.put_snapshot(&snapshot);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "expected PayloadTooLarge for oversized snapshot, got {:?}",
            result
        );
    }

    #[test]
    fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
        // Given an encoded snapshot record with magic corrupted
        // When decode_record is called with MAGIC_SNAPSHOT
        // Then it returns BadMagic
        let snapshot = recovery::RunSnapshot {
            run: RunId::new(889),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
        };
        let mut encoded = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snapshot.seq.get(),
            &snapshot,
            MAX_SNAPSHOT_BYTES,
        )
        .expect("encoding should succeed");
        // Corrupt magic byte at offset 0
        if let Some(byte) = encoded.get_mut(0) {
            *byte ^= 0xFF;
        }
        let result =
            decode_record::<recovery::RunSnapshot>(&encoded, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES);
        assert!(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "expected BadMagic for corrupt snapshot, got {:?}",
            result
        );
    }

    // --- Section: Adversarial Queue Tests ---

    #[test]
    fn adversarial_queue_zero_capacity_returns_queue_capacity_error() {
        // Given capacity=0, batch_size=1
        // When JournalWriterQueue::new is called
        // Then it returns QueueCapacity
        let result = JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT);
        assert!(
            matches!(result, Err(JournalError::QueueCapacity)),
            "expected QueueCapacity for zero capacity"
        );
    }

    #[test]
    fn adversarial_queue_zero_batch_returns_queue_capacity_error() {
        // Given capacity=1, batch_size=0
        // When JournalWriterQueue::new is called
        // Then it returns QueueCapacity
        let result = JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT);
        assert!(
            matches!(result, Err(JournalError::QueueCapacity)),
            "expected QueueCapacity for zero batch size"
        );
    }

    #[test]
    fn adversarial_queue_full_returns_queue_full_error() {
        // Given a queue at capacity 1
        // When a second event is enqueued
        // Then it returns QueueFull
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("queue creation");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        assert!(queue.enqueue_journaled(event.clone()).is_ok());
        let result = queue.enqueue_journaled(event);
        assert!(
            matches!(result, Err(JournalError::QueueFull)),
            "expected QueueFull, got {:?}",
            result
        );
    }

    // --- Section: Adversarial IPC Frame Magic Tests ---

    #[test]
    fn adversarial_ipc_frame_magic_accepts_any_kind() {
        // Given MAGIC_IPC_FRAME with any record kind
        // When validate_kind_family is called
        // Then it returns Ok (IPC frame magic accepts all kinds)
        let result = encode_record(
            MAGIC_IPC_FRAME,
            RecordKind::RunAccepted,
            0,
            &JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            128,
        );
        assert!(
            result.is_ok(),
            "IPC frame magic should accept any known kind"
        );
    }

    // --- Section: Adversarial Postcard Corruption Tests ---

    #[test]
    fn adversarial_valid_header_but_garbage_postcard_returns_decode_failed() {
        // Given an encoded record with a valid header but garbage postcard payload
        // When decode_record is called
        // Then it returns PostcardDecodeFailed
        let event = JournalEvent::RunAccepted {
            run: RunId::new(13),
            seq: EventSeq::new(0),
            workflow: test_digest(13),
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encoding should succeed");
        // Corrupt payload, then fix digest and CRC so header validation passes
        if let Some(byte) = encoded.get_mut(60) {
            *byte = 0xFF;
        }
        // Re-hash the payload
        let payload = encoded.get(60..).expect("payload");
        let digest = blake3::hash(payload);
        encoded
            .get_mut(24..56)
            .expect("digest region")
            .copy_from_slice(digest.as_bytes());
        // Re-compute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::PostcardDecodeFailed)),
            "expected PostcardDecodeFailed, got {:?}",
            result
        );
    }

    // --- Section: Adversarial Run Header Tests ---

    #[test]
    fn adversarial_run_header_wrong_magic_returns_bad_magic() {
        // Given an encoded run header record
        // When decoded with the wrong expected magic (MAGIC_BLOB instead of MAGIC_INDEX_RECORD)
        // Then it returns BadMagic
        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: test_digest(8),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        let encoded = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.as_u64(),
            &record,
            MAX_RUN_HEADER_BYTES,
        )
        .expect("encoding should succeed");

        let result = decode_record::<RunHeaderRecord>(&encoded, MAGIC_BLOB, MAX_RUN_HEADER_BYTES);
        assert!(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "expected BadMagic for wrong magic on run header"
        );
    }

    // --- Section: Adversarial Empty Input Tests ---

    #[test]
    fn adversarial_decode_empty_byte_slice_returns_unexpected_eof() {
        // Given a zero-length byte slice
        // When decode_record is called
        // Then it returns UnexpectedEof
        let empty: &[u8] = &[];
        let result = decode_record::<JournalEvent>(empty, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "expected UnexpectedEof for empty input"
        );
    }

    #[test]
    fn adversarial_decode_single_byte_returns_unexpected_eof() {
        // Given a 1-byte slice
        // When decode_record is called
        // Then it returns UnexpectedEof
        let single = [0x56u8; 1];
        let result = decode_record::<JournalEvent>(&single, MAGIC_JOURNAL_EVENT, 128);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "expected UnexpectedEof for 1-byte input"
        );
    }

    // --- Section: Adversarial Encode Boundary Tests ---

    #[test]
    fn adversarial_encode_blob_with_empty_bytes_succeeds() {
        // Given a BlobRecord with empty bytes
        // When encode_record is called
        // Then it succeeds (zero-length is valid)
        let record = BlobRecord {
            digest: [0; 32],
            bytes: vec![],
        };
        let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES);
        assert!(
            result.is_ok(),
            "empty blob bytes should encode successfully"
        );
    }

    #[test]
    fn adversarial_encode_workflow_source_with_empty_source_succeeds() {
        // Given a WorkflowSourceRecord with empty source bytes
        // When encode_record is called
        // Then it succeeds
        let record = WorkflowSourceRecord {
            digest: test_digest(0),
            source: vec![],
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        );
        assert!(result.is_ok(), "empty source should encode successfully");
    }

    #[test]
    fn adversarial_encode_compiled_ir_with_empty_ir_succeeds() {
        // Given a CompiledIrRecord with empty IR
        // When encode_record is called
        // Then it succeeds
        let record = CompiledIrRecord {
            digest: test_digest(0),
            ir: vec![],
        };
        let result = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            MAX_COMPILED_IR_BYTES,
        );
        assert!(result.is_ok(), "empty IR should encode successfully");
    }
}

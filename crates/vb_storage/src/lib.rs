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
mod tests {
    use super::{
        BlobRecord, CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent,
        MAGIC_COMPILED_ARTIFACT, MAGIC_JOURNAL_EVENT, MAGIC_WORKFLOW_SOURCE,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
        blob_key, compiled_ir_key, decode_record, encode_record, index_action_key,
        index_status_key, index_workflow_key, journal_key, run_event_key, run_header_key,
        run_snapshot_key, workflow_source_key,
    };
    use crate::recovery::RunSnapshot;
    use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest, WorkflowId};

    #[test]
    fn journal_key_is_fixed_width() {
        let key = journal_key(RunId::new(1), EventSeq::new(2));

        assert!(matches!(key, Ok(bytes) if bytes.len() == 17));
    }

    #[test]
    fn run_event_key_uses_required_prefix_and_big_endian_layout() {
        let key = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9));

        assert!(matches!(
            key,
            Ok(bytes) if bytes == [
                0x11, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09,
            ]
        ));
    }

    #[test]
    fn key_encoders_use_required_lengths() {
        let digest = [7_u8; 32];

        assert!(matches!(workflow_source_key(digest), Ok(bytes) if bytes.len() == 33));
        assert!(matches!(compiled_ir_key(digest), Ok(bytes) if bytes.len() == 33));
        assert!(matches!(run_header_key(RunId::new(1)), Ok(bytes) if bytes.len() == 9));
        assert!(
            matches!(run_snapshot_key(RunId::new(1), EventSeq::new(2)), Ok(bytes) if bytes.len() == 17)
        );
        assert!(matches!(blob_key(digest), Ok(bytes) if bytes.len() == 33));
        assert!(matches!(index_status_key(3, 4, RunId::new(5)), Ok(bytes) if bytes.len() == 18));
        assert!(
            matches!(index_workflow_key(WorkflowId::new(6), RunId::new(7)), Ok(bytes) if bytes.len() == 13)
        );
        assert!(
            matches!(index_action_key(ActionId::new(8), RunId::new(9), StepIdx::new(10)), Ok(bytes) if bytes.len() == 13)
        );
    }

    #[test]
    fn envelope_round_trips_and_reports_metadata() {
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
        assert!(matches!(&encoded, Ok(bytes) if bytes.len() > 60));
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

        assert!(matches!(
            decoded,
            Ok((envelope, decoded_event))
                if envelope.magic == MAGIC_JOURNAL_EVENT
                    && envelope.record_kind == RecordKind::RunFinished.id()
                    && envelope.sequence == 12
                    && decoded_event == event
        ));
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

        assert!(
            matches!(journal.workflow_source(workflow_digest), Ok(Some(record)) if record == source)
        );
        assert!(matches!(journal.compiled_ir(compiled_digest), Ok(Some(record)) if record == ir));
        assert!(matches!(journal.run_header(RunId::new(3)), Ok(Some(record)) if record == header));
        assert!(
            matches!(journal.snapshot(RunId::new(3), EventSeq::new(7)), Ok(Some(record)) if record == snapshot)
        );
        assert!(matches!(journal.blob([9; 32]), Ok(Some(record)) if record == blob));
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
        let replay = journal.events_for_run(run);

        assert!(matches!(replay, Ok(events) if events == vec![accepted, finished]));
    }
}

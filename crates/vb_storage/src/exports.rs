//! Public re-exports from submodules.
//!
//! Consumers of `vb_storage` import from this module (re-exported at the crate root).

// Core types
pub use crate::constants::{
    CRC_OFFSET, CURRENT_SCHEMA_VERSION, DIGEST_BYTES, KEYSPACE_BLOB, KEYSPACE_COMPILED_IR,
    KEYSPACE_INDEX_ACTION, KEYSPACE_INDEX_STATUS, KEYSPACE_INDEX_WORKFLOW, KEYSPACE_RECOVERY_STAMP,
    KEYSPACE_RUN_EVENT, KEYSPACE_RUN_HEADER, KEYSPACE_RUN_SNAPSHOT, KEYSPACE_WORKFLOW_SOURCE,
    MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME, MAGIC_JOURNAL_EVENT,
    MAGIC_RECOVERY_STAMP, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BATCH_COUNT, MAX_BLOB_BYTES,
    MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RECOVERY_STAMP_BYTES,
    MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB,
    PREFIX_COMPILED_IR, PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW,
    PREFIX_RECOVERY_STAMP, PREFIX_RUN_EVENT, PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT,
    PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES, RECORD_HEADER_LEN, RECOVERY_STAMP_KEY_BYTES,
};
pub use crate::error::{JournalError, KeyDecodeError};
pub use crate::events::{DurableActionOutcome, JournalEvent, SlotWriteExtra};
pub use crate::records::{
    BlobRecord, CompiledIrRecord, RecordKind, RecoveryStampRecord, RunHeaderRecord,
    WorkflowSourceRecord,
};
pub use crate::recovery::{ActionReplayTracker, RunSnapshot};
pub use crate::slot_extra::{
    DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraEnvelope,
    SlotWrittenExtraError, decode_slot_written_extra, encode_slot_written_extra,
};
pub use crate::types::*;

// Journal
pub use crate::journal::incident::{
    IncidentAnalysis, SideEffect, SideEffectCertainty, analyze_incident_events, build_repair_hints,
    derive_lifecycle_state_from_events, lifecycle_state_to_inspect_status,
};
pub use crate::journal::incident::{lifecycle, model, repair};
pub use crate::journal::{EventReplayLimit, FjallJournal, ReadOnlyJournal};

// Batch
pub use crate::batch::JournalWriteBatch;

// Queue
pub use crate::queue::JournalWriterQueue;

// Types
pub use crate::types::JournalWriterFlushReport;

// Trimming
pub use crate::trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};

// Codec
pub use crate::codec::{
    decode_envelope_only, decode_record, decode_record_header, encode_record, encode_record_header,
    verify_digest_match,
};

// Admission
pub use crate::admission::{
    AcceptedArtifact, Durability, VerificationProof, VerificationWarning, admit_compiled_artifact,
    submit_artifact, submit_artifact_with_contracts,
};

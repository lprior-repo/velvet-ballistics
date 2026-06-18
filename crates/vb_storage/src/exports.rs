//! Public re-exports from submodules.
//!
//! Consumers of `vb_storage` import from this module (re-exported at the crate root).

// Core types
pub use crate::constants::*;
pub use crate::error::{JournalError, KeyDecodeError};
pub use crate::events::{DurableActionOutcome, JournalEvent};
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
    AcceptedArtifact, VerificationProof, VerificationWarning, admit_compiled_artifact,
    submit_artifact, submit_artifact_with_contracts,
};

#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

// ============================================================================
// Submodules
// ============================================================================

pub mod admission;
pub mod artifacts;
pub mod batch;
pub mod binary;
pub mod blobs;
pub mod codec;
#[cfg(miri)]
pub mod codec_miri_tests;
pub mod constants;
pub mod error;
pub mod events;
pub mod headers;
pub mod indexes;
pub mod journal;
#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_codec;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_magic;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_schema;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_kind;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_payload_len;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_crc;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_digest_checks_vb_2bzz;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_hydrate_proofs;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_admission;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_postcard_envelope_wire;

#[cfg(all(kani, feature = "kani-typed-partitioned-ids"))]
pub mod kani_typed_partitioned_ids;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_decode_order;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_numeric_fields;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_payload_bounds;

// --- vb-vzcuf Kani harnesses (PS-001 through PS-009) ---
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps001;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps002;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps003;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps004;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps005;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps006;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps007;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps008;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps009;

#[cfg(kani)]
pub mod kani_vbjpq733_proofs;

pub mod keys;
pub mod preview;
pub mod process_lock;
mod public_api;

// PO-010: register the deterministic replay proptest module for `cargo test --lib`
// evidence collection. This is test-only verification wiring and does not alter
// production runtime behavior.
#[cfg(test)]
#[path = "po010_proptests.rs"]
mod proptests;

// vb-b8i8f: proptest_storage.rs disabled — proptest 1.11.0 block-form
// incompatibility. File requires rewrite to single-test form.
// Will be fixed in follow-up bead. See LANDING-NOTE-001.
// #[cfg(test)]
// #[path = "proptest_storage.rs"]
// mod proptest_storage;

#[cfg(test)]
#[path = "proptests.rs"]
mod proptest_integration;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;

#[cfg(test)]
#[path = "error_code_tests.rs"]
mod error_code_tests;

#[cfg(test)]
#[path = "edge_case_tests.rs"]
mod edge_case_tests;

#[cfg(test)]
#[path = "type_tests.rs"]
mod type_tests;

#[cfg(test)]
#[path = "index_tests.rs"]
mod index_tests;

// vb-3wn7x: pending action index maintenance contract tests.
#[cfg(test)]
#[path = "index_maintenance_tests.rs"]
mod index_maintenance_tests;

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod artifact_tests;

#[cfg(test)]
#[path = "blob_tests.rs"]
mod blob_tests;

#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;

#[cfg(test)]
#[path = "hydrate_tests.rs"]
mod hydrate_tests;

#[cfg(test)]
#[path = "process_lock_tests.rs"]
mod process_lock_tests;

#[cfg(test)]
#[path = "record_tests.rs"]
mod record_tests;

#[cfg(test)]
#[path = "recover_tests.rs"]
mod recover_tests;

#[cfg(test)]
#[path = "recovery_type_tests.rs"]
mod recovery_type_tests;

#[cfg(test)]
#[path = "replay_core_tests.rs"]
mod replay_core_tests;

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;

pub mod queue;
pub mod records;
pub mod recovery;
pub mod security_tests;
pub mod slot_extra;
pub mod snapshots;
pub mod tests;
pub mod trimming;
pub mod types;
pub mod vb_2bok_durability_gate_tests;

// ============================================================================
// Re-exports from submodules
// ============================================================================

pub use public_api::*;

// Core types
pub use constants::*;
pub use error::{JournalError, KeyDecodeError};
pub use events::{DurableActionOutcome, JournalEvent};
pub use records::{
    BlobRecord, CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
};
pub use recovery::{
    ActionReplayTracker, RunSnapshot, JournalObservation, JournalObservationSignature,
    ObservationSignatureError, semantic_observation_signature,
};
pub use slot_extra::{
    DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraEnvelope,
    SlotWrittenExtraError, decode_slot_written_extra, encode_slot_written_extra,
};
pub use types::*;

// Journal
pub use journal::incident::{
    IncidentAnalysis, SideEffect, SideEffectCertainty, analyze_incident_events, build_repair_hints,
    derive_lifecycle_state_from_events, event_to_lifecycle, lifecycle_state_to_inspect_status,
};
pub use journal::{EventReplayLimit, FjallJournal, ReadOnlyJournal};

// Batch
pub use batch::JournalWriteBatch;

// Queue
pub use queue::JournalWriterQueue;

// Types
pub use types::JournalWriterFlushReport;

// Trimming
pub use trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};

// Codec
pub use codec::{
    decode_journal_event, decode_record, decode_record_header, encode_record, encode_record_header,
    validate_journal_event_record_kind, verify_digest_match,
};

// Admission
pub use admission::{
    AcceptedArtifact, VerificationProof, VerificationWarning, admit_compiled_artifact,
    submit_artifact, submit_artifact_with_contracts,
};

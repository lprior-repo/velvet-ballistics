#![forbid(unsafe_code)]
//! Durability gate test fixtures and helpers.
//!
//! Shared across all durability gate test submodules.

use crate::{FjallJournal, JournalError};
use vb_core::CompiledWorkflow;

/// Create a temporary journal backed by a `TempDir`.
/// The temp dir is returned alongside the journal so it lives as long as the journal.
pub fn temp_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
    let temp = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
    let journal = FjallJournal::open(temp.path(), None)?;
    Ok((temp, journal))
}

/// Build a minimal valid workflow with a correct digest.
pub fn minimal_valid_workflow() -> Result<CompiledWorkflow, String> {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ResourceContract, WorkflowParts};
    use vb_core::{CompiledNode, CompiledNodeKind, ConstIdx, SlotIdx, StepIdx};

    let mut parts = WorkflowParts {
        name: Box::<str>::from("vb_2bok_test"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let hash_bytes = postcard::to_allocvec(&parts)
        .map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = vb_core::WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

/// Submit an artifact into a fresh journal for convenience in tests that need multiple policies.
pub use submit_artifact_in_fresh_journal;

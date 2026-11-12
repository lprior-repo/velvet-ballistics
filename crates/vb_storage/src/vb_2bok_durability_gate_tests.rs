#![forbid(unsafe_code)]
//! Durability Gate Tests for vb-2bok — RED PHASE
//!
//! These tests define expected behavior for the durability gate boundary
//! (vb_storage admission.rs).

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod durability_gate_tests {
    use crate::admission::{accepted_artifact_digest, admit_compiled_artifact, submit_artifact};
    use crate::codec::{decode_record, encode_record};
    use crate::constants::{
        CRC_OFFSET, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        RECORD_HEADER_BYTES,
    };
    use crate::records::RecordKind;
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, JournalEvent,
        WorkflowSourceRecord,
    };
    use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

    // =========================================================================
    // Test fixtures and helpers
    // =========================================================================

    fn temp_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
        let temp = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
        let journal = FjallJournal::open(temp.path(), None)?;
        Ok((temp, journal))
    }

    fn minimal_valid_workflow() -> Result<CompiledWorkflow, String> {
        use vb_core::value::ConstValue;
        use vb_core::workflow::{ResourceContract, WorkflowParts, node};
        use vb_core::{CompiledNodeKind, ConstIdx, SlotIdx, StepIdx};

        let mut parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::<str>::from("vb_2bok_test"),
            input_slots: Box::new([]),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::new([
                node(
                    StepIdx::new(0),
                    CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                    Some(SlotIdx::new(0)),
                    Some(StepIdx::new(1)),
                ),
                node(
                    StepIdx::new(1),
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    None,
                    None,
                ),
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
        parts.digest = WorkflowDigest::from_bytes(computed.into());

        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn compiled_record_digest(workflow: &CompiledWorkflow) -> Result<WorkflowDigest, String> {
        let bytes = postcard::to_allocvec(&workflow.to_parts())
            .map_err(|e| format!("serialize compiled workflow parts: {e}"))?;
        Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
    }

    include!("vb_2bok_durability_gate_tests/durability_gate_unit.rs");
    include!("vb_2bok_durability_gate_tests/durability_gate_codec_integration.rs");
    include!("vb_2bok_durability_gate_tests/durability_gate_error_taxonomy.rs");
}

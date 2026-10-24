use super::OwnedRunInputsRecordV1;
use crate::constants::{MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES};
use crate::{FjallJournal, JournalError, RecordKind, RunInputsRecord, codec};
use vb_core::{CompiledWorkflow, RunId, WorkflowDigest};

/// Typed retained input formats; scalar bytes retain their historical layout.
#[derive(Debug)]
pub enum RetainedRunInputs {
    /// Historical scalar retained inputs.
    Scalar(RunInputsRecord),
    /// Complete validated owned inputs.
    Owned(OwnedRunInputsRecordV1),
}

fn scalar(
    bytes: &[u8],
    run: RunId,
    artifact: WorkflowDigest,
    source: WorkflowDigest,
    workflow: &CompiledWorkflow,
) -> Result<RetainedRunInputs, JournalError> {
    let (_, record): (_, RunInputsRecord) =
        codec::decode_record(bytes, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES)?;
    if record.run() != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: record.run(),
        });
    }
    if record.artifact_digest() != artifact || record.source_digest() != source {
        return Err(super::OwnedInputError::IdentityMismatch.into());
    }
    record.validate_workflow(workflow)?;
    Ok(RetainedRunInputs::Scalar(record))
}

fn decode(
    bytes: &[u8],
    run: RunId,
    artifact: WorkflowDigest,
    source: WorkflowDigest,
    workflow: &CompiledWorkflow,
) -> Result<RetainedRunInputs, JournalError> {
    let header = codec::decode_record_header(bytes, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES)?;
    match RecordKind::from_id(header.record_kind) {
        Some(RecordKind::RunOwnedInputs) => {
            OwnedRunInputsRecordV1::decode(bytes, run, artifact, source, workflow)
                .map(RetainedRunInputs::Owned)
        }
        Some(RecordKind::RunInputs) => scalar(bytes, run, artifact, source, workflow),
        _ => Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: header.record_kind,
            payload_kind: RecordKind::RunOwnedInputs.id(),
        }),
    }
}

impl FjallJournal {
    /// Reads either retained format, checking explicit independently accepted bindings.
    pub fn retained_run_inputs(
        &self,
        run: RunId,
        artifact: WorkflowDigest,
        source: WorkflowDigest,
        workflow: &CompiledWorkflow,
    ) -> Result<Option<RetainedRunInputs>, JournalError> {
        let key = crate::keys::run_header_key(run)?;
        let Some(bytes) = self.run_inputs.get(key)? else {
            return Ok(None);
        };
        decode(&bytes, run, artifact, source, workflow).map(Some)
    }

    /// Reads only owned inputs; scalar records are never reinterpreted as owned.
    pub fn owned_run_inputs(
        &self,
        run: RunId,
        artifact: WorkflowDigest,
        source: WorkflowDigest,
        workflow: &CompiledWorkflow,
    ) -> Result<Option<OwnedRunInputsRecordV1>, JournalError> {
        match self.retained_run_inputs(run, artifact, source, workflow)? {
            Some(RetainedRunInputs::Owned(record)) => Ok(Some(record)),
            None => Ok(None),
            Some(RetainedRunInputs::Scalar(_)) => Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind: RecordKind::RunInputs.id(),
                payload_kind: RecordKind::RunOwnedInputs.id(),
            }),
        }
    }
}

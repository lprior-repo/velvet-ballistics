#![forbid(unsafe_code)]
//! Artifact-bound typed inputs retained at the run submission boundary.

mod digest;
mod owned;
mod submission;
mod validation;

use crate::constants::{MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES};
use crate::{FjallJournal, JournalError, RecordKind, codec::EnforceKindParity};
pub use digest::{RunInputBinding, RunPolicyPin, compute_input_digest};
pub use owned::{OwnedInputError, OwnedRunInputsRecordV1, RetainedRunInputs};
pub use submission::RunSubmission;
use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue, WorkflowDigest};

/// Immutable input mapping and its run, artifact, workflow and source identities.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunInputsRecord {
    run: RunId,
    artifact_digest: WorkflowDigest,
    workflow_digest: WorkflowDigest,
    source_digest: WorkflowDigest,
    input_digest: WorkflowDigest,
    inputs: Box<[(SlotIdx, SlotValue)]>,
}

impl RunInputsRecord {
    /// Constrains typed input mappings against the numeric workflow schema.
    pub fn new(
        run: RunId,
        artifact_digest: WorkflowDigest,
        source_digest: WorkflowDigest,
        workflow: &CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> Result<Self, JournalError> {
        crate::keys::run_header_key(run)?;
        validation::validate_inputs(workflow, &inputs)?;
        Ok(Self {
            run,
            artifact_digest,
            workflow_digest: workflow.digest(),
            source_digest,
            input_digest: compute_input_digest(&inputs)?,
            inputs,
        })
    }

    /// Run owning this mapping.
    pub const fn run(&self) -> RunId {
        self.run
    }
    /// Content-addressed accepted artifact.
    pub const fn artifact_digest(&self) -> WorkflowDigest {
        self.artifact_digest
    }
    /// Digest of the compiled numeric workflow.
    pub const fn workflow_digest(&self) -> WorkflowDigest {
        self.workflow_digest
    }
    /// BLAKE3 of original source bytes, for cold diagnostics only.
    pub const fn source_digest(&self) -> WorkflowDigest {
        self.source_digest
    }
    /// Canonical digest of the retained typed mapping.
    pub const fn input_digest(&self) -> WorkflowDigest {
        self.input_digest
    }
    /// Original validated typed values in supplied mapping order.
    pub fn inputs(&self) -> &[(SlotIdx, SlotValue)] {
        &self.inputs
    }

    /// Checks the normative admission receipt before using retained inputs for recovery.
    pub fn validate_admission(&self, event: &crate::JournalEvent) -> Result<(), JournalError> {
        match event {
            crate::JournalEvent::RunAdmission {
                run,
                artifact_digest,
                input_digest: RunInputBinding::Digest(input_digest),
                ..
            } if *run == self.run
                && *artifact_digest == self.artifact_digest
                && *input_digest == self.input_digest =>
            {
                Ok(())
            }
            _ => Err(JournalError::PayloadDigestMismatch),
        }
    }

    /// Revalidates retained input state against its bound executable workflow.
    pub fn validate_workflow(&self, workflow: &CompiledWorkflow) -> Result<(), JournalError> {
        if self.workflow_digest != workflow.digest()
            || compute_input_digest(&self.inputs)? != self.input_digest
        {
            return Err(JournalError::PayloadDigestMismatch);
        }
        validation::validate_inputs(workflow, &self.inputs)
    }
}

impl EnforceKindParity for RunInputsRecord {
    fn enforce_kind_parity(
        envelope: &crate::RecordEnvelope,
        value: &Self,
    ) -> Result<(), JournalError> {
        if envelope.record_kind != RecordKind::RunInputs.id() {
            return Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind: envelope.record_kind,
                payload_kind: RecordKind::RunInputs.id(),
            });
        }
        if envelope.sequence != value.run.get()
            || compute_input_digest(&value.inputs)? != value.input_digest
        {
            return Err(JournalError::PayloadDigestMismatch);
        }
        Ok(())
    }
}

impl FjallJournal {
    /// Reads retained inputs with envelope checksum and run identity validation.
    /// The consumer must validate the workflow and artifact binding before use.
    pub fn run_inputs(&self, run: RunId) -> Result<Option<RunInputsRecord>, JournalError> {
        let key = crate::keys::run_header_key(run)?;
        self.decode_optional_with(
            &self.run_inputs,
            &key,
            MAGIC_INDEX_RECORD,
            MAX_RUN_INPUT_BYTES,
            |_, record: &RunInputsRecord| {
                if record.run == run {
                    Ok(())
                } else {
                    Err(JournalError::WrongRun {
                        expected: run,
                        actual: record.run,
                    })
                }
            },
        )
    }
}

#[cfg(test)]
mod receipt_tests;
#[cfg(test)]
mod submission_tests;
#[cfg(test)]
mod tests;

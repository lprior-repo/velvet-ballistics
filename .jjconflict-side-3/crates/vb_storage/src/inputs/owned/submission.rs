use super::OwnedRunInputsRecordV1;
use crate::{JournalError, JournalWriteBatch, RunInputsRecord};
use vb_core::{CompiledWorkflow, RunId, WorkflowDigest};

pub(in crate::inputs) enum SubmissionInputs {
    Scalar(RunInputsRecord),
    Owned(OwnedRunInputsRecordV1),
}

impl SubmissionInputs {
    pub(in crate::inputs) fn run(&self) -> RunId {
        match self {
            Self::Scalar(v) => v.run(),
            Self::Owned(v) => v.run(),
        }
    }
    pub(in crate::inputs) fn artifact_digest(&self) -> WorkflowDigest {
        match self {
            Self::Scalar(v) => v.artifact_digest(),
            Self::Owned(v) => v.artifact_digest(),
        }
    }
    pub(in crate::inputs) fn workflow_digest(&self) -> WorkflowDigest {
        match self {
            Self::Scalar(v) => v.workflow_digest(),
            Self::Owned(v) => v.workflow_digest(),
        }
    }
    pub(in crate::inputs) fn source_digest(&self) -> WorkflowDigest {
        match self {
            Self::Scalar(v) => v.source_digest(),
            Self::Owned(v) => v.source_digest(),
        }
    }
    pub(in crate::inputs) fn input_digest(&self) -> WorkflowDigest {
        match self {
            Self::Scalar(v) => v.input_digest(),
            Self::Owned(v) => v.input_digest(),
        }
    }
    pub(in crate::inputs) fn validate_workflow(
        &self,
        wf: &CompiledWorkflow,
    ) -> Result<(), JournalError> {
        match self {
            Self::Scalar(v) => v.validate_workflow(wf),
            Self::Owned(v) => v.validate_workflow(wf),
        }
    }
    pub(in crate::inputs) fn stage(
        &self,
        batch: &mut JournalWriteBatch<'_>,
    ) -> Result<(), JournalError> {
        match self {
            Self::Scalar(v) => batch.put_run_inputs(v),
            Self::Owned(v) => batch.put_run_owned_inputs(v),
        }
    }
}

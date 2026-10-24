use super::wire::{Body, VERSION, Wire};
use super::{OwnedInputError, OwnedRunInputsRecordV1};
use crate::constants::{MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES};
use crate::{JournalError, JournalEvent, RecordKind, RunInputBinding, codec};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue, Taint, WorkflowDigest};

impl OwnedRunInputsRecordV1 {
    /// Retains the complete validated bundle and exact accepted projection.
    pub fn new(
        run: RunId,
        artifact: WorkflowDigest,
        source: WorkflowDigest,
        bundle: &OwnedRootBundle,
        workflow: &CompiledWorkflow,
    ) -> Result<Self, JournalError> {
        crate::keys::run_header_key(run)?;
        if !bundle.matches_workflow(workflow) {
            return Err(OwnedInputError::ProjectionMismatch.into());
        }
        let body = Body {
            version: VERSION,
            run,
            artifact,
            workflow: bundle.digest(),
            source,
            slots: bundle.frame_slot_count(),
            inputs: bundle.input_slots().to_vec().into_boxed_slice(),
            resources: *bundle.resource_contract(),
            roots: bundle.roots().to_vec().into_boxed_slice(),
            checkpoint: bundle.checkpoint().clone(),
        };
        let digest = body.digest()?;
        let record = Self {
            data: Box::new(Wire { body, digest }),
        };
        record.validate_encoded_size()?;
        Ok(record)
    }

    /// Decodes an exact envelope and reconstructs through the real bundle validator.
    pub fn decode(
        bytes: &[u8],
        run: RunId,
        artifact: WorkflowDigest,
        source: WorkflowDigest,
        workflow: &CompiledWorkflow,
    ) -> Result<Self, JournalError> {
        crate::keys::run_header_key(run)?;
        let (_, data) = codec::decode_record(bytes, MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES)?;
        let record = Self {
            data: Box::new(data),
        };
        if record.run() != run {
            return Err(JournalError::WrongRun {
                expected: run,
                actual: record.run(),
            });
        }
        if record.artifact_digest() != artifact || record.source_digest() != source {
            return Err(OwnedInputError::IdentityMismatch.into());
        }
        record.validate_workflow(workflow)?;
        Ok(record)
    }

    /// Revalidates all stored data against an independently accepted workflow.
    pub fn restore(&self, workflow: &CompiledWorkflow) -> Result<OwnedRootBundle, JournalError> {
        self.data.validate_integrity()?;
        let body = &self.data.body;
        if body.workflow != workflow.digest()
            || body.slots != workflow.slot_count()
            || body.inputs.as_ref() != workflow.input_slots()
            || body.resources != workflow.resource_contract()
        {
            return Err(OwnedInputError::ProjectionMismatch.into());
        }
        let bundle = OwnedRootBundle::new(body.roots.clone(), body.checkpoint.clone(), workflow)
            .map_err(OwnedInputError::Bundle)?;
        if !bundle.matches_workflow(workflow) {
            return Err(OwnedInputError::ProjectionMismatch.into());
        }
        if bundle.roots() != body.roots.as_ref() {
            return Err(OwnedInputError::NonCanonicalRoots.into());
        }
        Ok(bundle)
    }

    /// Checks projection, roots, arenas, references, taints, caps and digest.
    pub fn validate_workflow(&self, workflow: &CompiledWorkflow) -> Result<(), JournalError> {
        self.restore(workflow).map(drop)
    }

    /// Validates the actual admission's run/artifact/complete-input binding.
    pub fn validate_admission(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.data.validate_integrity()?;
        match event {
            JournalEvent::RunAdmission {
                run,
                artifact_digest,
                input_digest: RunInputBinding::Digest(digest),
                ..
            } if *run == self.run()
                && *artifact_digest == self.artifact_digest()
                && *digest == self.input_digest() =>
            {
                Ok(())
            }
            _ => Err(OwnedInputError::AdmissionMismatch.into()),
        }
    }

    fn validate_encoded_size(&self) -> Result<(), JournalError> {
        codec::encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunOwnedInputs,
            self.run().get(),
            self,
            MAX_RUN_INPUT_BYTES,
        )
        .map(drop)
    }

    /// Run owning all retained values.
    pub const fn run(&self) -> RunId {
        self.data.body.run
    }
    /// Accepted artifact identity.
    pub const fn artifact_digest(&self) -> WorkflowDigest {
        self.data.body.artifact
    }
    /// Complete retained workflow identity.
    pub const fn workflow_digest(&self) -> WorkflowDigest {
        self.data.body.workflow
    }
    /// Original authored source identity.
    pub const fn source_digest(&self) -> WorkflowDigest {
        self.data.body.source
    }
    /// Domain-separated complete owned-input identity.
    pub const fn input_digest(&self) -> WorkflowDigest {
        self.data.digest
    }
    /// Canonical roots and their aggregate taints.
    pub fn roots(&self) -> &[(SlotIdx, SlotValue, Taint)] {
        &self.data.body.roots
    }
    /// Complete identifier-ordered arenas and per-item taints.
    pub fn checkpoint(&self) -> &ValueStoreCheckpointV1 {
        &self.data.body.checkpoint
    }
}

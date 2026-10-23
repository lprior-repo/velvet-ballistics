use super::{ExecutorCatalog, ExecutorError};
use crate::admission::verification::{self, ExecutorProfile};
use crate::{AcceptedArtifact, FjallJournal};
use vb_core::action_pack::{ActionPack, PackedArtifactEvidence};
use vb_core::artifact::ArtifactEvidence;
use vb_core::{
    ActionContract, ActionId, CompiledWorkflow, RuntimePolicy, SideEffect, WorkflowParts,
};

const MAX_PACKS: usize = 64;

impl ExecutorCatalog {
    /// Cold preparation checks registered executor authority and actual conformance.
    pub fn prepare_artifact(
        &self,
        workflow: &CompiledWorkflow,
        policy: RuntimePolicy,
        contracts: &[ActionContract],
        packs: &[ActionPack],
    ) -> Result<AcceptedArtifact, ExecutorError> {
        self.check_packs(contracts, packs)?;
        Ok(verification::prepare_with_profile(
            workflow,
            policy,
            contracts,
            ExecutorProfile::Registered {
                catalog: self,
                packs,
            },
        )?)
    }

    /// Verify hostile serialized pack metadata against this actual registered catalog.
    pub fn verify_artifact(
        &self,
        artifact: &AcceptedArtifact,
        policy: RuntimePolicy,
    ) -> Result<CompiledWorkflow, ExecutorError> {
        let packed = checked_envelope(artifact, policy)?;
        let workflow = decode_workflow(artifact)?;
        let expected =
            self.prepare_artifact(&workflow, policy, &artifact.action_contracts, &packed.packs)?;
        verification::verify_expected_bindings(artifact, &expected)?;
        Ok(workflow)
    }

    /// Persist exact checked metadata and verify its strict durable readback.
    pub fn submit_artifact(
        &self,
        journal: &FjallJournal,
        workflow: &CompiledWorkflow,
        policy: RuntimePolicy,
        contracts: &[ActionContract],
        packs: &[ActionPack],
    ) -> Result<AcceptedArtifact, ExecutorError> {
        let artifact = self.prepare_artifact(workflow, policy, contracts, packs)?;
        let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
            journal,
            artifact,
            RuntimePolicy::Strict,
        )?;
        let stored = journal
            .compiled_ir(stamped.digest)?
            .ok_or(ExecutorError::Corrupt)?;
        let restored = postcard::from_bytes(&stored.ir)?;
        self.verify_artifact(&restored, policy)?;
        if stamped != restored {
            return Err(ExecutorError::Corrupt);
        }
        Ok(stamped)
    }

    pub(crate) fn check_packs(
        &self,
        contracts: &[ActionContract],
        packs: &[ActionPack],
    ) -> Result<(), ExecutorError> {
        if packs.is_empty()
            || packs.len() > MAX_PACKS
            || !packs
                .windows(2)
                .all(|pair| matches!(pair, [a, b] if a.action < b.action))
        {
            return Err(ExecutorError::PackMismatch);
        }
        let expected = contracts
            .iter()
            .filter(|contract| contract.side_effect != SideEffect::None)
            .count();
        if expected != packs.len() {
            return Err(ExecutorError::PackMismatch);
        }
        packs.iter().try_for_each(|pack| {
            let contract = contracts
                .get(usize::from(pack.action.get()))
                .ok_or(ExecutorError::PackMismatch)?;
            self.check_pack(contract, pack)
        })
    }

    pub(super) fn pack_for<'a>(
        &self,
        artifact: &'a AcceptedArtifact,
        action: ActionId,
    ) -> Result<&'a ActionPack, ExecutorError> {
        let ArtifactEvidence::Packed(packed) = &artifact.verification.evidence else {
            return Err(ExecutorError::CatalogRequired);
        };
        let index = packed
            .packs
            .binary_search_by_key(&action, |pack| pack.action)
            .map_err(|_| ExecutorError::PackMismatch)?;
        packed.packs.get(index).ok_or(ExecutorError::PackMismatch)
    }
}

fn checked_envelope(
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<&PackedArtifactEvidence, ExecutorError> {
    let ArtifactEvidence::Packed(packed) = &artifact.verification.evidence else {
        return Err(ExecutorError::CatalogRequired);
    };
    if packed.base.schema_version != verification::EVIDENCE_VERSION {
        return Err(crate::JournalError::UnsupportedSchemaVersion {
            version: packed.base.schema_version,
        }
        .into());
    }
    if packed.base.admission_policy != policy
        || packed.packs.len() > MAX_PACKS
        || artifact.digest != artifact.verification.digest
        || crate::admission::accepted_artifact_digest(artifact)? != artifact.digest
    {
        return Err(ExecutorError::PackMismatch);
    }
    Ok(packed)
}

fn decode_workflow(artifact: &AcceptedArtifact) -> Result<CompiledWorkflow, ExecutorError> {
    let len = u32::try_from(artifact.ir.len()).map_err(|_| ExecutorError::Capacity)?;
    if len > crate::constants::MAX_COMPILED_IR_BYTES {
        return Err(ExecutorError::Capacity);
    }
    let (parts, rest): (WorkflowParts, _) = postcard::take_from_bytes(&artifact.ir)?;
    if !rest.is_empty() {
        return Err(ExecutorError::Corrupt);
    }
    CompiledWorkflow::try_from_parts(parts).map_err(|source| {
        ExecutorError::Artifact(crate::JournalError::WorkflowReconstruction(source))
    })
}

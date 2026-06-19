#![forbid(unsafe_code)]
//! Admission-check gates and submit methods for the runtime facade.
//!
//! This module implements the three-call admission preflight gates:
//!
//! 1. `preflight_artifact_gate` — artifact existence, gate count, proof
//!    flags, digest binding, capability coverage.
//! 2. `preflight_step_gate` — per-workflow step-count ceiling check.
//! 3. `preflight_budget_gate` — full policy + capacity admission.
//!
//! It also provides the submit methods that hold `admission_lock` for the
//! entire preflight+enqueue pair so the budget reservation is atomic with
//! the queue commit.

use vb_core::WorkflowDigest;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::CompiledWorkflow;

use crate::admission::AdmissionBudgetRequest;
use crate::shard::Shard;
use crate::shard::ShardCommand;
use crate::{Runtime, RuntimeError, RuntimeResult};

impl Runtime {
    /// Three-call admission preflight.
    ///
    /// Evaluates the gates in order and fails closed on the first rejection.
    /// The caller (`Runtime::submit_*`) MUST hold `shard.admission_lock` for
    /// the entire preflight+enqueue pair so the budget reservation is atomic
    /// with the queue commit.
    pub(crate) fn preflight_direct_admission(
        shard: &Shard,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        if !super::admission_policy::requires_admission(shard.policy) {
            return Ok(());
        }
        let digest = workflow.digest();
        Self::preflight_artifact_gate(shard, run, digest, &caps)?;
        let budget_request = build_admission_budget_request(workflow)
            .map_err(|error| super::admission_result::map_aggregate_budget_error(error, digest))?;
        Self::preflight_step_gate(workflow, &budget_request, shard.policy)?;
        Self::preflight_budget_gate(shard, run, digest, &budget_request, &caps)?;
        Ok(())
    }

    /// Gate 1: artifact-existence and capability gate.
    fn preflight_artifact_gate(
        shard: &Shard,
        run: RunId,
        digest: WorkflowDigest,
        caps: &CapabilitySet,
    ) -> RuntimeResult<()> {
        crate::admission::admit_artifact_run(
            shard.artifact_store.as_ref(),
            shard.policy,
            run,
            digest,
            caps.clone(),
        )
        .map(|_admission| ())
        .map_err(|error| super::admission_result::map_admission_error(error, digest))
    }

    /// Gate 2: per-workflow step-count ceiling.
    fn preflight_step_gate(
        workflow: &CompiledWorkflow,
        budget_request: &AdmissionBudgetRequest,
        policy: RuntimePolicy,
    ) -> RuntimeResult<()> {
        if !super::admission_policy::requires_admission(policy) {
            return Ok(());
        }
        let limit = crate::admission::per_workflow_step_ceiling();
        let declared = u32::from(workflow.resource_contract().max_steps);
        let actual = budget_request.requested.max_steps_executable;
        let observed = declared.max(actual);
        if observed > limit {
            return Err(RuntimeError::AdmissionBudgetExceeded {
                actual: observed,
                limit,
            });
        }
        Ok(())
    }

    /// Gate 3: full policy + capacity admission.
    fn preflight_budget_gate(
        shard: &Shard,
        run: RunId,
        digest: WorkflowDigest,
        budget_request: &AdmissionBudgetRequest,
        caps: &CapabilitySet,
    ) -> RuntimeResult<()> {
        let store_adapter = AlwaysPresentArtifactStoreAdapter;
        crate::admission::admit_run_with_budget_policy(
            &store_adapter,
            shard.policy,
            digest,
            run,
            caps.clone(),
            *budget_request,
        )
        .map(|_admission| ())
        .map_err(|error| super::admission_result::map_admission_error(error, digest))
    }
}

// ── Submit methods ──────────────────────────────────────────────────────

impl Runtime {
    /// Submits a run using a compiled workflow.
    ///
    /// Admission is atomic with the enqueue: the per-shard `admission_lock`
    /// is held for the duration of the preflight and the enqueue so two
    /// concurrent submits cannot squeeze in between the budget reservation
    /// and the queue commit. Fails closed if the workflow's declared or
    /// computed IR step budget exceeds `vb_core::limits::MAX_STEPS_PER_WORKFLOW`.
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
    }

    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a run with pre-mapped runtime input slots.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, CapabilitySet::empty())?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps: CapabilitySet::empty(),
        })
    }

    /// Submits a run with inputs, capability grants, and validated action
    /// contracts.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[vb_core::action::ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
        })
    }

    /// Submits a run whose compiled artifact is already stored in the
    /// runtime's `FjallJournal`, identified by `artifact_digest`.
    ///
    /// This is the master §66 facade wrapper for an already-accepted artifact.
    /// The wrapper decodes the stored accepted-artifact envelope, validates
    /// the artifact, converts granted capabilities into a runtime
    /// [`CapabilitySet`], and submits the decoded workflow through the owning
    /// shard's normal admission path.
    ///
    /// Failure semantics (master §66 line 3421):
    /// - Rejects with `Err(RuntimeError::AdmissionArtifactNotFound { digest })`
    ///   when the `artifact_digest` is not present in the `compiled_ir` keyspace.
    /// - Rejects with `Err(RuntimeError::AdmissionArtifactInvalid { digest })`
    ///   when the stored envelope is malformed (postcard decode failure,
    ///   trailing bytes, or workflow parts deserialization failure).
    /// - Rejects with `Err(RuntimeError::AdmissionArtifactDigestMismatch { .. })`
    ///   when the envelope's `digest` field does not match `artifact_digest`.
    /// - Rejects with `Err(RuntimeError::UnsupportedOperation { operation })`
    ///   when the runtime is not backed by a storage journal
    ///   (e.g., `NoopRuntimeJournal` or `VolatileRuntimeJournal`).
    /// - Rejects with `Err(RuntimeError::AdmissionCapabilityDenied { .. })`
    ///   when the accepted artifact requires a capability that was not granted.
    /// - Rejects with `Err(RuntimeError::UnsupportedOperation { operation })`
    ///   when non-empty `input` bytes are supplied; this v0.1 path has no
    ///   public input-bin decoder and fails closed rather than ignoring bytes.
    /// - Rejects with `Err(RuntimeError::StorageJournalAppend { source })`
    ///   when durable artifact lookup or shard journal probing fails.
    ///
    /// On success, the wrapper enqueues `ShardCommand::Submit` on the owning
    /// shard. The shard records `RuntimeJournalEvent::RunSubmitted` and
    /// `RuntimeJournalEvent::RunAdmission` when it dispatches the command.
    pub fn submit_artifact(
        &self,
        run: RunId,
        artifact_digest: WorkflowDigest,
        input: &[u8],
        capabilities: &[Capability],
    ) -> RuntimeResult<()> {
        reject_unsupported_artifact_input(input)?;
        let artifact = self.load_submit_artifact(artifact_digest)?;
        validate_artifact_envelope(&artifact, artifact_digest)?;
        validate_artifact_ir_digest(&artifact, artifact_digest)?;
        let workflow = decode_artifact_workflow(&artifact, artifact_digest)?;
        let caps = capability_set_from_slice(capabilities, artifact_digest)?;
        validate_artifact_capabilities(&artifact, &caps, artifact_digest)?;
        self.enqueue_decoded_artifact(run, workflow, caps)
    }

    fn load_submit_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
        let fjall_journal = self.storage_journal_for_submit_artifact()?;
        load_accepted_artifact(fjall_journal.as_ref(), artifact_digest)
    }

    fn storage_journal_for_submit_artifact(
        &self,
    ) -> RuntimeResult<std::sync::Arc<vb_storage::FjallJournal>> {
        self.journal
            .storage_journal()
            .ok_or(RuntimeError::UnsupportedOperation {
                operation: "submit_artifact_without_storage_journal",
            })
    }

    fn enqueue_decoded_artifact(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let _admission_guard = shard.lock_admission()?;
        Self::preflight_direct_admission(shard, run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps,
        })
    }
}

fn reject_unsupported_artifact_input(input: &[u8]) -> RuntimeResult<()> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(RuntimeError::UnsupportedOperation {
            operation: "submit_artifact_input_decode",
        })
    }
}

fn load_accepted_artifact(
    journal: &vb_storage::FjallJournal,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
    let record = load_compiled_ir_record(journal, artifact_digest)?;
    validate_compiled_ir_record(&record, artifact_digest)?;
    let artifact = decode_compiled_ir_artifact(&record, artifact_digest)?;
    validate_record_digest(&record, &artifact, artifact_digest)?;
    Ok(artifact)
}

fn load_compiled_ir_record(
    journal: &vb_storage::FjallJournal,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<vb_storage::CompiledIrRecord> {
    journal
        .compiled_ir(artifact_digest)?
        .ok_or(RuntimeError::AdmissionArtifactNotFound {
            digest: artifact_digest,
        })
}

fn validate_compiled_ir_record(
    record: &vb_storage::CompiledIrRecord,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    vb_storage::admission::validate_compiled_ir_record(record).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        }
    })
}

fn decode_compiled_ir_artifact(
    record: &vb_storage::CompiledIrRecord,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<vb_storage::admission::AcceptedArtifact> {
    vb_storage::admission::decode_accepted_artifact_envelope(&record.ir).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        }
    })
}

fn validate_record_digest(
    record: &vb_storage::CompiledIrRecord,
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    if record.digest == artifact_digest {
        Ok(())
    } else {
        Err(RuntimeError::AdmissionDigestMismatch {
            requested: artifact_digest,
            record: record.digest,
            envelope: artifact.digest,
        })
    }
}

fn validate_artifact_envelope(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    if artifact.digest != artifact_digest {
        return Err(RuntimeError::AdmissionArtifactDigestMismatch {
            requested: artifact_digest,
            found: artifact.digest,
        });
    }
    crate::admission::validate_accepted_artifact_envelope(artifact)
        .map_err(crate::admission::map_artifact_envelope_error)
        .map_err(|error| super::admission_result::map_admission_error(error, artifact_digest))
}

fn validate_artifact_ir_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    let computed = blake3::hash(&artifact.ir);
    let computed_digest = WorkflowDigest::from_bytes(*computed.as_bytes());
    if computed_digest == artifact.digest {
        Ok(())
    } else {
        Err(RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        })
    }
}

fn decode_artifact_workflow(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<CompiledWorkflow> {
    let (mut parts, remaining) = postcard::take_from_bytes::<vb_core::workflow::WorkflowParts>(
        &artifact.ir,
    )
    .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
        digest: artifact_digest,
    })?;
    if !remaining.is_empty() {
        return Err(RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        });
    }
    parts.digest = artifact.digest;
    CompiledWorkflow::try_from_parts(parts).map_err(|_| RuntimeError::AdmissionArtifactInvalid {
        digest: artifact_digest,
    })
}

fn capability_set_from_slice(
    capabilities: &[Capability],
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<CapabilitySet> {
    let mut grants = Vec::new();
    grants.try_reserve_exact(capabilities.len()).map_err(|_| {
        RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        }
    })?;
    for capability in capabilities {
        grants.push(capability.clone());
    }
    Ok(CapabilitySet::from_grants(grants.into_boxed_slice()))
}

fn validate_artifact_capabilities(
    artifact: &vb_storage::admission::AcceptedArtifact,
    caps: &CapabilitySet,
    artifact_digest: WorkflowDigest,
) -> RuntimeResult<()> {
    for required in artifact.required_capabilities.iter() {
        crate::admission::check_capability(required.action_id(), required, caps).map_err(
            |error| super::admission_result::map_admission_error(error, artifact_digest),
        )?;
    }
    Ok(())
}

// ── Helper types ────────────────────────────────────────────────────────

/// Adapter that implements [`crate::admission::ArtifactStore`] for the
/// trivial case where the artifact has already been validated by gate 1
/// (`admit_artifact_run`). The production preflight uses this adapter so
/// gate 3 (`admit_run_with_budget_policy`) only evaluates the budget
/// policy dimensions; the artifact-existence check inside
/// `admit_run_with_budget_policy` would otherwise re-walk a heavier
/// storage path that has already been satisfied.
struct AlwaysPresentArtifactStoreAdapter;

impl crate::admission::ArtifactStore for AlwaysPresentArtifactStoreAdapter {
    fn compiled_ir_exists(&self, _digest: vb_core::ids::WorkflowDigest) -> bool {
        true
    }
}

// ── Budget-request helper ───────────────────────────────────────────────

/// Build the [`AdmissionBudgetRequest`](crate::admission::AdmissionBudgetRequest)
/// used by the production preflight. Derives the requested budget from the
/// actual compiled workflow IR, not just from declared `ResourceContract`
/// values.
fn build_admission_budget_request(
    workflow: &CompiledWorkflow,
) -> Result<AdmissionBudgetRequest, vb_core::budget::AggregateBudgetError> {
    super::admission_policy::build_admission_budget_request(workflow)
}

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
use vb_core::capability::CapabilitySet;
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
    /// This is the master §66 facade wrapper around the storage-level
    /// `vb_storage::admission::submit_artifact` function. The wrapper is a
    /// thin facade that does NOT duplicate storage admission logic.
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
    /// - Rejects with `Err(RuntimeError::StorageJournalAppend { source })`
    ///   when the storage-level admission or journal event append fails.
    ///
    /// On success, the wrapper records exactly one `RuntimeJournalEvent::RunSubmitted`
    /// event (which maps to `JournalEvent::RunAccepted` in storage) and returns
    /// `Ok(())`. The `input` and `capabilities` parameters are reserved for a
    /// future integration that threads them into the per-shard run submission
    /// path; the storage-level admission function does not consume them.
    #[allow(clippy::too_many_lines)]
    pub fn submit_artifact(
        &self,
        run: RunId,
        artifact_digest: WorkflowDigest,
        _input: &[u8],
        _capabilities: &[vb_core::capability::Capability],
    ) -> RuntimeResult<()> {
        // (1) Look up the stored accepted artifact in the runtime's
        //     FjallJournal. Reject if the digest is missing, the
        //     envelope is malformed, or the envelope's digest does not
        //     match the requested digest.
        let fjall_journal =
            self.journal
                .storage_journal()
                .ok_or(RuntimeError::UnsupportedOperation {
                    operation: "submit_artifact_without_storage_journal",
                })?;
        let record = fjall_journal.compiled_ir(artifact_digest)?.ok_or(
            RuntimeError::AdmissionArtifactNotFound {
                digest: artifact_digest,
            },
        )?;

        // Decode the AcceptedArtifact envelope from the record's IR bytes.
        // Reject on malformed envelope or trailing bytes.
        let (artifact, remaining) =
            postcard::take_from_bytes::<vb_storage::admission::AcceptedArtifact>(&record.ir)
                .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
                    digest: artifact_digest,
                })?;
        let envelope_end = record.ir.len().checked_sub(remaining.len()).ok_or(
            RuntimeError::AdmissionArtifactInvalid {
                digest: artifact_digest,
            },
        )?;
        if envelope_end != record.ir.len() {
            return Err(RuntimeError::AdmissionArtifactInvalid {
                digest: artifact_digest,
            });
        }

        // Verify the envelope's digest matches the requested digest.
        if artifact.digest != artifact_digest {
            return Err(RuntimeError::AdmissionArtifactDigestMismatch {
                requested: artifact_digest,
                found: artifact.digest,
            });
        }

        // Decode the WorkflowParts from the artifact's IR bytes and
        // build a CompiledWorkflow for the storage-level admission call.
        let (mut parts, parts_remaining) = postcard::take_from_bytes::<
            vb_core::workflow::WorkflowParts,
        >(&artifact.ir)
        .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
            digest: artifact_digest,
        })?;
        let parts_end = artifact.ir.len().checked_sub(parts_remaining.len()).ok_or(
            RuntimeError::AdmissionArtifactInvalid {
                digest: artifact_digest,
            },
        )?;
        if parts_end != artifact.ir.len() {
            return Err(RuntimeError::AdmissionArtifactInvalid {
                digest: artifact_digest,
            });
        }
        parts.digest = artifact.digest;
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|_| {
            RuntimeError::AdmissionArtifactInvalid {
                digest: artifact_digest,
            }
        })?;

        // (2) Call vb_storage::admission::submit_artifact to perform the
        //     storage-side admission. This validates the workflow, computes
        //     the checksum, and persists the artifact (idempotent for an
        //     already-stored artifact with matching metadata hash).
        vb_storage::admission::submit_artifact(
            fjall_journal.as_ref(),
            &workflow,
            RuntimePolicy::Journaled,
        )?;

        // (3) Record a RunAccepted journal event before returning Ok(()).
        self.journal.append_sequenced(
            crate::journal::RuntimeJournalEvent::RunSubmitted {
                run,
                workflow: artifact_digest,
            },
            vb_storage::EventSeq::new(0),
        )?;

        Ok(())
    }
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

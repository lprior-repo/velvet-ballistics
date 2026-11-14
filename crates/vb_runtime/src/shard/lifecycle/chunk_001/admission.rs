impl Shard {
    fn append_admission_header_journal_event(
        &mut self,
        run: RunId,
        event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        match self.append_journal_event(event) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.discard_journal_sequence(run);
                Err(RuntimeError::admission_header_persistence_failed(error))
            }
        }
    }

    pub(crate) fn build_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        use crate::admission::admit_artifact_run;

        match admit_artifact_run(self.artifact_store.as_ref(), self.policy, run, digest, caps) {
            Ok(admission) => Ok(Some(admission)),
            Err(error) => Err(map_admission_error(error, digest)),
        }
    }

    pub(crate) fn build_admission_with_budget(
        &self,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
        lifetime_cap: Option<std::num::NonZeroU64>,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        let admission = self.build_admission(run, workflow.digest(), caps)?;
        self.validate_secret_presence(workflow)?;
        match admission {
            Some(adm) => {
                let mut budget = vb_core::budget::AggregateResourceBudget::from_workflow(workflow)
                    .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
                        digest: workflow.digest(),
                    })?;
                if let Some(cap) = lifetime_cap {
                    let profile_cap =
                        vb_core::BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds;
                    if cap.get() > profile_cap {
                        return Err(RuntimeError::AdmissionArtifactInvalid {
                            digest: workflow.digest(),
                        });
                    }
                    budget.max_run_time_seconds = cap.get();
                }
                Ok(Some(adm.with_attached_budget(budget)))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn validate_submit_admission(
        &self,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.build_admission(run, workflow.digest(), caps)?;
        execution_binding::validate(self.artifact_store.as_ref(), self.policy, workflow, &[])?;
        self.validate_secret_presence(workflow)?;
        Ok(())
    }

    /// Validates admission requirements for a submitted run with action contracts.
    ///
    /// Checks: contract ID matches its registry index, duplicate action IDs,
    /// well-formed contract names, that granted capabilities cover the required
    /// capabilities declared by the contracts, and that each workflow Do action
    /// has a corresponding contract at its registry index.
    pub(crate) fn validate_submit_admission_with_contracts(
        &self,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        let digest = workflow.digest();
        self.build_admission(run, digest, caps.clone())?;
        super::contract_validation::validate_contract_ids(action_contracts)?;
        super::contract_validation::validate_contract_capabilities(action_contracts, &caps)?;
        super::contract_validation::validate_workflow_action_binding(workflow, action_contracts)?;
        execution_binding::validate(
            self.artifact_store.as_ref(),
            self.policy,
            workflow,
            action_contracts,
        )?;
        self.validate_secret_presence(workflow)?;
        Ok(())
    }
}

fn map_admission_error(
    error: crate::admission::AdmissionError,
    digest: vb_core::ids::WorkflowDigest,
) -> RuntimeError {
    use crate::admission::AdmissionError;
    match error {
        AdmissionError::ArtifactNotFound { digest } => {
            RuntimeError::AdmissionArtifactNotFound { digest }
        }
        AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        } => RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        AdmissionError::ResourceCapacityExceeded { available, .. } => {
            RuntimeError::ActiveRunCapacityExceeded {
                capacity: usize::try_from(available).map_or(usize::MAX, |value| value),
            }
        }
        other => map_remaining_admission_error(other, digest),
    }
}

fn map_remaining_admission_error(
    error: crate::admission::AdmissionError,
    digest: vb_core::ids::WorkflowDigest,
) -> RuntimeError {
    use crate::admission::AdmissionError;
    match error {
        AdmissionError::ArtifactDigestMismatch { requested, found } => {
            RuntimeError::AdmissionArtifactDigestMismatch { requested, found }
        }
        AdmissionError::ArtifactCertificateStale { digest, .. } => {
            RuntimeError::AdmissionArtifactStale { digest }
        }
        AdmissionError::CapabilityCountMismatch {
            required_count,
            granted_count,
        } => RuntimeError::AdmissionCapabilityCountMismatch {
            required_count,
            granted_count,
        },
        other => map_invalid_admission_error(other, digest),
    }
}

fn map_invalid_admission_error(
    error: crate::admission::AdmissionError,
    digest: vb_core::ids::WorkflowDigest,
) -> RuntimeError {
    use crate::admission::AdmissionError;
    match error {
        AdmissionError::BudgetPolicyExceeded { .. }
        | AdmissionError::ResourceBudgetOverflow { .. }
        | AdmissionError::ResourceBudgetUnderflow { .. }
        | AdmissionError::ResourceBudgetInvalidCapacity { .. }
        | AdmissionError::ResourceStepCeilingExceeded { .. }
        | AdmissionError::ResourcePerTickCeilingExceeded { .. }
        | AdmissionError::ArtifactEnvelopeDecodeFailed
        | AdmissionError::ArtifactInvalidGateCount { .. }
        | AdmissionError::ArtifactInvalidProofFlag { .. }
        | AdmissionError::ArtifactVersionIncompatible { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        _ => RuntimeError::AdmissionArtifactInvalid { digest },
    }
}

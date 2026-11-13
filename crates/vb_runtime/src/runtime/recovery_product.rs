#[path = "recovery_product/attempts.rs"]
mod attempts;

impl Runtime {
    /// Builds a typed durable recovery product without mutating shard state.
    pub fn recover_product(
        &self,
        run: RunId,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let fjall_journal = self.recovery_storage_journal()?;
        let prefix = Self::load_recovery_prefix_evidence(fjall_journal.as_ref(), run)?;
        let recovered = self.load_recovery_artifact_and_workflow(
            std::sync::Arc::clone(&fjall_journal),
            prefix.admission.artifact_digest(),
        )?;
        Self::validate_recovery_admission(run, self.policy, &prefix, &recovered)?;
        self.shard_for(run)?
            .validate_secret_presence(&recovered.workflow)?;
        if crate::recovery::run_is_currently_terminal(&prefix.events)
            && let Ok(hydration) = vb_storage::recovery::summarize_recovery_events(&prefix.events)
        {
            return Ok(
                crate::recovery::RuntimeRecoveryProduct::cannot_resume_terminal(
                    hydration.summary(),
                ),
            );
        }
        let retained_inputs = crate::recovery::load_retained_inputs_for_recovery(
            &prefix.events,
            fjall_journal.as_ref(),
            &recovered.artifact,
            &recovered.workflow,
            run,
        )?;
        self.build_runtime_recovery_product(run, prefix, recovered, retained_inputs)
    }

    fn recovery_storage_journal(&self) -> RuntimeResult<std::sync::Arc<vb_storage::FjallJournal>> {
        self.journal
            .storage_journal()
            .ok_or(RuntimeError::RecoveryNotAvailable)
    }

    fn load_recovery_prefix_evidence(
        journal: &vb_storage::FjallJournal,
        run: RunId,
    ) -> RuntimeResult<RecoveryPrefixEvidence> {
        let events = Self::load_recovery_events(journal, run)?;
        let (workflow_digest, next_seq) = Self::recovery_prefix_identity(&events)?;
        let admission = Self::recovery_prefix_admission(&events)?;
        Ok(RecoveryPrefixEvidence {
            events,
            admission,
            workflow_digest,
            next_seq,
        })
    }

    fn recovery_prefix_identity(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<(WorkflowDigest, vb_storage::EventSeq)> {
        if events.is_empty() {
            return Self::empty_recovery_prefix_identity(events);
        }
        Self::durable_recovery_prefix_identity(events)
    }

    fn empty_recovery_prefix_identity(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<(WorkflowDigest, vb_storage::EventSeq)> {
        let seed = Self::recover_frame_seed_from_events(events)?;
        Ok((
            Self::recovery_workflow_digest(&seed)?,
            Self::next_recovery_sequence(&seed.summary())?,
        ))
    }

    fn durable_recovery_prefix_identity(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<(WorkflowDigest, vb_storage::EventSeq)> {
        let workflow_digest = events
            .iter()
            .find_map(|event| match event {
                vb_storage::JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
                _ => None,
            })
            .ok_or(RuntimeError::RecoveryCannotResume {
                reason: String::from("workflow_digest_missing_from_recovery_summary"),
            })?;
        let next_seq = events
            .last()
            .ok_or(RuntimeError::RecoveryCannotResume {
                reason: String::from("recovery_events_missing"),
            })?
            .seq()
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        Ok((workflow_digest, next_seq))
    }

    fn recovery_prefix_admission(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<crate::admission::RunAdmission> {
        crate::recovery::hydrate_run_admission_from_events(events).ok_or(
            RuntimeError::RecoveryCannotResume {
                reason: String::from("admission_missing"),
            },
        )
    }

    fn load_recovery_events(
        journal: &vb_storage::FjallJournal,
        run: RunId,
    ) -> RuntimeResult<Vec<vb_storage::JournalEvent>> {
        journal
            .events_for_run_full(run)
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })
    }

    /// Reconstructs a typed [`vb_storage::recovery::RecoveryFrameSeedProduct`]
    /// from the durable event slice.
    ///
    /// **FINDING-002 typestate closure.** This helper no longer accepts
    /// the raw [`vb_storage::recovery::RecoveryFrameSeed`] DTO. It threads
    /// the storage-classified typestate product into the runtime
    /// `RecoveredSeedProductContext`, so the storage
    /// `RecoveryCannotResumeState` witness survives into
    /// `RuntimeRecoveryProduct::CannotResume { reason }` without being
    /// erased by a raw-DTO roundtrip. The internal call uses
    /// `recover_runtime_frame_seed_from_events` (the non-`_raw_` entry
    /// point) so the storage layer performs the cannot-resume/resumable
    /// classification once and the runtime reuses that classification.
    fn recover_frame_seed_from_events(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<vb_storage::recovery::RecoveryFrameSeedProduct> {
        vb_storage::recovery::recover_runtime_frame_seed_from_events(events).map_err(|error| {
            RuntimeError::Recovery {
                error: error.to_string(),
            }
        })
    }

    fn recovery_workflow_digest(
        seed: &vb_storage::recovery::RecoveryFrameSeedProduct,
    ) -> RuntimeResult<WorkflowDigest> {
        seed.summary()
            .workflow
            .ok_or(RuntimeError::RecoveryCannotResume {
                reason: String::from("workflow_digest_missing_from_recovery_summary"),
            })
    }

    fn next_recovery_sequence(
        summary: &vb_storage::recovery::RecoveryRuntimeSummary,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        summary
            .last_seq
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }

    fn validate_recovery_admission(
        run: RunId,
        runtime_policy: vb_core::RuntimePolicy,
        prefix: &RecoveryPrefixEvidence,
        recovered: &RecoveryArtifactWorkflow,
    ) -> RuntimeResult<()> {
        crate::recovery::validate_recovered_admission_evidence(
            crate::recovery::RecoveredAdmissionEvidence {
                context: crate::recovery::RecoveredAdmissionContext {
                    run,
                    expected_workflow: prefix.workflow_digest,
                    runtime_policy,
                },
                admission: &prefix.admission,
                artifact: &recovered.artifact,
                workflow: &recovered.workflow,
            },
        )
    }

    fn build_runtime_recovery_product(
        &self,
        run: RunId,
        prefix: RecoveryPrefixEvidence,
        recovered: RecoveryArtifactWorkflow,
        retained_inputs: crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        if let Some(product) = self.retry_timer_recovery_product(run, &prefix, &recovered)? {
            return Ok(product);
        }
        attempts::validate_owned_run_attempts(
            &prefix.events,
            &recovered.workflow,
            &retained_inputs,
        )?;
        let expected = crate::recovery::action_abi_digests_from_contracts(
            &recovered.artifact.action_contracts,
        )?;
        let evidence =
            Self::recover_seed_evidence(&prefix.events, &recovered.workflow, &retained_inputs)?;
        let boundary = Self::recovered_boundary_from_events(
            &evidence.seed,
            &prefix.events,
            &recovered.workflow,
            &expected,
        )?;
        attempts::validate_pending_action_capacity(
            boundary,
            &recovered.workflow,
            evidence.complete_frame.as_ref(),
        )?;
        // Enforce receipt-only reconciliation for recovered registered actions:
        // a pending registered action must be reconciliable from the durable
        // receipt before the run is allowed to resume (no live re-execution).
        self.validate_registered_recovery(&recovered.artifact, boundary)?;
        Self::product_from_recovered_seed(run, prefix, evidence, boundary, retained_inputs)
    }
}

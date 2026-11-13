#[path = "recovery_seed/owned.rs"]
mod owned;

impl Runtime {
    /// Reconstructs the typed recovery seed plus any replayed frame/store
    /// evidence for a loaded workflow.
    ///
    /// Deterministic owned controls and source-only traces with retained
    /// handles replay from the input checkpoint used by the live run. Other
    /// workflows keep the generic seed and scalar replay path.
    fn recover_seed_evidence(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<RecoveredSeedEvidence> {
        if let Some(evidence) =
            Self::recover_owned_seed_evidence(events, workflow, retained_inputs)?
        {
            return Ok(evidence);
        }
        let (seed, repaired_failure_route) =
            Self::recover_seed_for_loaded_workflow(events, workflow)?;
        let complete_frame = Self::complete_scalar_frame(events, workflow, repaired_failure_route)?;
        Ok(RecoveredSeedEvidence {
            seed,
            complete_frame,
            recovered_store: None,
        })
    }

    fn complete_scalar_frame(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        repaired_failure_route: bool,
    ) -> RuntimeResult<Option<vb_core::RunFrame>> {
        if repaired_failure_route {
            return Ok(None);
        }
        vb_storage::recovery::complete_scalar::replay_complete_scalar(events, workflow)
            .map(|replayed| replayed.map(|complete| complete.into_frame()))
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })
    }

    /// Reconstructs the workflow-aware seed and repairs only a partial handler route.
    fn recover_seed_for_loaded_workflow(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<(vb_storage::recovery::RecoveryFrameSeedProduct, bool)> {
        let product = vb_storage::recovery::recover_runtime_frame_seed_from_events_with_workflow(
            events, workflow,
        )
        .map_err(|error| RuntimeError::Recovery {
            error: error.to_string(),
        })?;
        let (seed, repaired) = Self::repair_routed_action_failure(product, events, workflow)?;
        Ok((seed, repaired))
    }

    fn repair_routed_action_failure(
        product: vb_storage::recovery::RecoveryFrameSeedProduct,
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<(vb_storage::recovery::RecoveryFrameSeedProduct, bool)> {
        let mut seed = product.seed().clone();
        let repaired = crate::recovery::restore_routed_action_failure(&mut seed, events, workflow)
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })?;
        Ok((
            vb_storage::recovery::RecoveryFrameSeedProduct::from_seed(seed),
            repaired,
        ))
    }

    /// Builds the recovered live-run boundary from the typed seed product.
    ///
    /// Takes the typed [`vb_storage::recovery::RecoveryFrameSeedProduct`]
    /// and deref-coerces to `&RecoveryFrameSeed` for the legacy
    /// `pending_action_ticket_from_events` / `recovered_run_boundary_from_seed`
    /// helpers, which still consume the raw DTO shape. The typed witness
    /// survives in the context for `product_from_recovered_context`.
    fn recovered_boundary_from_events(
        seed: &vb_storage::recovery::RecoveryFrameSeedProduct,
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        expected: &[(vb_core::ActionId, WorkflowDigest)],
    ) -> RuntimeResult<crate::recovery::RecoveredRunBoundary> {
        let pending_action = crate::recovery::pending_action_ticket_from_events(
            seed.seed(),
            events,
            workflow,
            expected,
        )?;
        Ok(crate::recovery::recovered_run_boundary_from_seed(
            seed.seed(),
            pending_action,
            workflow,
            events,
        ))
    }

    fn product_from_recovered_seed(
        run: RunId,
        prefix: RecoveryPrefixEvidence,
        evidence: RecoveredSeedEvidence,
        boundary: crate::recovery::RecoveredRunBoundary,
        retained_inputs: crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let context = RecoveredSeedProductContext {
            run,
            prefix,
            seed: evidence.seed,
            boundary,
            complete_frame: evidence.complete_frame,
            recovered_store: evidence.recovered_store,
        };
        Self::product_from_recovered_context(context, &retained_inputs)
    }

    fn product_from_recovered_context(
        context: RecoveredSeedProductContext,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let replay_store = context
            .recovered_store
            .clone()
            .map(vb_core::value_store::ValueStore::from_checkpoint_v1)
            .transpose()
            .map_err(|error| RuntimeError::Recovery {
                error: format!("recovered value store invalid: {error}"),
            })?;
        let store = replay_store.as_ref().or_else(|| retained_inputs.store());
        let cannot_resume = crate::recovery::classify_full_recovery_resume(
            context.seed.seed(),
            context.boundary,
            retained_inputs.initial_inputs(),
            store,
        );
        if !cannot_resume.is_resumable() {
            return Ok(crate::recovery::RuntimeRecoveryProduct::cannot_resume(
                context.seed.summary(),
                cannot_resume,
            ));
        }
        // Terminal-lifecycle gate: a run whose latest terminal evidence (RunFailed /
        // RunFinished / RunCancelled / RunKilled) belongs to its current attempt must not be
        // silently re-driven by automatic cold recovery. The next-attempt RetryCheckpoint is
        // evidence only until an explicit `retry` drives `recover_retry` (a separate path).
        if crate::recovery::run_is_currently_terminal(&context.prefix.events) {
            return Ok(
                crate::recovery::RuntimeRecoveryProduct::cannot_resume_terminal(
                    context.seed.summary(),
                ),
            );
        }
        Self::resumable_product_from_recovered_context(context, retained_inputs, store)
    }

    fn resumable_product_from_recovered_context(
        context: RecoveredSeedProductContext,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
        store: Option<&vb_core::value_store::ValueStore>,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let initial_inputs = retained_inputs.initial_inputs().to_vec();
        let frame = match context.complete_frame {
            Some(frame) => frame,
            None => crate::recovery::hydrate_frame_for_full_recovery_with_inputs(
                context.seed.seed(),
                context.boundary,
                initial_inputs,
                store,
            )?,
        };
        let collect_states = Self::hydrate_recovered_collect_states(&context.prefix.events)?;
        let open_gates = crate::manual_gate::reconstruct_open_gates(&context.prefix.events)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        let bound_open_gates =
            crate::manual_gate::reconstruct_bound_open_gates(&context.prefix.events)
                .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        let frame_step_count = frame.step_count();
        Ok(crate::recovery::RuntimeRecoveryProduct::resumable(
            crate::recovery::ResumableRecoveryProduct::new(
                crate::recovery::ResumableRecoveryParts {
                    run: context.run,
                    summary: context.seed.summary(),
                    frame,
                    artifact_digest: context.prefix.admission.artifact_digest(),
                    workflow_digest: context.prefix.workflow_digest,
                    next_seq: context.prefix.next_seq,
                    collect_states,
                    boundary: context.boundary,
                    retry_checkpoint: None,
                    open_gates,
                    bound_open_gates,
                    admission: context.prefix.admission.clone(),
                    run_started_millis: context.prefix.run_started_millis,
                    recovered_action_attempts: Some(crate::recovery::recover_action_attempts(
                        &context.prefix.events,
                        frame_step_count,
                    )),
                    recovered_journal_bytes_written: Some(
                        crate::recovery::recover_journal_bytes_written(&context.prefix.events)?,
                    ),
                    recovered_tenant: crate::recovery::hydrate_run_tenant(&context.prefix.events),
                    restored_store: context
                        .recovered_store
                        .clone()
                        .or_else(|| retained_inputs.store_checkpoint()),
                },
            ),
        ))
    }

    fn hydrate_recovered_collect_states(
        events: &[vb_storage::JournalEvent],
    ) -> RuntimeResult<crate::primitives::collect::CollectStates> {
        crate::primitives::collect::hydrate_collect_states_from_recovered_journal(events).map_err(
            |_| RuntimeError::RecoveryCannotResume {
                reason: String::from("collect_states_missing"),
            },
        )
    }

    fn enqueue_recovered_product(
        &self,
        product: crate::recovery::ResumableRecoveryProduct,
    ) -> RuntimeResult<()> {
        let run = product.run_id();
        let mut command = product.into_recover_command();
        let shard = self.shard_for(run)?;
        if command.retry_checkpoint.is_none() {
            let recovered = self.load_recovery_artifact_and_workflow(
                self.recovery_storage_journal()?,
                command.artifact_digest,
            )?;
            if recovered.workflow.digest() != command.workflow_digest {
                return Err(RuntimeError::Recovery {
                    error: String::from("recovery workflow identity mismatch"),
                });
            }
            shard.initialize_secret_frame(&recovered.workflow, &mut command.frame)?;
        }
        shard.enqueue(ShardCommand::Recover {
            command: Box::new(command),
        })
    }

    fn load_recovery_artifact_and_workflow(
        &self,
        journal: std::sync::Arc<vb_storage::FjallJournal>,
        artifact_digest: WorkflowDigest,
    ) -> RuntimeResult<RecoveryArtifactWorkflow> {
        use crate::admission::AcceptedArtifactStore;

        let store = crate::admission::StorageArtifactStore::with_catalog(
            journal,
            self.journal.executor_catalog(),
        );
        let artifact = store
            .load_accepted_artifact(artifact_digest)
            .map_err(|error| RuntimeError::RecoveryCannotResume {
                reason: match error {
                    crate::admission::ArtifactEnvelopeError::ArtifactNotFound { .. } => {
                        String::from("artifact_missing")
                    }
                    _ => String::from("artifact_decode_failed"),
                },
            })?;
        let workflow =
            crate::admission::verify_stored_artifact(&store, &artifact, Some(self.policy))
                .map_err(|error| RuntimeError::Recovery {
                    error: error.to_string(),
                })?;
        Ok(RecoveryArtifactWorkflow { artifact, workflow })
    }
}

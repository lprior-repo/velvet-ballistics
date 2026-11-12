struct RecoveredWorkflowSeed {
    seed: vb_storage::recovery::RecoveryFrameSeedProduct,
    replay_cursor: Option<(StepIdx, u64)>,
}

impl From<vb_storage::recovery::RecoveryFrameSeedProduct> for RecoveredWorkflowSeed {
    fn from(seed: vb_storage::recovery::RecoveryFrameSeedProduct) -> Self {
        Self {
            seed,
            replay_cursor: None,
        }
    }
}

impl
    From<(
        vb_storage::recovery::RecoveryFrameSeedProduct,
        Option<(StepIdx, u64)>,
    )> for RecoveredWorkflowSeed
{
    fn from(
        (seed, replay_cursor): (
            vb_storage::recovery::RecoveryFrameSeedProduct,
            Option<(StepIdx, u64)>,
        ),
    ) -> Self {
        Self {
            seed,
            replay_cursor,
        }
    }
}

impl Runtime {
    /// Reconstructs a typed [`vb_storage::recovery::RecoveryFrameSeedProduct`]
    /// for a workflow-aware replay path.
    ///
    /// **FINDING-002 typestate closure.** Uses the typed entry point
    /// `recover_runtime_frame_seed_from_events_with_replay_inputs` so the
    /// storage-classified cannot-resume/resumable product is the source
    /// of truth; the runtime never sees the raw DTO.
    fn recover_seed_for_loaded_workflow(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<RecoveredWorkflowSeed> {
        let recovered = if inputs.initial_inputs().is_empty() && inputs.store().is_none() {
            vb_storage::recovery::recover_runtime_frame_seed_from_events_with_workflow(
                events, workflow,
            )
            .map(|seed| (seed, None))
        } else {
            vb_storage::recovery::recover_runtime_frame_seed_from_events_with_replay_inputs(
                events,
                workflow,
                inputs.initial_inputs(),
                inputs.store(),
            )
        };
        recovered
            .map(RecoveredWorkflowSeed::from)
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })
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
        seed: impl Into<RecoveredWorkflowSeed>,
        boundary: crate::recovery::RecoveredRunBoundary,
        retained_inputs: crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let RecoveredWorkflowSeed {
            seed,
            replay_cursor,
        } = seed.into();
        let context = RecoveredSeedProductContext {
            run,
            prefix,
            seed,
            replay_cursor,
            boundary,
        };
        Self::product_from_recovered_context(context, &retained_inputs)
    }

    fn product_from_recovered_context(
        context: RecoveredSeedProductContext,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let cannot_resume = crate::recovery::classify_full_recovery_resume(
            context.seed.seed(),
            context.boundary,
            retained_inputs.initial_inputs(),
            retained_inputs.store(),
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
        Self::resumable_product_from_recovered_context(context, retained_inputs)
    }

    fn resumable_product_from_recovered_context(
        context: RecoveredSeedProductContext,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<crate::recovery::RuntimeRecoveryProduct> {
        let frame = Self::hydrate_replayed_frame(&context, retained_inputs)?;
        let collect_states = Self::hydrate_recovered_collect_states(&context.prefix.events)?;
        let open_gates = crate::manual_gate::reconstruct_open_gates(&context.prefix.events)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
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
                    open_gates,
                    admission: context.prefix.admission.clone(),
                    restored_store: retained_inputs.store_checkpoint(),
                },
            ),
        ))
    }

    fn hydrate_replayed_frame(
        context: &RecoveredSeedProductContext,
        inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<vb_core::RunFrame> {
        let frame = crate::recovery::hydrate_frame_for_full_recovery_with_inputs(
            context.seed.seed(),
            context.boundary,
            inputs.initial_inputs().to_vec(),
            inputs.store(),
        )?;
        Self::apply_replay_cursor(frame, context.replay_cursor)
    }

    fn apply_replay_cursor(
        frame: vb_core::RunFrame,
        cursor: Option<(StepIdx, u64)>,
    ) -> RuntimeResult<vb_core::RunFrame> {
        let Some((pc, _)) = cursor else {
            return Ok(frame);
        };
        let mut checkpoint = frame.checkpoint_v1();
        checkpoint.pc = pc;
        vb_core::RunFrame::from_checkpoint_v1(checkpoint)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)
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
        let command = product.into_recover_command();
        let shard = self.shard_for(run)?;
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
            vb_storage::decode_compiled_ir(&artifact.ir, Some(artifact.artifact_version)).map_err(
                |error| RuntimeError::Recovery {
                    error: error.to_string(),
                },
            )?;
        Ok(RecoveryArtifactWorkflow { artifact, workflow })
    }
}

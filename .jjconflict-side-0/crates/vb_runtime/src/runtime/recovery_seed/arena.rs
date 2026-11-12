use super::*;

impl Runtime {
    pub(super) fn recover_seed_for_loaded_workflow(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        inputs: &mut crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<RecoveredWorkflowSeed> {
        let outcome =
            vb_storage::recovery::recover_runtime_frame_seed_from_events_with_replay_outcome(
                events,
                workflow,
                inputs.initial_inputs(),
                inputs.store(),
            )
            .map_err(|error| RuntimeError::Recovery {
                error: error.to_string(),
            })?;
        let (seed, replay_cursor, store) = outcome.into_parts();
        if store.is_some() {
            inputs.use_replayed_store(store);
        }
        Ok(RecoveredWorkflowSeed {
            seed,
            replay_cursor,
        })
    }
    // Recover seed and boundary from loaded workflow artifact.
    pub(super) fn recover_seed_and_boundary(
        &self,
        prefix: &RecoveryPrefixEvidence,
        recovered: &RecoveryArtifactWorkflow,
        inputs: &mut crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<(RecoveredWorkflowSeed, crate::recovery::RecoveredRunBoundary)> {
        let expected = crate::recovery::action_abi_digests_from_contracts(
            &recovered.artifact.action_contracts,
        )?;
        let seed =
            Self::recover_seed_for_loaded_workflow(&prefix.events, &recovered.workflow, inputs)?;
        let boundary = Self::recovered_boundary_from_events(
            &seed.seed,
            &prefix.events,
            &recovered.workflow,
            &expected,
        )?;
        self.validate_registered_recovery(&recovered.artifact, boundary)?;
        Ok((seed, boundary))
    }

    // Replay complete-scalar events into a final run frame if the run finished.
    pub(super) fn recover_complete_frame(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<Option<vb_core::RunFrame>> {
        let replayed =
            vb_storage::recovery::complete_scalar::replay_complete_scalar(events, workflow)
                .map_err(|error| RuntimeError::Recovery {
                    error: error.to_string(),
                })?;
        Ok(replayed.map(|r| r.into_frame()))
    }
}

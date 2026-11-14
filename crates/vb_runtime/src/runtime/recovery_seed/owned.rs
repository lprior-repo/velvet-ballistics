use super::*;

impl Runtime {
    pub(super) fn recover_owned_seed_evidence(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    ) -> RuntimeResult<Option<RecoveredSeedEvidence>> {
        let input_store = retained_inputs.store_checkpoint();
        if !is_owned_replay_candidate(workflow, retained_inputs, input_store.as_ref()) {
            return Ok(None);
        }
        Self::replay_owned_seed_evidence(events, workflow, retained_inputs, input_store.as_ref())
    }

    fn replay_owned_seed_evidence(
        events: &[vb_storage::JournalEvent],
        workflow: &CompiledWorkflow,
        retained_inputs: &crate::recovery::RecoveredRetainedInputs,
        input_store: Option<&vb_core::value_store::ValueStoreCheckpointV1>,
    ) -> RuntimeResult<Option<RecoveredSeedEvidence>> {
        let Some(evidence) = vb_storage::recovery::replay_owned_for_each_with_collect_snapshots(
            events,
            workflow,
            retained_inputs.initial_inputs(),
            input_store,
        )
        .map_err(|error| RuntimeError::Recovery {
            error: error.to_string(),
        })?
        else {
            return Ok(None);
        };
        Self::validate_replayed_collect_snapshots(events, workflow, &evidence.collect_snapshots)?;
        let replay = evidence.replay;
        Ok(Some(RecoveredSeedEvidence {
            seed: replay.product,
            complete_frame: Some(replay.frame),
            recovered_store: Some(replay.store),
        }))
    }
}

fn is_owned_replay_candidate(
    workflow: &CompiledWorkflow,
    retained_inputs: &crate::recovery::RecoveredRetainedInputs,
    input_store: Option<&vb_core::value_store::ValueStoreCheckpointV1>,
) -> bool {
    vb_storage::recovery::is_owned_for_each_contract(workflow)
        || vb_storage::recovery::is_owned_input_replay_contract(
            workflow,
            retained_inputs.initial_inputs(),
            input_store,
        )
}

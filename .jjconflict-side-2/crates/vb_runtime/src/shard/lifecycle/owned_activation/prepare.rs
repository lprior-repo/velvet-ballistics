use super::prefix::PreparedOwnedActivation;
use crate::RuntimeResult;
use crate::prepared_inputs::prepare_bound_runtime_inputs;
use crate::shard::types::RegisteredExecutionMode;
use crate::shard::{RunState, Shard};
use std::borrow::Cow;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CompiledWorkflow, RunFrame, RunId, SlotIdx, SlotValue, Taint, ValueStore};

type PreparedOwnedInputs<'a> = (ValueStore, Cow<'a, [(SlotIdx, SlotValue, Taint)]>);

#[cfg(test)]
#[path = "prepare_tests.rs"]
mod tests;

impl Shard {
    pub(super) fn prepare_owned_state(
        &mut self,
        run: RunId,
        accepted: PreparedOwnedActivation,
    ) -> RuntimeResult<RunState> {
        let (store, roots) = prepare_inputs(&accepted.workflow, &accepted.bundle)?;
        self.prepare_run_slots(run)?;
        let mut frame = self.take_frame_for(run, &accepted.workflow)?;
        for (slot, value, taint) in roots.iter() {
            if let Err(error) = frame.write_slot_with_taint(*slot, *value, *taint) {
                self.release_frame(frame);
                return Err(error.into());
            }
        }
        Ok(owned_run_state(accepted, frame, store))
    }
}

fn prepare_inputs<'a>(
    workflow: &CompiledWorkflow,
    bundle: &'a OwnedRootBundle,
) -> RuntimeResult<PreparedOwnedInputs<'a>> {
    match prepare_bound_runtime_inputs(workflow, bundle.checkpoint(), bundle.roots())? {
        Some(prepared) => Ok((prepared.store, Cow::Owned(prepared.roots))),
        None => Ok((
            ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?,
            Cow::Borrowed(bundle.roots()),
        )),
    }
}

fn owned_run_state(
    accepted: PreparedOwnedActivation,
    frame: RunFrame,
    store: ValueStore,
) -> RunState {
    let step_count = frame.step_count();
    RunState {
        registered_execution: RegisteredExecutionMode::Live,
        frame,
        run_attempt: core::num::NonZeroU16::MIN,
        workflow: accepted.workflow,
        store,
        action_attempts: crate::shard::helpers::new_action_attempts(step_count),
        admission: Some(accepted.admission),
        run_started_millis: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        journal_bytes_written: 0,
        action_contracts: accepted.contracts,
    }
}

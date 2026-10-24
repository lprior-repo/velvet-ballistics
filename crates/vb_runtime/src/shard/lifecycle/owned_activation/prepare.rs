use super::prefix::PreparedOwnedActivation;
use crate::RuntimeResult;
use crate::shard::types::RegisteredExecutionMode;
use crate::shard::{RunState, Shard};
use vb_core::{RunId, ValueStore};

impl Shard {
    pub(super) fn prepare_owned_state(
        &mut self,
        run: RunId,
        accepted: PreparedOwnedActivation,
    ) -> RuntimeResult<RunState> {
        let store = ValueStore::from_checkpoint_v1(accepted.bundle.checkpoint().clone())?;
        let mut frame = self.take_frame_for(run, &accepted.workflow)?;
        for (slot, value, taint) in accepted.bundle.roots() {
            if let Err(error) = frame.write_slot_with_taint(*slot, *value, *taint) {
                self.release_frame(frame);
                return Err(error.into());
            }
        }
        let step_count = frame.step_count();
        Ok(RunState {
            registered_execution: RegisteredExecutionMode::Live,
            frame,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow: accepted.workflow,
            store,
            action_attempts: crate::shard::helpers::new_action_attempts(step_count),
            admission: Some(accepted.admission),
            collect_states: crate::primitives::collect::CollectStates::new(),
            action_contracts: accepted.contracts,
        })
    }
}

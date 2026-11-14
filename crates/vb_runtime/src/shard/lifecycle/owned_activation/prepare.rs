use super::prefix::PreparedOwnedActivation;
use crate::RuntimeResult;
use crate::shard::types::RegisteredExecutionMode;
use crate::shard::{RunState, Shard};
use vb_core::{RunFrame, RunId, ValueStore};

impl Shard {
    pub(super) fn prepare_owned_state(
        &mut self,
        run: RunId,
        accepted: PreparedOwnedActivation,
    ) -> RuntimeResult<RunState> {
        let store = ValueStore::from_checkpoint_v1(accepted.bundle.checkpoint().clone())?;
        let frame = self.take_frame_for(run, &accepted.workflow)?;
        let frame = self.write_accepted_roots(frame, &accepted)?;
        let step_count = frame.step_count();
        let run_started_millis = Self::resolve_accepted_run_started_millis(&accepted)?;
        Ok(RunState {
            registered_execution: RegisteredExecutionMode::Live,
            frame,
            run_attempt: core::num::NonZeroU16::MIN,
            workflow: accepted.workflow,
            store,
            action_attempts: crate::shard::helpers::new_action_attempts(step_count),
            admission: Some(accepted.admission),
            run_started_millis,
            collect_states: crate::primitives::collect::CollectStates::new(),
            journal_bytes_written: accepted.journal_bytes_written,
            action_contracts: accepted.contracts,
        })
    }

    fn resolve_accepted_run_started_millis(
        accepted: &PreparedOwnedActivation,
    ) -> RuntimeResult<Option<u64>> {
        crate::recovery::resolve_run_started_millis(
            Some(&accepted.admission),
            accepted.run_started_millis,
        )
    }
    fn write_accepted_roots(
        &mut self,
        mut frame: RunFrame,
        accepted: &PreparedOwnedActivation,
    ) -> RuntimeResult<RunFrame> {
        for (slot, value, taint) in accepted.bundle.roots() {
            if let Err(error) = frame.write_slot_with_taint(*slot, *value, *taint) {
                self.release_frame(frame);
                return Err(error.into());
            }
        }
        Ok(frame)
    }
}

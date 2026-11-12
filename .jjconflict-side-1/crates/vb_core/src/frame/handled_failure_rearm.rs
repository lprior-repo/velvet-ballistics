#![forbid(unsafe_code)]

use super::{CoreError, CoreResult, RunFrame, StepIdx, StepState};
use crate::workflow::CompiledWorkflow;

fn unauthorized() -> CoreError {
    CoreError::InvalidCompiledWorkflow {
        reason: "handled failure lacks successful owned recovery",
    }
}

impl RunFrame {
    /// Checks recovery authority without changing failure evidence or frame state.
    pub fn validate_handled_failure_rearm(
        &self,
        plan: &CompiledWorkflow,
        control: StepIdx,
        failed: StepIdx,
    ) -> CoreResult<()> {
        let Some(region) = plan.loop_region(control) else {
            return Err(unauthorized());
        };
        if self.pc() != region.body()
            || self.step_state(region.start())? != StepState::Succeeded
            || self.step_state(control)? != StepState::Succeeded
            || failed < region.body()
            || failed >= control
            || self.step_state(failed)? != StepState::Failed
        {
            return Err(unauthorized());
        }

        let mut current = failed;
        // A handler can itself fail and recover. Only actual, forward declared
        // error edges inside this owned activation can discharge the failure.
        for _ in 0..plan.node_count() {
            let Some(handler) = plan
                .node(current)
                .and_then(|node| node.routing.error.handler())
            else {
                return Err(unauthorized());
            };
            if handler <= current || handler > control {
                return Err(unauthorized());
            }
            match self.step_state(handler)? {
                StepState::Succeeded => return Ok(()),
                StepState::Failed => current = handler,
                _ => return Err(unauthorized()),
            }
        }
        Err(unauthorized())
    }

    /// Rearms a recovered failure only at its validated owner's successful backedge.
    /// General `mark_pending` deliberately continues to reject Failed states.
    pub fn rearm_handled_failure(
        &mut self,
        plan: &CompiledWorkflow,
        control: StepIdx,
        failed: StepIdx,
    ) -> CoreResult<()> {
        self.validate_handled_failure_rearm(plan, control, failed)?;
        let state = self
            .states
            .get_mut(failed.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step: failed })?;
        *state = StepState::Pending;
        Ok(())
    }
}

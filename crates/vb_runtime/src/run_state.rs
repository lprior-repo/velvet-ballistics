//! Run state types.

use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

/// Mutable run state owned directly by the shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// Active run frame.
    pub frame: RunFrame,
    /// Compiled workflow for this run.
    pub workflow: CompiledWorkflow,
    /// Cold value store for list, object, and blob handles.
    pub store: ValueStore,
    /// Per-Do-step attempt counters owned with the live frame.
    action_attempts: Box<[u16]>,
    /// Admission record for this run, if admission gating was performed.
    pub admission: Option<crate::admission::RunAdmission>,
}

impl RunState {
    /// Creates a new run state with the given frame, workflow, and admission.
    pub fn new(
        frame: RunFrame,
        workflow: CompiledWorkflow,
        admission: Option<crate::admission::RunAdmission>,
    ) -> Self {
        let frame_step_count = frame.step_count();
        Self {
            frame,
            workflow,
            store: ValueStore::new(),
            action_attempts: new_action_attempts(frame_step_count),
            admission,
        }
    }

    /// Returns the number of action attempts for a step.
    pub fn action_attempt(&self, step: StepIdx) -> Option<u16> {
        self.action_attempts.get(step.as_usize()).copied()
    }

    /// Sets the action attempt count for a step.
    pub fn set_action_attempt(&mut self, step: StepIdx, attempt: u16) {
        if let Some(a) = self.action_attempts.get_mut(step.as_usize()) {
            *a = attempt;
        }
    }

    /// Returns a mutable reference to the action attempts slice.
    pub fn action_attempts_mut(&mut self) -> &mut [u16] {
        &mut self.action_attempts
    }
}

/// Creates a new action attempts counter for the given step count.
fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_state_new_allocates_zeroed_attempts() {
        let step_count = 4u16;
        let attempts = new_action_attempts(step_count);
        assert_eq!(attempts.len(), 4);
        assert!(attempts.iter().all(|&a| a == 0));
    }
}

//! Pristine owned activation uses actual retained authority, never queued arenas.

#[path = "owned_activation/prefix.rs"]
mod prefix;
#[path = "owned_activation/prepare.rs"]
mod prepare;

use crate::shard::{RuntimeState, Shard};
use crate::{RuntimeError, RuntimeResult};
use vb_core::{RunId, RuntimePolicy, WorkflowDigest};

impl Shard {
    pub(crate) fn preflight_owned_activation(
        &self,
        run: RunId,
        source: WorkflowDigest,
    ) -> RuntimeResult<()> {
        self.validate_owned_activation_capacity(run)?;
        prefix::load(self, run, source).map(drop)
    }

    pub(crate) fn handle_owned_activation(
        &mut self,
        run: RunId,
        source: WorkflowDigest,
    ) -> RuntimeResult<()> {
        self.validate_owned_activation_capacity(run)?;
        let accepted = prefix::load(self, run, source)?;
        self.prepare_run_slots(run)?;
        let next_seq = accepted.next_seq;
        let state = self.prepare_owned_state(run, accepted)?;
        self.publish_owned_state(run, state, next_seq)?;
        self.trace_ring
            .push(crate::trace::TraceEvent::RunSubmitted { run });
        self.counters.inc_submitted();
        match self.drive_run(run) {
            Ok(()) => Ok(()),
            Err(error) => {
                if !self.run_state_contains(run) {
                    self.discard_journal_sequence(run);
                }
                Err(error)
            }
        }
    }

    fn validate_owned_activation_capacity(&self, run: RunId) -> RuntimeResult<()> {
        if self.policy != RuntimePolicy::Strict {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "owned_activation_strict_only",
            });
        }
        if self.journal.durability_profile() != Some(vb_storage::DurabilityProfile::Strict) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "owned_activation_requires_strict_journal",
            });
        }
        if self.shutting_down {
            return Err(RuntimeError::ShutdownInProgress);
        }
        if self.run_state_contains(run)
            || self.checked_out_run_contains(run)
            || self.runtime_state_get(run).is_some()
            || self.terminal_runs_contains(run)
        {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if self.active_run_count() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        self.journal.probe()
    }

    fn publish_owned_state(
        &mut self,
        run: RunId,
        state: crate::shard::RunState,
        next_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        // admit_run_state consumes state on failure. Retain a cold frame copy
        // solely to return the taken pool capacity on that fallible path.
        let rollback_frame = state.frame.clone();
        if let Err(error) = self.seed_journal_sequence(run, next_seq) {
            self.release_frame(state.frame);
            return Err(error);
        }
        if let Err(error) = self.admit_run_state(run, state, RuntimeState::Initial) {
            self.release_frame(rollback_frame);
            self.discard_journal_sequence(run);
            return Err(error);
        }
        Ok(())
    }
}

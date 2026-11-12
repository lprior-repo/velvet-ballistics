//! Identity-only public transport for a pristine strict owned submission.

use super::Runtime;
use crate::shard::ShardCommand;
use crate::{RuntimeError, RuntimeResult};
use vb_core::{RunId, RuntimePolicy, WorkflowDigest};

impl Runtime {
    /// Queues activation of the same journal's pristine atomic owned submission.
    ///
    /// This validates before enqueue and the shard validates again when processing.
    /// Queue-full leaves the retained submission intact and retryable. Success
    /// means queued, not executed; no global multi-process exactly-once claim.
    pub fn activate_owned_submission(
        &self,
        run: RunId,
        expected_source_digest: WorkflowDigest,
    ) -> RuntimeResult<()> {
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
        self.journal.probe()?;
        match self.shard_for_run(run) {
            Ok(_) => return Err(RuntimeError::RunAlreadyExists),
            Err(RuntimeError::RunNotFound) => {}
            Err(error) => return Err(error),
        }
        let shard = self.shard_for(run)?;
        shard.preflight_owned_activation(run, expected_source_digest)?;
        shard.enqueue(ShardCommand::ActivateOwnedSubmission {
            run,
            expected_source_digest,
        })
    }
}

impl Runtime {
    /// Borrows actual actor-owned state for cold diagnostic observation only.
    ///
    /// This grants no activation/restoration authority. A mutable tick cannot
    /// occur until this immutable borrow ends. Terminal/no-active runs are absent.
    pub fn inspect_run_state(&self, run: RunId) -> RuntimeResult<&crate::shard::types::RunState> {
        self.shard_for_run(run)?
            .run_state_get(run)
            .ok_or(RuntimeError::RunNotFound)
    }
}

impl Shard {
    /// Enqueues a command. Returns `QueueFull` on overflow.
    /// For submit variants, validates journal health before enqueueing
    /// because handle_submit writes to journal before returning.
    /// When an admitted per-run queue depth is resolvable for the
    /// command's run, enqueueing past it returns
    /// `PerRunQueueDepthExceeded` and the command is not queued.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        if self.shutting_down {
            return Err(RuntimeError::ShutdownInProgress);
        }
        self.validate_secret_submission(&cmd)?;
        match &cmd {
            ShardCommand::Submit { workflow, .. }
            | ShardCommand::SubmitPrePersisted { workflow, .. }
            | ShardCommand::SubmitWithContracts { workflow, .. } => {
                let inputs = self.prepare_secret_inputs(workflow, &[])?;
                crate::shard::helpers::prepare_input_slots(&inputs, workflow)?;
            }
            ShardCommand::SubmitWithInputs {
                workflow, inputs, ..
            }
            | ShardCommand::SubmitWithInputsAndContracts {
                workflow, inputs, ..
            } => {
                let inputs = self.prepare_secret_inputs(workflow, inputs)?;
                crate::shard::helpers::prepare_input_slots(&inputs, workflow)?;
            }
            _ => {}
        }
        match &cmd {
            ShardCommand::Submit { .. }
            | ShardCommand::ActivateOwnedSubmission { .. }
            | ShardCommand::SubmitPrePersisted { .. }
            | ShardCommand::SubmitWithInputs { .. }
            | ShardCommand::SubmitWithContracts { .. }
            | ShardCommand::SubmitWithInputsAndContracts { .. } => {
                // Probe journal health before accepting the command.
                self.journal.probe()?;
            }
            _ => {}
        }
        let limit = self.admitted_queue_depth(&cmd)?;
        self.command_queue.enqueue_with_limit(cmd, limit)
    }

    /// Resolves the admitted per-run queue depth for a command, when one
    /// is available at enqueue time. An active run's attached budget
    /// governs; a submit variant's workflow budget is the budget that
    /// admission will attach; a recovery command's historical admission
    /// governs when it carries one. Commands for runs with no attached
    /// budget, and run-less commands, have no per-run cap here and are
    /// bounded only by shard-wide queue capacity.
    fn admitted_queue_depth(&self, cmd: &ShardCommand) -> RuntimeResult<Option<u32>> {
        match cmd {
            ShardCommand::Submit { workflow, .. }
            | ShardCommand::SubmitPrePersisted { workflow, .. }
            | ShardCommand::SubmitWithInputs { workflow, .. }
            | ShardCommand::SubmitWithContracts { workflow, .. }
            | ShardCommand::SubmitWithInputsAndContracts { workflow, .. } => {
                let budget = vb_core::budget::AggregateResourceBudget::from_workflow(workflow)
                    .map_err(|_| RuntimeError::AdmissionArtifactInvalid {
                        digest: workflow.digest(),
                    })?;
                Ok(Some(budget.max_queue_depth))
            }
            ShardCommand::Recover { command } => Ok(command.admitted_queue_depth()),
            _ => Ok(cmd.run_id().and_then(|run| {
                self.run_state_get(run)
                    .and_then(|state| state.admission.as_ref())
                    .and_then(|admission| admission.budget())
                    .map(|budget| budget.max_queue_depth)
            })),
        }
    }

    /// Returns the number of commands currently pending for a run.
    #[must_use]
    pub fn command_queue_pending_for(&self, run: RunId) -> u32 {
        self.command_queue.pending_for(run)
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn command_queue_len(&self) -> usize {
        self.command_queue.len()
    }

    /// Returns the remaining free slots in the command queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.command_queue.remaining_capacity()
    }

    /// Returns true if the command queue is full.
    #[must_use]
    pub fn is_queue_full(&self) -> bool {
        self.command_queue.is_full()
    }

    /// Returns the command queue capacity.
    #[must_use]
    pub fn command_queue_capacity(&self) -> usize {
        self.command_queue.capacity()
    }

    /// Returns the number of active runs on this shard.
    #[must_use]
    pub fn active_run_count(&self) -> usize {
        self.runs
            .len()
            .saturating_add(self.run_aggregate.checked_out_len())
    }

    fn run_capacity_error(capacity: usize) -> RuntimeError {
        RuntimeError::ActiveRunCapacityExceeded { capacity }
    }
}

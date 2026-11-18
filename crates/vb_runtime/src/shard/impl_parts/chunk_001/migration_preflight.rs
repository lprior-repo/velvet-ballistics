// Destination validation for pending-command migration.
// Read-only authority validation, no reservation or queue mutation.
impl Shard {
    pub(crate) fn preflight_migration_command(
        &self,
        command: &ShardCommand,
    ) -> RuntimeResult<Option<u32>> {
        if self.control.shutting_down {
            return Err(RuntimeError::ShutdownInProgress);
        }
        if let ShardCommand::ActivateOwnedSubmission {
            run,
            expected_source_digest,
        } = command
        {
            let verified = self.preflight_owned_activation(*run, *expected_source_digest)?;
            if !verified.matches(*run, *expected_source_digest) {
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "owned_activation_queue_authority_mismatch",
                });
            }
            return Ok(minimum_queue_depth(
                self.active_run_queue_depth(command),
                verified.depth(),
            ));
        }
        self.prepare_submission_inputs(command)?;
        self.probe_submission_journal(command)?;
        self.admitted_queue_depth(command)
    }

    pub(crate) fn require_migration_source_unowned(
        &self,
        command: &ShardCommand,
    ) -> RuntimeResult<()> {
        if command.run_id().is_some_and(|run| {
            self.run_state_contains(run)
                || self.checked_out_run_contains(run)
                || self.runtime_state_get(run).is_some()
                || self.charged_quotas.contains_key(&run)
                || self.terminal_runs_contains(run)
        }) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "migrate_source_run_owned",
            });
        }
        Ok(())
    }
}

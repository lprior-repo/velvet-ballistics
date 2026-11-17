impl Shard {
    // Transfers command custody only; source run ownership is never moved.
    pub(crate) fn transfer_pending_commands_to(&mut self, target: &mut Self) -> RuntimeResult<()> {
        self.command_queue.require_unfenced()?;
        target.command_queue.require_unfenced()?;
        if self.command_queue.is_empty() {
            return Ok(());
        }
        if target.control.shutting_down {
            return Err(RuntimeError::ShutdownInProgress);
        }
        if !self.quota.shares_with(&target.quota) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "migration_quota_ledger_mismatch",
            });
        }
        if self.command_queue.len() > target.command_queue.remaining_capacity() {
            return Err(RuntimeError::QueueFull);
        }
        self.command_queue.stage_migration_source()?;
        match self.transfer_staged_commands(target) {
            Ok(()) => Ok(()),
            Err(error) => self.restore_failed_command_transfer(target, error),
        }
    }

    fn transfer_staged_commands(&mut self, target: &mut Self) -> RuntimeResult<()> {
        target
            .command_queue
            .prepare_migration_target(&self.command_queue)?;
        self.validate_staged_migration(target)?;
        target
            .command_queue
            .transfer_staged_migration(&mut self.command_queue)
    }

    fn validate_staged_migration(&self, target: &mut Self) -> RuntimeResult<()> {
        for queued in self.command_queue.migration_entries() {
            self.require_migration_source_unowned(&queued.command)?;
            let depth = target.preflight_migration_command(&queued.command)?;
            if let Some(reservation) = queued.quota {
                if target.pending_submission_quota(&queued.command)? != Some(reservation) {
                    return Err(RuntimeError::UnsupportedOperation {
                        operation: "migration_quota_reservation_mismatch",
                    });
                }
            }
            if let Some(run) = queued.command.run_id() {
                target.command_queue.restrict_migration_depth(run, depth)?;
            }
        }
        Ok(())
    }

    fn restore_failed_command_transfer(
        &mut self,
        target: &mut Self,
        error: RuntimeError,
    ) -> RuntimeResult<()> {
        let restored = if target.command_queue.is_migration_fenced() {
            target
                .command_queue
                .rollback_migration_pair(&mut self.command_queue)
        } else {
            self.command_queue.restore_migration_source()
        };
        match restored {
            Ok(()) => Err(error),
            Err(rollback) => Err(RuntimeError::rollback_failed(
                "migration_queue_transfer",
                error,
                rollback,
            )),
        }
    }
}

include!("migration_preflight.rs");

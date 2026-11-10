impl Shard {
    fn recovered_retry_timer(
        &mut self,
        boundary: crate::recovery::RecoveredRunBoundary,
        checkpoint: Option<&crate::run_checkpoint::RunCheckpointV1>,
    ) -> RuntimeResult<Option<PendingTimer>> {
        let Some(timer) = boundary.retry_timer() else {
            return Ok(None);
        };
        let checkpoint = checkpoint.ok_or(RuntimeError::InvalidTimerFire)?;
        if checkpoint.retry_step != timer.step || timer.generation == 0 {
            return Err(RuntimeError::InvalidTimerFire);
        }
        self.reserve_pending_timer_slot(checkpoint.run_id)?;
        let deadline = retry_deadline(timer.deadline_unix_ms)?;
        Ok(Some(PendingTimer {
            step: timer.step,
            generation: timer.generation,
            deadline,
            kind: PendingTimerKind::Wait,
        }))
    }

    fn restore_retry_boundary_frame(
        &self,
        state: &mut RunState,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<()> {
        if let Some(timer) = boundary.retry_timer() {
            state
                .frame
                .mark_running(timer.step)
                .and_then(|_| state.frame.mark_waiting(timer.step))
                .map_err(|_| RuntimeError::InvalidTimerFire)?;
        } else if let Some(ticket) = boundary.pending_action_ticket() {
            state
                .frame
                .mark_running(ticket.ticket().step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        }
        Ok(())
    }
}

fn retry_deadline(deadline_unix_ms: u64) -> RuntimeResult<std::time::Instant> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch()
        .ok_or(RuntimeError::InvalidTimerFire)?;
    // UTC is truncated to milliseconds; round upwards to prevent early firing.
    let remaining = deadline_unix_ms
        .saturating_sub(now)
        .checked_add(1)
        .ok_or(RuntimeError::InvalidTimerFire)?;
    std::time::Instant::now()
        .checked_add(std::time::Duration::from_millis(remaining))
        .ok_or(RuntimeError::InvalidTimerFire)
}

impl Shard {
    /// Restores a recovered open ask or untimed wait boundary in memory.
    ///
    /// Both boundaries suspend without a drive step and without journal
    /// mutation. Returns false for every other boundary kind.
    pub(crate) fn restore_recovered_suspended_boundary(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        match boundary.kind() {
            crate::recovery::RecoveredRunBoundaryKind::OpenAsk => {
                self.restore_recovered_open_ask(run, boundary)
            }
            crate::recovery::RecoveredRunBoundaryKind::OpenWaitEvent => {
                self.restore_recovered_open_wait(run, boundary)
            }
            _ => Ok(false),
        }
    }

    fn restore_recovered_open_ask(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(ask) = boundary.open_ask() else {
            return Ok(false);
        };
        self.record_pending_ask_authority(run, ask.step(), ask.boundary_seq());
        self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }

    fn restore_recovered_open_wait(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(wait) = boundary.open_wait_event() else {
            return Ok(false);
        };
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_waiting(wait.step())
            .map_err(|_| RuntimeError::Recovery {
                error: String::from("recover open wait frame state"),
            })?;
        self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }
}

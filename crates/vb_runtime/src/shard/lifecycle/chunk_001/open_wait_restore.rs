impl Shard {
    pub(crate) fn restore_open_wait_event(
        &mut self,
        run: RunId,
        boundary: crate::recovery::RecoveredRunBoundary,
    ) -> RuntimeResult<bool> {
        let Some(wait) = boundary.open_wait_event() else {
            return Ok(false);
        };
        self.run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?
            .frame
            .mark_waiting(wait.step())
            .map_err(|_| RuntimeError::Recovery {
                error: String::from("recover open wait frame state"),
            })?;
        self.apply(run, crate::shard::types::RuntimeEvent::AwaitTimer)?;
        Ok(true)
    }
}

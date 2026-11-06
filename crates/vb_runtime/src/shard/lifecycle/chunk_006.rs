impl Shard {
    /// §77.17 run-start enforcement: re-checks the run's journaled
    /// compatibility decisions before its first live effects.
    ///
    /// Runs only when the run is (re)starting (`Initial` after submit or
    /// recovery, `Resuming` after resume). Continuation drives enter with
    /// `Running` or `Resumable`, so a started run never re-scans its
    /// journal per step. Noop and volatile journals hold no decisions and
    /// pass unchanged. A blocked (re)start leaves the run admitted and
    /// retryable, exactly like a pre-commit drive failure.
    fn gate_compat_run_start(&self, run: RunId) -> RuntimeResult<()> {
        match self.runtime_state_get(run) {
            Some(RuntimeState::Initial) | Some(RuntimeState::Resuming) => {}
            _ => return Ok(()),
        }
        let Some(journal) = self.journal.storage_journal() else {
            return Ok(());
        };
        vb_storage::compat::gate_run_start(&journal, run)
            .map(|_| ())
            .map_err(|error| RuntimeError::compat_gate_blocked(run, error))
    }
}

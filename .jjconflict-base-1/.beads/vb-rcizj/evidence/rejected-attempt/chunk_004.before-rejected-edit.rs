impl Shard {
    fn reserve_cancel_kill_terminalization(&mut self, run: RunId) -> RuntimeResult<()> {
        self.reserve_checked_out_run_slot(run)?;
        self.reserve_terminal_run_slot(run)
    }

    fn append_cancel_terminal_events(
        &mut self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        self.append_terminal_events_with_pending_action(
            run,
            RuntimeJournalEvent::RunCancelled { run, reason },
        )
    }

    fn append_kill_terminal_events(&mut self, run: RunId) -> RuntimeResult<()> {
        self.append_terminal_events_with_pending_action(run, RuntimeJournalEvent::RunKilled { run })
    }

    fn append_terminal_events_with_pending_action(
        &mut self,
        run: RunId,
        terminal: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        match self.pending_action_get(run) {
            Some(ticket) if ticket.run == run => self.append_journal_events_atomically([
                RuntimeJournalEvent::ActionAbandoned { ticket },
                terminal,
            ]),
            Some(_) => Err(RuntimeError::InvalidActionCompletion),
            None => self.append_journal_event(terminal),
        }
    }

    fn require_pending_action_ownership(&self, ticket: ActionTicket) -> RuntimeResult<()> {
        if !self.run_state_contains(ticket.run) {
            return Err(RuntimeError::RunNotFound);
        }
        match self.pending_action_get(ticket.run) {
            Some(pending) if pending == ticket => Ok(()),
            Some(_) | None => Err(RuntimeError::InvalidActionCompletion),
        }
    }

    fn require_legacy_pending_action_ownership(
        &self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<ActionTicket> {
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        match self.pending_action_get(run) {
            Some(pending) if pending.run == run && pending.step == step => Ok(pending),
            Some(_) | None => Err(RuntimeError::InvalidActionCompletion),
        }
    }
}

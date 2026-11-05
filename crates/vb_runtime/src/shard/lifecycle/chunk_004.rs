impl Shard {
    fn reserve_cancel_kill_terminalization(&mut self, run: RunId) -> RuntimeResult<()> {
        self.reserve_checked_out_run_slot(run)?;
        self.reserve_terminal_run_slot(run)
    }

    fn append_cancel_terminal_events(
        &mut self,
        run: RunId,
        reason: Option<String>,
        attempt: u16,
    ) -> RuntimeResult<()> {
        self.append_terminal_events_with_pending_action(
            run,
            RuntimeJournalEvent::RunCancelled {
                run,
                reason,
                attempt,
            },
        )
    }

    fn append_kill_terminal_events(&mut self, run: RunId, attempt: u16) -> RuntimeResult<()> {
        self.append_terminal_events_with_pending_action(
            run,
            RuntimeJournalEvent::RunKilled { run, attempt },
        )
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

    pub(super) fn require_pending_action_ownership(
        &self,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        if !self.run_state_contains(ticket.run) {
            return Err(RuntimeError::RunNotFound);
        }
        match self.pending_action_get(ticket.run) {
            Some(pending) if pending.same_invocation(&ticket) => {
                // Identity is token-agnostic, so this is where a superseded
                // issuance (same invocation, older token) is caught. Tampered
                // fields fall through to the specific diagnostics below.
                self.require_current_fencing_token(ticket)
            }
            Some(pending)
                if ticket.attempt > 0
                    && ticket.attempt < pending.attempt
                    && ticket.run == pending.run
                    && ticket.step == pending.step
                    && ticket.action == pending.action
                    && ticket.capacity == pending.capacity
                    && vb_core::action::action_ticket_has_valid_key(ticket)
                    && ticket.seq <= pending.seq =>
            {
                Err(RuntimeError::StaleAttempt {
                    incoming: ticket.attempt,
                    current: pending.attempt,
                })
            }
            Some(_) | None => Err(RuntimeError::InvalidActionCompletion),
        }
    }

    /// Rejects a completion whose fencing token was superseded by a later
    /// issuance for the same run.
    fn require_current_fencing_token(&self, ticket: ActionTicket) -> RuntimeResult<()> {
        if self.fencing_tokens.accepts(ticket.run, ticket.fencing_token) {
            Ok(())
        } else {
            Err(RuntimeError::StaleFencingToken {
                incoming: ticket.fencing_token,
            })
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

impl Shard {
    /// Emits an `ActionAbandoned` journal event for every in-flight
    /// action ticket owned by `run`. The emitted events precede the
    /// run-terminal event so recovery observes the abandonments
    /// before the cancel/kill marker.
    fn emit_action_abandoned_for_pending(&mut self, run: RunId) -> RuntimeResult<()> {
        let tickets = self.collect_pending_action_tickets(run);
        for ticket in &tickets {
            self.append_journal_event(RuntimeJournalEvent::ActionAbandoned { ticket: *ticket })?;
        }
        for _ in &tickets {
            let _removed_ticket = self.pending_action_remove(run);
        }
        Ok(())
    }

    /// Drains every in-flight action ticket owned by `run` into a
    /// owned `Vec`.
    fn collect_pending_action_tickets(
        &self,
        run: RunId,
    ) -> Vec<vb_core::action::ActionTicket> {
        self.pending_action_clone()
            .into_iter()
            .filter_map(|(candidate, ticket)| {
                if candidate == run { Some(ticket) } else { None }
            })
            .collect()
    }
}

impl Runtime {
    /// Completes an action for a run.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        if let Some(ticket) = self.lookup_pending_action_ticket(run, u64::from(step.get())) {
            shard.reject_unregistered_completion(ticket)?;
        }
        shard.enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action for a run with its typed output payload.
    ///
    /// Validates the ticket against the current run state before enqueuing.
    /// Returns `InvalidActionCompletion` if the ticket is invalid (wrong action,
    /// non-running step, or stale attempt).
    pub fn complete_action_with_output(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for_mut(ticket.run)?;
        shard.reject_unregistered_completion(ticket)?;
        // Validate ticket before enqueuing — fail fast with InvalidActionCompletion
        // if the ticket doesn't match the current run state.
        if let Some(state) = shard.run_state_get(ticket.run) {
            crate::shard::lifecycle::preflight_action_completion(state, ticket, output)?;
        }
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        shard.enqueue(ShardCommand::RuntimeActionFailed { ticket, failure })
    }

    /// Looks up the canonical in-flight action ticket for `run`.
    ///
    /// IPC handlers receive only a `(run_id, ticket: u64)` wire pair. The wire
    /// `ticket` encodes only the lower-16-bit step index and does not carry the
    /// real `seq`, `action`, `attempt`, `idempotency_key`, or `capacity`
    /// values. The canonical ticket is held by the shard's `pending_actions`
    /// index and is the only authoritative source of those fields.
    ///
    /// Returns `None` if:
    /// - the run is not found,
    /// - the run has no in-flight action ticket,
    /// - the wire `ticket`'s lower 16 bits do not match the canonical step.
    ///
    /// This lookup is the boundary that prevents the IPC layer from
    /// fabricating dummy `seq`/`action`/`idempotency_key`/`capacity` fields
    /// (see bead vb-xb62s). Callers must fail closed if this returns `None`
    /// and must never invent fields the wire did not carry.
    #[must_use]
    pub fn lookup_pending_action_ticket(
        &self,
        run: RunId,
        ticket_wire: u64,
    ) -> Option<vb_core::action::ActionTicket> {
        let shard = self.shard_for_run(run).ok()?;
        let canonical = shard.pending_action_get(run)?;
        let wire_step = u16::try_from(ticket_wire).ok()?;
        if canonical.step.get() != wire_step {
            return None;
        }
        Some(canonical)
    }

    /// Looks up the opaque authority for the run's currently suspended Ask boundary.
    ///
    /// The ticket is issued only from the shard's current Asking state and its
    /// journal-backed AskScheduled boundary. A stale or unknown boundary returns
    /// `None`; callers must not construct or alter ticket fields themselves.
    #[must_use]
    pub fn lookup_pending_ask_ticket(
        &self,
        run: RunId,
        ask_step: vb_core::ids::StepIdx,
    ) -> Option<crate::shard::AskTicket> {
        self.shard_for_run(run)
            .ok()?
            .pending_ask_ticket(run, ask_step)
    }

    /// Lists trace events for a run without draining the shard trace ring.
    ///
    /// RA-030 wave-15 (vb-sxkz6): a run may have been migrated to a shard
    /// other than the home shard selected by the hash of `RunId`. Scan all
    /// shards so the trace ring of the owning shard is read.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard = self.shard_for_run(run)?;
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Answers an ask with an explicit typed payload and resume ticket.
    ///
    /// RA-030: a run may have been migrated to a shard other than the home
    /// shard selected by the hash of `RunId` (see `migrate_shard` / RA-021).
    /// Test the home shard first as a fast path, then scan the remaining
    /// shards for active or terminal ownership so the answer is routed to
    /// whichever shard actually holds the run state.
    pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run();
        let shard = self.shard_for_run(run)?;
        shard.validate_pending_ask_answer(answer)?;
        shard.enqueue(ShardCommand::AskAnswered { answer })
    }

    /// Submits an operator decision to an open manual gate.
    ///
    /// The gate must already be open for the run: the submission is
    /// rejected before enqueueing when the gate is closed or belongs to
    /// another run, mirroring the fail-fast pre-validation of
    /// [`Runtime::answer_ask`]. An accepted decision is enqueued for
    /// drive-time application, where it is journaled before any state
    /// change; the decision counts against the run's admitted per-run
    /// queue depth.
    pub fn submit_gate_decision(&self, run: RunId, input: GateDecisionInput) -> RuntimeResult<()> {
        let shard = self.shard_for_run(run)?;
        shard.validate_open_gate_decision(run, &input)?;
        shard.enqueue(ShardCommand::SubmitGateDecision { run, input })
    }

    /// Legacy run-only timer delivery is fail-closed because it carries no authority.
    pub fn timer_fired(&self, run: RunId) -> RuntimeResult<()> {
        let _shard = self.shard_for_run(run)?;
        Err(RuntimeError::InvalidTimerFire)
    }

    /// Captures the current timer authority for tests and typed scheduler handoff.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run's timer entry.
    /// Returns `Err(RuntimeError::RunNotFound)` if no shard owns the run.
    /// Returns `Err(RuntimeError::InvalidTimerFire)` when the owner shard has
    /// no live timer entry for the run.
    pub fn capture_timer_entry(&self, run: RunId) -> RuntimeResult<TimerEntry> {
        let shard = self.shard_for_run(run)?;
        shard.timer_entry(run).ok_or(RuntimeError::InvalidTimerFire)
    }

    /// Advances a run from a timer-wheel-captured authority entry.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run owner.
    pub fn timer_entry_fired(&self, entry: TimerEntry) -> RuntimeResult<()> {
        let shard = self.shard_for_run(entry.run)?;
        shard.enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        })
    }

    /// Takes the latest inspect response from the run's shard.
    ///
    /// RA-030 wave-15 (vb-sxkz6): scans all shards for the run owner.
    pub fn take_inspect_response(&mut self, run: RunId) -> RuntimeResult<Option<InspectResponse>> {
        let shard = self.shard_for_run_mut(run)?;
        Ok(shard.take_inspect_response())
    }

    /// Drains all trace events from all shards.
    pub fn drain_trace(&mut self) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        for shard in &mut self.shards {
            let capacity = shard.trace_ring_mut().capacity();
            shard.trace_ring_mut().drain_into(capacity, &mut events);
        }
        events
    }
}

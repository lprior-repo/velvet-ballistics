use crate::shard::types::TerminalOutcome;

impl Shard {
    /// Handles an ask answer for a suspended run.
    ///
    /// # RS-104 durability ordering (B-012 pattern):
    /// The answer sequence (`SlotWritten` followed by `AskAnswered`) is
    /// appended to the journal BEFORE any frame or timer state mutation.
    /// If either durable `append_journal_event_durable` call fails, the
    /// function returns the typed error WITHOUT touching the frame or
    /// the pending timer, leaving the run suspendable so the caller can
    /// retry the answer. This matches the B-012 fix used by
    /// `handle_cancel` and prevents a journal failure from leaving the
    /// run with an unsynchronized in-memory progress (frame advanced,
    /// timer removed) and no durable answer evidence.
    ///
    /// # Flux refinement (PO-vb282my-AA-FLUX-001):
    /// SlotWritten-before-AskAnswered ordering guarantee:
    /// The AskAnswered journal append is only reachable AFTER a
    /// successful SlotWritten journal append. Both appends use
    /// `append_journal_event_durable` (synchronous, buffer-bypassing)
    /// so the events are committed before any state mutation. If
    /// SlotWritten fails (returns Err), AskAnswered is never attempted.
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, answer: AskAnswer) -> RuntimeResult<()>
    ///     ensures result.is_err() || journal_has(SlotWritten{run, slot})
    /// )]
    /// ```
    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        let pending_timer = self
            .pending_timer_get(run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if pending_timer.step != answer.ticket.ask_step
            || pending_timer.kind != PendingTimerKind::Ask
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        // RS-103 preflight + RS-104: read-only access to the run state so
        // the validation below does not hold a mutable borrow across the
        // journal appends. The durable appends happen BEFORE any frame or
        // timer mutation; on failure the function returns Err and the run
        // stays suspendable for retry (mirrors the B-012 fix in
        // `handle_cancel` / `handle_kill`).
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let resume_step = state
            .workflow
            .node(pending_timer.step)
            .and_then(|node| node.next)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let ask_resume = state
            .workflow
            .node(resume_step)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let expected_answer_slot = match ask_resume.kind {
            vb_core::workflow::CompiledNodeKind::AskResume { answer } => answer,
            _ => return Err(RuntimeError::InvalidActionCompletion),
        };
        if answer.ticket.resume_step != resume_step
            || answer.answer_slot != expected_answer_slot
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let contract = state.workflow.resource_contract();
        if answer.taint == Taint::Secret && !contract.allows_secret_results {
            return Err(RuntimeError::SecretResultNotAllowed);
        }
        if answer.encoded_len > contract.max_ipc_payload_bytes {
            return Err(RuntimeError::IpcPayloadSizeExceeded {
                size: answer.encoded_len,
                max: contract.max_ipc_payload_bytes,
            });
        }
        // RS-104: pre-encode the answer value and capture the slot/step
        // BEFORE any in-memory mutation so the journal appends below can
        // succeed without depending on frame state. The `?` operator on
        // both appends propagates the typed error WITHOUT touching the
        // frame or the pending timer — the run stays suspendable for a
        // retry on journal failure.
        let encoded_answer_value =
            postcard::to_allocvec(&answer.value).map_err(|_| RuntimeError::EncodeFailed)?;
        let answer_step = answer.ticket.ask_step;
        let answer_slot = answer.answer_slot;
        // RS-104 / PO-vb282my-AA-FLUX-001: append SlotWritten first
        // (durable variant bypasses the coalesce buffer per RS-107),
        // then AskAnswered. Both appends happen BEFORE any frame or
        // timer state mutation, so a journal failure leaves the run
        // suspendable: the caller can retry the answer without an
        // inconsistent in-memory state.
        self.append_journal_event_durable(RuntimeJournalEvent::SlotWritten {
            run,
            slot: answer_slot,
            value: encoded_answer_value,
            taint: answer.taint,
            extra: None,
        })?;
        self.append_journal_event_durable(RuntimeJournalEvent::AskAnswered {
            run,
            step: answer_step,
            slot: answer_slot,
        })?;
        // RS-104: state mutations now occur AFTER the answer sequence is
        // durable. A failure here (frame write rejected, etc.) leaves
        // the journal evidence intact; recovery can replay by reading
        // the SlotWritten record and re-applying the slot write. The
        // pending timer is removed only after the answer sequence is
        // durable so a journal failure does not orphan a timer the run
        // can no longer authorize.
        {
            let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
            state
                .frame
                .write_slot_with_taint(answer_slot, answer.value, answer.taint)
                .map_err(|_| RuntimeError::RunNotFound)?;
            state
                .frame
                .set_pc(answer.ticket.resume_step)
                .map_err(|_| RuntimeError::RunNotFound)?;
        }
        self.pending_timer_remove(run);
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer_step,
            slot: answer_slot,
        });
        // RS-004: derive the live per-step attempt counter from
        // `state.action_attempts` so the durable journal record matches
        // the same source as `ActionFailed`. Ask steps do not currently
        // drive `action_attempts` (asks have no ticket-based retry), so
        // this resolves to 1 via the `.max(1)` clamp and remains
        // forward-compatible if ask retry tracking is added later.
        let attempt = self
            .run_state_get(run)
            .and_then(|state| {
                state
                    .action_attempts
                    .get(answer.ticket.ask_step.as_usize())
                    .copied()
            })
            .unwrap_or(0)
            .max(1);
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step: answer.ticket.ask_step,
            output: answer.answer_slot,
            attempt,
        })?;
        self.drive_run(run)
    }

    pub(crate) fn handle_timer(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        let Some(current_timer) = self.pending_timer_get(run) else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let mut state = self.take_run_state(run)?;
        let timer = match self.pending_timer_remove(run) {
            Some(timer) => timer,
            None => {
                self.run_state_insert(run, state);
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        if let Err(error) =
            crate::shard::helpers::advance_after_timer_fire(&mut state, timer)
        {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
        match timer.kind {
            PendingTimerKind::Wait => {
                if let Err(error) =
                    self.append_journal_event(RuntimeJournalEvent::WaitResolved {
                        run,
                        step: timer.step,
                    })
                {
                    self.run_state_insert(run, state);
                    return Err(error);
                }
            }
            PendingTimerKind::Ask => {}
        }
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        if let Err(error) = self.flush_evidence(run, &mut evidence) {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
        self.apply_drive_result(run, state, result)
    }

    pub(crate) fn handle_cancel(
        &mut self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        // RQ-W0-17 / RQ-W0-19: cancel on an already-terminal run is a typed
        // no-op. The first cancel wins; subsequent cancels must not produce a
        // new journal event or increment counters. This preserves the
        // idempotency contract exercised by
        // cancel_after_kill_is_typed_noop and
        // cancel_kill_alternating_keeps_terminalization_idempotent.
        if self.terminal_runs_contains(run) {
            return Ok(());
        }
        self.pending_timer_remove(run);
        // B-012: journal the RunCancelled event BEFORE state removal so the
        // terminal event is durable on disk. If the journal append fails,
        // we propagate the typed error and leave state intact for retry
        // (no event recorded, run not removed — caller can retry cancel).
        // The durable variant bypasses the coalesce buffer to guarantee
        // synchronous durability per RS-005 / RQ-W0-19.
        self.append_journal_event_durable(RuntimeJournalEvent::RunCancelled { run, reason })?;
        if let Some(state) = self.run_state_remove(run) {
            // RS-005: drop any coalesce-buffer entries for this run. A
            // previous flush failure (e.g. the journal permanently rejecting a
            // `StepStarted` event) leaves buffered events in `coalesce_buffer`
            // per RQ-W0-19 so a retry could persist them; once the operator
            // cancels, those entries are orphans and would block the terminal
            // flush.
            self.discard_buffered_events_for_run(run);
            self.release_frame(state.frame);
            self.terminal_runs_insert(run);
            self.terminal_outcome_record(run, TerminalOutcome::Cancelled);
            // RQ-W0-17: cancel is no longer conflated with fail, but the
            // legacy `runs_failed` counter still counts every non-successful
            // terminal lifecycle so historical observability contracts hold.
            self.counters.inc_cancelled();
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
        }
        // RS-101: route the cancel through the FSM TerminalRemove event so
        // `runtime_states` is cleared consistently with the other terminal
        // paths (fail/finish/done). Without this, the FSM map retains a
        // stale entry (Initial/Running/Resumable) for a cancelled run.
        _ = self.apply(run, RuntimeEvent::TerminalRemove);
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_kill(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        // RQ-W0-17 / RQ-W0-19: kill on an already-terminal run is a typed
        // no-op. The first terminalization wins; subsequent kills must not
        // produce a new journal event or increment counters. Mirrors the
        // handle_cancel idempotency guard. After the first terminalization
        // (Cancel or Kill), the run lives in `terminal_runs` but NOT in
        // `runs`, so this guard short-circuits before any journaling.
        if self.terminal_runs_contains(run) {
            return Ok(());
        }
        // RS-102 / B-012: journal the RunKilled event BEFORE state removal
        // so the terminal event is durable on disk. If the journal append
        // fails, we propagate the typed error and leave state intact for
        // retry (no event recorded, run not removed — caller can retry
        // kill). Mirrors the B-012 fix in `handle_cancel` and the
        // RS-104 fix in `handle_ask_answer`. The durable variant
        // bypasses the coalesce buffer to guarantee synchronous
        // durability per RS-005 / RQ-W0-19.
        self.append_journal_event_durable(RuntimeJournalEvent::RunKilled { run, reason: reason.clone() })?;
        self.pending_timer_remove(run);
        let state = self
            .run_state_remove(run)
            .ok_or(RuntimeError::RunNotFound)?;
        // RS-005: drop any coalesce-buffer entries for this run before
        // appending the terminal event. Symmetric with `handle_cancel`:
        // orphaned buffered events must not block the terminal flush.
        self.discard_buffered_events_for_run(run);
        self.release_frame(state.frame);
        self.terminal_runs_insert(run);
        self.terminal_outcome_record(run, TerminalOutcome::Killed);
        // RQ-W0-17: kill is no longer conflated with fail, but the
        // legacy `runs_failed` counter still counts every non-successful
        // terminal lifecycle so historical observability contracts hold.
        self.counters.inc_killed();
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunKilled { run });
        // RS-101: route the kill through the FSM TerminalRemove event so
        // `runtime_states` is cleared consistently with the other terminal
        // paths (fail/finish/done). Without this, the FSM map retains a
        // stale entry (Initial/Running/Resumable) for a killed run.
        _ = self.apply(run, RuntimeEvent::TerminalRemove);
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
        Ok(())
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        if let Err(error) = self.flush_evidence(run, &mut evidence) {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
        self.apply_drive_result(run, state, result)
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        match self.run_state_remove(run) {
            Some(state) => Ok(state),
            None => Err(RuntimeError::RunNotFound),
        }
    }

    fn drive_state(
        state: &mut RunState,
        step_budget_per_tick: u64,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        let mut budget = vb_core::engine::StepBudget::new(step_budget_per_tick);
        let empty_caps = CapabilitySet::empty();
        let granted = state
            .admission
            .as_ref()
            .map(|a| a.granted_capabilities())
            .unwrap_or(&empty_caps);
        drive_deterministic_full(
            &state.workflow,
            &mut state.frame,
            &mut budget,
            &mut state.store,
            &state.action_contracts,
            RetryPolicy::NEVER,
            evidence,
            &mut state.collect_states,
            granted,
        )
    }

     fn apply_drive_result(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeEngineResult<RuntimeSignal>,
    ) -> RuntimeResult<()> {
        match result {
            Ok(RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted) => {
                _ = self.apply(run, RuntimeEvent::DriveContinue);
                self.keep_run_with_snapshot(run, state)?;
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.apply_terminal_finished(run, state),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                self.apply_awaiting_action(run, state, ticket)
            }
            Ok(RuntimeSignal::AwaitingWait(deadline_slot)) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Wait, deadline_slot)
            }
            Ok(RuntimeSignal::AwaitingEvent { event, timeout_slot }) => {
                // CE-001 fix: WaitEvent without a timeout has no deadline slot.
                // The event slot MUST NOT be substituted as a deadline; an event
                // without a timeout never races the timer and is resumed only
                // when the event fires. With a timeout, the deadline slot is the
                // workflow's `timeout_slot`, not the event slot.
                let _ = event;
                self.apply_awaiting_event(run, state, timeout_slot)
            }
            Ok(RuntimeSignal::AwaitingAsk(timeout_slot)) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Ask, timeout_slot.unwrap_or(vb_core::ids::SlotIdx::ZERO))
            }
            Err(_) => self.apply_terminal_failed(run, state),
        }
    }

    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        _ = self.apply(run, RuntimeEvent::AwaitAction);
        self.await_action(run, state, ticket)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        deadline_slot: SlotIdx,
    ) -> RuntimeResult<()> {
        self.await_timer(run, state, kind, deadline_slot)?;
        _ = self.apply(run, RuntimeEvent::AwaitTimer);
        Ok(())
    }

    /// CE-001: WaitEvent authority. The deadline slot is the
    /// `timeout_slot` when present; otherwise there is no deadline
    /// and the host waits for the event to fire. The event slot is
    /// never substituted as a deadline.
    fn apply_awaiting_event(
        &mut self,
        run: RunId,
        state: RunState,
        timeout_slot: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        match timeout_slot {
            Some(slot) => self.apply_awaiting_timer(run, state, PendingTimerKind::Wait, slot),
            None => {
                // Event-only wait: no deadline. Insert the run without a
                // pending timer so the run is suspended until an external
                // event command resumes it.
                //
                // RS-107 follow-up (durable variant): use
                // `append_journal_event_durable` so the WaitScheduled
                // sentinel is durably persisted before the run is
                // re-inserted into the suspended state. The pre-fix
                // `let _ = self.append_journal_event(...)` silently
                // swallowed the error (Holzman violation) AND would only
                // buffer the event in `coalesce_buffer` when the window
                // is active, so a crash before the next flush would lose
                // the sentinel and recovery would fail to rehydrate the
                // suspended wait. Restore `state` on failure so the run
                // is not silently dropped from shard bookkeeping (RS-005).
                self.counters.add_steps(state.frame.executed());
                let step = state.frame.pc();
                if let Err(error) = self
                    .append_journal_event_durable(RuntimeJournalEvent::WaitScheduled {
                        run,
                        step,
                        deadline_ms: u64::MAX,
                    })
                {
                    self.run_state_insert(run, state);
                    return Err(error);
                }
                self.run_state_insert(run, state);
                Ok(())
            }
        }
    }

    fn apply_terminal_finished(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        _ = self.apply(run, RuntimeEvent::DriveFinished);
        self.finish_run(run, state)
    }

    fn apply_terminal_failed(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        // apply() handles runtime_states mutation; fail_run_state handles cleanup only
        _ = self.apply(run, RuntimeEvent::Fail);
        self.fail_run_state(run, state)
    }
}

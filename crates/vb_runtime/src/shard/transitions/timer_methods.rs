impl Shard {
    /// Transitions a run to awaiting a timer (wait or ask timeout).
    pub(crate) fn await_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<TransitionOutcome> {
        let registration = self.prepare_await_timer_registration(run, state, kind)?;
        let AwaitTimerRegistration {
            state,
            step,
            register_timer,
            timer,
            ask_boundary_seq,
        } = registration;
        let mut state = state;
        if register_timer || kind == PendingTimerKind::Ask {
            let (updated, within_budget) =
                self.append_await_timer_schedule(run, state, step, kind)?;
            state = updated;
            if !within_budget {
                return Ok(TransitionOutcome::JournalBudgetExceeded(Box::new(state)));
            }
        }
        self.finish_await_timer_registration(run, state, step, timer, ask_boundary_seq)
    }

    fn append_await_timer_schedule(
        &mut self,
        run: RunId,
        state: RunState,
        step: vb_core::ids::StepIdx,
        kind: PendingTimerKind,
    ) -> RuntimeResult<(RunState, bool)> {
        let event = match kind {
            PendingTimerKind::Wait => RuntimeJournalEvent::WaitScheduled { run, step },
            PendingTimerKind::Ask => RuntimeJournalEvent::AskScheduled { run, step },
        };
        let projected = self.journal_bytes_after_append(&state, core::slice::from_ref(&event));
        let (mut state, projected) = self.preserve_await_state(run, state, projected)?;
        if Self::journal_bytes_exceeds_budget(&state, projected) {
            return Ok((state, false));
        }
        if let Err(error) = self.append_journal_event(event) {
            self.run_state_insert(run, state)?;
            return Err(error);
        }
        state.journal_bytes_written = projected;
        Ok((state, true))
    }

    fn prepare_await_timer_registration(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<AwaitTimerRegistration> {
        self.add_executed_step_delta(run, state.frame.executed());
        let step = state.frame.pc();
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        let (state, generation) =
            self.prepare_await_timer_generation(run, state, register_timer)?;
        let ask_boundary_seq = self.prepare_ask_boundary_sequence(run, kind);
        let (state, ask_boundary_seq) = self.preserve_await_state(run, state, ask_boundary_seq)?;
        let timer = generation.map(|generation| Self::new_await_timer(step, kind, generation));
        let state = self.prepare_await_timer_storage(run, state, timer)?;
        Ok(AwaitTimerRegistration {
            state,
            step,
            register_timer,
            timer,
            ask_boundary_seq,
        })
    }

    fn preserve_await_state<T>(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeResult<T>,
    ) -> RuntimeResult<(RunState, T)> {
        match result {
            Ok(value) => Ok((state, value)),
            Err(primary) => {
                if let Err(rollback) = self.run_state_insert(run, state) {
                    return Err(RuntimeError::rollback_failed(
                        "await timer preflight",
                        primary,
                        rollback,
                    ));
                }
                Err(primary)
            }
        }
    }

    fn new_await_timer(
        step: vb_core::ids::StepIdx,
        kind: PendingTimerKind,
        generation: u64,
    ) -> PendingTimer {
        PendingTimer {
            step,
            kind,
            generation,
            deadline: Instant::now(),
        }
    }

    fn prepare_await_timer_storage(
        &mut self,
        run: RunId,
        state: RunState,
        timer: Option<PendingTimer>,
    ) -> RuntimeResult<RunState> {
        if let Some(timer) = timer
            && let Err(error) = self.prepare_pending_timer(run, timer)
        {
            self.run_state_insert(run, state)?;
            return Err(error);
        }
        Ok(state)
    }

    fn prepare_await_timer_generation(
        &mut self,
        run: RunId,
        state: RunState,
        register_timer: bool,
    ) -> RuntimeResult<(RunState, Option<u64>)> {
        if !register_timer {
            return Ok((state, None));
        }
        match self.scheduled_timer_generation(run, 0) {
            Ok(generation) => Ok((state, Some(generation))),
            Err(error) => {
                self.run_state_insert(run, state)?;
                Err(error)
            }
        }
    }

    fn prepare_ask_boundary_sequence(
        &mut self,
        run: RunId,
        kind: PendingTimerKind,
    ) -> RuntimeResult<Option<vb_storage::EventSeq>> {
        if kind != PendingTimerKind::Ask {
            return Ok(None);
        }
        self.reserve_pending_ask_authority_slot(run)?;
        Ok(Some(self.journal_sequence_for(run)))
    }

    fn finish_await_timer_registration(
        &mut self,
        run: RunId,
        state: RunState,
        step: vb_core::ids::StepIdx,
        timer: Option<PendingTimer>,
        ask_boundary_seq: Option<vb_storage::EventSeq>,
    ) -> RuntimeResult<TransitionOutcome> {
        if let Some(timer) = timer {
            self.pending_timer_insert(run, timer)?;
        }
        if let Some(boundary_seq) = ask_boundary_seq {
            self.record_pending_ask_authority(run, step, boundary_seq);
        }
        self.run_state_insert(run, state)?;
        Ok(TransitionOutcome::Applied)
    }
}

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
            generation,
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
        self.finish_await_timer_registration(run, state, step, kind, generation, ask_boundary_seq)
    }

    fn append_await_timer_schedule(
        &mut self,
        run: RunId,
        mut state: RunState,
        step: vb_core::ids::StepIdx,
        kind: PendingTimerKind,
    ) -> RuntimeResult<(RunState, bool)> {
        let event = match kind {
            PendingTimerKind::Wait => RuntimeJournalEvent::WaitScheduled { run, step },
            PendingTimerKind::Ask => RuntimeJournalEvent::AskScheduled { run, step },
        };
        let projected = self.journal_bytes_after_append(&state, core::slice::from_ref(&event))?;
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
        let step = state.frame.pc();
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        if register_timer {
            crate::shard::lifecycle::awaiting_deadline(&state, step)?;
        }
        self.add_executed_step_delta(run, state.frame.executed());
        self.prepare_await_timer_registration_slots(
            run,
            kind,
            AwaitTimerRegistration {
                state,
                step,
                register_timer,
                generation: None,
                ask_boundary_seq: None,
            },
        )
    }

    fn prepare_await_timer_registration_slots(
        &mut self,
        run: RunId,
        kind: PendingTimerKind,
        mut registration: AwaitTimerRegistration,
    ) -> RuntimeResult<AwaitTimerRegistration> {
        let (state, generation) = self.prepare_await_timer_generation(
            run,
            registration.state,
            registration.register_timer,
        )?;
        registration.state = state;
        registration.generation = generation;
        registration.ask_boundary_seq = self.prepare_ask_boundary_sequence(run, kind)?;
        if registration.register_timer {
            self.reserve_pending_timer_slot(run)?;
        }
        Ok(registration)
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
        kind: PendingTimerKind,
        generation: Option<u64>,
        ask_boundary_seq: Option<vb_storage::EventSeq>,
    ) -> RuntimeResult<TransitionOutcome> {
        if let Some(generation) = generation {
            self.pending_timer_insert(
                run,
                PendingTimer {
                    step,
                    kind,
                    generation,
                    deadline: Instant::now(),
                },
            )?;
        }
        if let Some(boundary_seq) = ask_boundary_seq {
            self.record_pending_ask_authority(run, step, boundary_seq);
        }
        self.run_state_insert(run, state)?;
        Ok(TransitionOutcome::Applied)
    }
}

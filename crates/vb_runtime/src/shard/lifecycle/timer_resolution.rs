impl Shard {
    fn validate_timer_authority(
        &self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<PendingTimer> {
        let Some(current_timer) = self.pending_timer_get(run) else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        if current_timer.deadline > std::time::Instant::now() {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let state_ref = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_timer_fire(state_ref, current_timer)?;
        Ok(current_timer)
    }

    fn append_timer_resolution_event(
        &mut self,
        run: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<bool> {
        let event = match timer.kind {
            PendingTimerKind::Wait => RuntimeJournalEvent::WaitResolved {
                run,
                step: timer.step,
            },
            PendingTimerKind::Ask => RuntimeJournalEvent::AskTimedOut {
                run,
                step: timer.step,
            },
        };
        self.append_command_journal_events(run, [event])
    }
}

impl Shard {
    fn ask_boundary_sequence(
        &self,
        run: RunId,
        preceding_events: usize,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        let offset = u64::try_from(preceding_events)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.journal_sequence_for(run)
            .get()
            .checked_add(offset)
            .map(vb_storage::EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }

    fn pending_timer_for_awaiting(
        &self,
        state: &RunState,
        step: StepIdx,
        kind: PendingTimerKind,
        generation: u64,
    ) -> PendingTimer {
        PendingTimer {
            step,
            kind,
            generation,
            deadline: awaiting_deadline(state, step),
        }
    }

    pub(super) fn scheduled_timer_generation(
        &self,
        run: RunId,
        preceding_events: usize,
    ) -> RuntimeResult<u64> {
        let offset = u64::try_from(preceding_events)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.journal_sequence_for(run)
            .get()
            .checked_add(offset)
            .and_then(|sequence| sequence.checked_add(1))
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }
}

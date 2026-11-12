impl Shard {
    /// Builds a fail-closed legacy timer-fired command without fabricating authority.
    #[must_use]
    pub fn timer_fired_command(&self, run: RunId) -> ShardCommand {
        ShardCommand::TimerFired {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Wait,
        }
    }

    /// Returns the current typed timer authority for explicit capture.
    #[must_use]
    pub fn timer_entry(&self, run: RunId) -> Option<crate::shard::timer_wheel::TimerEntry> {
        self.pending_timer_get(run)
            .map(|timer| crate::shard::timer_wheel::TimerEntry {
                run,
                generation: timer.generation,
                deadline: timer.deadline,
                kind: timer.kind,
            })
    }

    /// Returns a mutable reference to the trace ring.
    pub fn trace_ring_mut(&mut self) -> &mut TraceRing {
        &mut self.trace_ring
    }

    /// Returns an immutable reference to the trace ring.
    #[must_use]
    pub const fn trace_ring(&self) -> &TraceRing {
        &self.trace_ring
    }

    /// Returns a direct non-queued diagnostic snapshot for a run.
    #[must_use]
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> InspectResponse {
        match self.run_state_get(run) {
            Some(state) => InspectResponse::Found(crate::shard::helpers::snapshot_from_state(
                run,
                correlation,
                state,
            )),
            None => InspectResponse::NotFound { run, correlation },
        }
    }

    /// Takes the latest inspect response, if one is available.
    pub fn take_inspect_response(&mut self) -> Option<InspectResponse> {
        self.inspect_response.take()
    }

    /// Returns true if the shard is shutting down.
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        self.shutting_down
    }

    /// Returns a read-only status snapshot without draining queues or mutating shard state.
    #[must_use]
    pub fn status(&self) -> ShardStatus {
        let shutting_down = self.shutting_down;
        ShardStatus {
            health: if shutting_down {
                ShardHealth::ShuttingDown
            } else {
                ShardHealth::Running
            },
            running: !shutting_down,
            shutting_down,
            command_queue_depth: self.command_queue.len(),
            command_queue_capacity: self.command_queue.capacity(),
            active_runs: self.active_run_count(),
            max_active_runs: self.max_active_runs,
            trace_capacity: self.trace_ring.capacity(),
            trace_dropped: self.trace_ring.dropped(),
            step_budget_per_tick: self.step_budget_per_tick,
            runtime_policy: self.policy,
        }
    }

    /// Drains evidence events from the collector and emits them to the
    /// journal and trace ring. This satisfies the Phase 40/44 evidence
    /// chain requirement: StepStarted before SlotWritten for every step,
    /// followed by StepSucceeded.
    pub(crate) fn prepare_evidence_events(
        &self,
        run: RunId,
        attempt: u16,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<(Vec<RuntimeJournalEvent>, Vec<TraceEvent>)> {
        let event_count = evidence.len();
        let mut journal_events = Vec::new();
        Self::reserve_drive_vec(&mut journal_events, event_count)?;
        let mut trace_events = Vec::new();
        Self::reserve_drive_vec(&mut trace_events, event_count)?;
        for event in evidence.drain() {
            Self::prepare_evidence_event(
                run,
                attempt,
                event,
                &mut journal_events,
                &mut trace_events,
            )?;
        }
        Ok((journal_events, trace_events))
    }

    fn reserve_drive_vec<T>(items: &mut Vec<T>, additional: usize) -> RuntimeResult<()> {
        items
            .try_reserve(additional)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::QueueFull))
    }

    pub(crate) fn push_drive_journal_event(
        events: &mut Vec<RuntimeJournalEvent>,
        event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Self::reserve_drive_vec(events, 1)?;
        events.push(event);
        Ok(())
    }

    fn prepare_evidence_event(
        run: RunId,
        attempt: u16,
        event: EvidenceEvent,
        journal_events: &mut Vec<RuntimeJournalEvent>,
        trace_events: &mut Vec<TraceEvent>,
    ) -> RuntimeResult<()> {
        match event {
            EvidenceEvent::StepStarted { step } => {
                Self::push_drive_journal_event(
                    journal_events,
                    RuntimeJournalEvent::StepStarted { run, step, attempt },
                )?;
                trace_events.push(TraceEvent::StepStarted { run, step });
                Ok(())
            }
            EvidenceEvent::StepFailed { step } => Self::push_drive_journal_event(
                journal_events,
                RuntimeJournalEvent::StepFailed { run, step, attempt },
            ),
            EvidenceEvent::StepSucceeded { step, output } => Self::push_drive_journal_event(
                journal_events,
                RuntimeJournalEvent::StepSucceeded {
                    run,
                    step,
                    output: match output {
                        Some(slot) => slot,
                        None => SlotIdx::ZERO,
                    },
                    attempt,
                },
            ),
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            } => Self::prepare_slot_written_event(
                run,
                slot,
                value,
                taint,
                extra,
                journal_events,
                trace_events,
            ),
        }
    }

    fn prepare_slot_written_event(
        run: RunId,
        slot: SlotIdx,
        value: vb_core::value::SlotValue,
        taint: vb_core::Taint,
        extra: Option<crate::primitives::collect::CollectPaginationState>,
        journal_events: &mut Vec<RuntimeJournalEvent>,
        trace_events: &mut Vec<TraceEvent>,
    ) -> RuntimeResult<()> {
        let encoded = postcard::to_allocvec(&value).map_err(|_| RuntimeError::EncodeFailed)?;
        let encoded_extra = extra
            .map(|state| postcard::to_allocvec(&state))
            .transpose()
            .map_err(|_| RuntimeError::EncodeFailed)?;
        trace_events.push(TraceEvent::SlotWritten {
            run,
            slot,
            value: encoded.clone(),
        });
        Self::push_drive_journal_event(
            journal_events,
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value: encoded,
                taint,
                extra: encoded_extra,
            },
        )
    }

    pub(crate) fn push_trace_events(&mut self, trace_events: Vec<TraceEvent>) {
        for event in trace_events {
            self.trace_ring.push(event);
        }
    }
}

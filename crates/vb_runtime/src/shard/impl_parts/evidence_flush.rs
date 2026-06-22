impl Shard {
    /// Drains evidence events from the collector and emits them to the
    /// journal and trace ring. This satisfies the Phase 40/44 evidence
    /// chain requirement: StepStarted before SlotWritten for every step,
    /// followed by StepSucceeded.
    ///
    /// RS-205: if any event fails to flush, the unprocessed suffix
    /// (the failed event and every event that would have followed) is
    /// restored into the collector so a subsequent flush attempt can
    /// retry them instead of silently dropping the evidence chain.
    pub(crate) fn flush_evidence(
        &mut self,
        run: RunId,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<()> {
        let events = evidence.drain();
        for (offset, event) in events.iter().enumerate() {
            let result = self.flush_evidence_event(run, *event);
            if let Err(error) = result {
                self.restore_evidence_suffix(evidence, &events, offset);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Restores the unprocessed evidence suffix (failed event + everything
    /// after it) back into the collector so a subsequent flush can retry.
    fn restore_evidence_suffix(
        &self,
        evidence: &mut EvidenceCollector,
        events: &[EvidenceEvent],
        failed_offset: usize,
    ) {
        if let Some(suffix) = events.get(failed_offset..) {
            for event in suffix.iter().copied() {
                evidence.push_event(event);
            }
        }
    }

    fn flush_evidence_event(&mut self, run: RunId, event: EvidenceEvent) -> RuntimeResult<()> {
        match event {
            EvidenceEvent::StepStarted { step } => self.flush_step_started(run, step),
            EvidenceEvent::StepSucceeded {
                step,
                output,
                attempt,
            } => self.flush_step_succeeded(run, step, output, attempt),
            EvidenceEvent::SlotWritten {
                slot,
                value,
                taint,
                extra,
            } => self.flush_slot_written(run, slot, value, taint, extra),
        }
    }

    fn flush_step_started(&mut self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        self.trace_ring.push(TraceEvent::StepStarted { run, step });
        self.append_journal_event(RuntimeJournalEvent::StepStarted { run, step })
    }

    /// Persists a `StepSucceeded` journal event with the live per-step
    /// attempt counter from `state.action_attempts`.
    ///
    /// RS-004: the engine emits `attempt: 1` (deterministic loop has no
    /// engine-level retries); the shard overrides this with the actual
    /// live attempt so the durable record matches `ActionFailed.attempt`
    /// for the same step. For deterministic steps where
    /// `state.action_attempts[step]` is 0, the attempt is clamped to 1
    /// via `.max(1)` so the journal never records `attempt = 0`.
    /// Out-of-bounds step indices map to 0 and clamp to 1 — the step
    /// index is owned by the engine that produced the evidence, so a
    /// mismatch indicates a corrupt collector, which is handled
    /// defensively without panicking.
    fn flush_step_succeeded(
        &mut self,
        run: RunId,
        step: StepIdx,
        output: Option<SlotIdx>,
        engine_attempt: u16,
    ) -> RuntimeResult<()> {
        let live_attempt = self
            .run_state_get(run)
            .and_then(|state| state.action_attempts.get(step.as_usize()).copied())
            .unwrap_or(0);
        let attempt = live_attempt.max(engine_attempt).max(1);
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: match output {
                Some(slot) => slot,
                None => SlotIdx::ZERO,
            },
            attempt,
        })
    }
}

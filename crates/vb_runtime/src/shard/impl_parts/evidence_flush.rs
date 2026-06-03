impl Shard {
    /// Drains evidence events from the collector and emits them to the
    /// journal and trace ring. This satisfies the Phase 40/44 evidence
    /// chain requirement: StepStarted before SlotWritten for every step,
    /// followed by StepSucceeded.
    pub(crate) fn flush_evidence(
        &mut self,
        run: RunId,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<()> {
        evidence
            .drain()
            .into_iter()
            .try_for_each(|event| self.flush_evidence_event(run, event))
    }

    fn flush_evidence_event(&mut self, run: RunId, event: EvidenceEvent) -> RuntimeResult<()> {
        match event {
            EvidenceEvent::StepStarted { step } => self.flush_step_started(run, step),
            EvidenceEvent::StepSucceeded { step, output } => {
                self.flush_step_succeeded(run, step, output)
            }
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

    fn flush_step_succeeded(
        &mut self,
        run: RunId,
        step: StepIdx,
        output: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: match output {
                Some(slot) => slot,
                None => SlotIdx::ZERO,
            },
            attempt: 1,
        })
    }
}

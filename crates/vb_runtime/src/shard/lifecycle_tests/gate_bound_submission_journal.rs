#[derive(Debug, Default)]
struct BoundJournal {
    decisions: Mutex<Vec<BoundGateOutcomeRecord>>,
    opened: Mutex<Vec<(BoundManualGate, u64, ApproverId, u64)>>,
    reject_failing_run: std::sync::atomic::AtomicBool,
    events: Mutex<Vec<RuntimeJournalEvent>>,
}

impl BoundJournal {
    fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn decisions(&self) -> RuntimeResult<Vec<BoundGateOutcomeRecord>> {
        self.decisions
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }

    fn opened(&self) -> RuntimeResult<Vec<(BoundManualGate, u64, ApproverId, u64)>> {
        self.opened
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }

    fn events(&self) -> RuntimeResult<Vec<RuntimeJournalEvent>> {
        self.events
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }

    fn run_failed_count(&self, run: RunId) -> RuntimeResult<usize> {
        Ok(self
            .events()?
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::RunFailed { run: r, .. } if *r == run
                )
            })
            .count())
    }
}

impl RuntimeJournal for BoundJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        for event in events {
            self.append(event.clone())?;
        }
        Ok(())
    }

    fn append_gate_decision_strict(
        &self,
        _record: vb_core::manual_gate::GateOutcomeRecord,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_gate_opened_strict(
        &self,
        _gate: ManualGate,
        _opened_at_ms: u64,
        _requester: ApproverId,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_gate_decision_bound_strict(
        &self,
        record: BoundGateOutcomeRecord,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        self.decisions
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(record);
        Ok(())
    }

    fn append_gate_decision_bound_failing_run_strict(
        &self,
        record: BoundGateOutcomeRecord,
        _seq: vb_storage::EventSeq,
        run_failed: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        if self.reject_failing_run.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "injected_failing_run_batch",
            });
        }
        self.decisions
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(record);
        self.append(run_failed)
    }

    fn append_gate_opened_bound_strict(
        &self,
        gate: BoundManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
        seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        self.opened
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push((gate, opened_at_ms, requester, seq.get()));
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}

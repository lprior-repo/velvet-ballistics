use std::sync::Arc;
use vb_core::manual_gate::{ApproverId, BoundGateOutcomeRecord, BoundManualGate};
use vb_runtime::RuntimeResult;
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

pub(super) struct PolicyTamperJournal {
    pub(super) inner: SharedRuntimeJournal,
    pub(super) mismatch: &'static str,
}

impl RuntimeJournal for PolicyTamperJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        self.inner.durability_profile()
    }
    fn executor_catalog(&self) -> Option<Arc<vb_storage::executor::ExecutorCatalog>> {
        self.inner.executor_catalog()
    }
    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.inner.storage_journal()
    }
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        self.inner.append(event)
    }
    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        self.inner.append_sequenced(event, seq)
    }
    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_sequenced_batch(events, seq)
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<JournalEvent> {
        self.inner.convert_to_storage_event(event, seq)
    }
    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_policy_decision_strict(decision, seq)
    }
    fn append_gate_decision_bound_strict(
        &self,
        mut record: BoundGateOutcomeRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let approval = record.bound_receipt.clone();
        match self.mismatch {
            "evidence" => record.record.evidence_shown = false,
            "reason" => record.record.reason = None,
            _ => {}
        }
        assert_eq!(record.bound_receipt, approval);
        self.inner.append_gate_decision_bound_strict(record, seq)
    }
    fn append_gate_opened_bound_strict(
        &self,
        mut gate: BoundManualGate,
        time: u64,
        requester: ApproverId,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        if self.mismatch == "timeout" {
            gate.gate.timeout_ms = 50;
        }
        self.inner
            .append_gate_opened_bound_strict(gate, time, requester, seq)
    }
    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }
}

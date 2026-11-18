//! Public-adapter fault injection over an accepted, captured handler prefix.
//! This is not a native Fjall failure, allocator failure, or proof that the
//! sliced prefix is a current atomic-batch crash boundary. No private runtime
//! state is changed; the only fault is the public journal conversion method.

use super::support::*;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use vb_core::{ActionFailureCode, SlotValue, StepState, Taint};
use vb_runtime::journal::{
    RuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal, StorageRuntimeJournal,
};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::AskAnswer;
use vb_runtime::{RuntimeError, RuntimeResult};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, RunInputsRecord};

const FAILURE: &str = "test_ask_schedule_conversion_refused";

struct RefuseAskConversion {
    inner: SharedRuntimeJournal,
    armed: AtomicBool,
    interceptions: AtomicUsize,
}

impl RuntimeJournal for RefuseAskConversion {
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
    fn append_admission_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
        inputs: RunInputsRecord,
    ) -> RuntimeResult<()> {
        self.inner.append_admission_batch(events, seq, inputs)
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<JournalEvent> {
        if matches!(&event, RuntimeJournalEvent::AskScheduled { run, .. } if *run == RUN)
            && self.armed.load(Ordering::SeqCst)
        {
            self.interceptions.fetch_add(1, Ordering::SeqCst);
            return Err(RuntimeError::UnsupportedOperation { operation: FAILURE });
        }
        self.inner.convert_to_storage_event(event, seq)
    }
    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }
    fn drain_for_shutdown(&self) -> RuntimeResult<vb_storage::JournalWriterFlushReport> {
        self.inner.drain_for_shutdown()
    }
    fn pending_high_water_for_run(&self, run: vb_core::RunId) -> RuntimeResult<Option<EventSeq>> {
        self.inner.pending_high_water_for_run(run)
    }
    fn flush_run_prefix(&self, run: vb_core::RunId) -> RuntimeResult<()> {
        self.inner.flush_run_prefix(run)
    }
    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_policy_decision_strict(decision, seq)
    }
    fn append_gate_decision_strict(
        &self,
        record: vb_core::manual_gate::GateOutcomeRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_gate_decision_strict(record, seq)
    }
    fn append_gate_opened_strict(
        &self,
        gate: vb_core::manual_gate::ManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner
            .append_gate_opened_strict(gate, opened_at_ms, requester, seq)
    }
    fn append_gate_decision_bound_strict(
        &self,
        record: vb_core::manual_gate::BoundGateOutcomeRecord,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_gate_decision_bound_strict(record, seq)
    }
    fn append_gate_opened_bound_strict(
        &self,
        gate: vb_core::manual_gate::BoundManualGate,
        opened_at_ms: u64,
        requester: vb_core::manual_gate::ApproverId,
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner
            .append_gate_opened_bound_strict(gate, opened_at_ms, requester, seq)
    }
}

fn captured_handler_prefix(
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult<(Vec<JournalEvent>, RunInputsRecord)> {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    submit(&journal, &mut runtime, workflow, &[contract()?])?;
    let work = action_step(workflow)?;
    let ask = ask_step(workflow)?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("action ticket")?;
    runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    pending_ask(&mut runtime, ask)?;
    let inputs = journal.run_inputs(RUN)?.ok_or("retained inputs")?;
    let events = journal.events_for_run_full(RUN)?;
    let failed = events
        .iter()
        .position(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
        .ok_or("action failure")?;
    let started = events
        .iter()
        .enumerate()
        .skip(failed)
        .find_map(|(index, event)| {
            matches!(event, JournalEvent::StepStarted { step, .. } if *step == ask).then_some(index)
        })
        .ok_or("started handler")?;
    Ok((
        events.get(..=started).ok_or("captured prefix")?.to_vec(),
        inputs,
    ))
}

#[test]
fn recovered_handler_ask_conversion_failure_retains_state_and_reopens() -> TestResult {
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let (prefix, inputs) = captured_handler_prefix(&workflow)?;
    let (_directory, journal, initial_runtime) =
        scratch_store(&workflow, &prefix, &[contract()?], &inputs)?;
    drop(initial_runtime);
    let adapter = Arc::new(RefuseAskConversion {
        inner: StorageRuntimeJournal::shared_journaled(Arc::clone(&journal)),
        armed: AtomicBool::new(true),
        interceptions: AtomicUsize::new(0),
    });
    let mut runtime = Runtime::new(NonZeroUsize::MIN, config(), adapter.clone())?;
    let before = journal.events_for_run_full(RUN)?;
    assert_eq!(before, prefix);
    runtime.recover_and_resume(RUN)?;
    assert_eq!(
        runtime.tick_all(),
        Err(RuntimeError::UnsupportedOperation { operation: FAILURE })
    );
    assert_eq!(adapter.interceptions.load(Ordering::SeqCst), 1);
    let state = runtime.inspect_run_state(RUN)?;
    assert_eq!(state.frame.step_state(work)?, StepState::Failed);
    assert_eq!(state.frame.step_state(ask)?, StepState::Asking);
    assert_eq!(state.frame.pc(), ask);
    assert_eq!(state.frame.executed(), 1);
    assert!(runtime.lookup_pending_ask_ticket(RUN, ask).is_none());
    assert!(
        runtime
            .lookup_pending_action_ticket(RUN, u64::from(work.get()))
            .is_none()
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(16).pending_timer_count(),
        0
    );
    assert_eq!(journal.events_for_run_full(RUN)?, before);

    // Retry via a fresh public Runtime, not private checkout repair or an
    // assertion that resume_run can retry this same live Runtime state.
    drop(runtime);
    adapter.armed.store(false, Ordering::SeqCst);
    let mut reopened = Runtime::new(NonZeroUsize::MIN, config(), adapter.clone())?;
    reopened.recover_and_resume(RUN)?;
    reopened.tick_all()?;
    let ticket = pending_ask(&mut reopened, ask)?;
    assert_eq!(adapter.interceptions.load(Ordering::SeqCst), 1);
    reopened.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(&mut reopened)?;
    assert_eq!(reopened.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::StepStarted { step, .. } if *step == ask))
            .count(),
        1
    );
    let tail = events.get(prefix.len()..).ok_or("retry journal tail")?;
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1
    );
    assert!(
        !tail
            .iter()
            .any(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
    );
    Ok(())
}

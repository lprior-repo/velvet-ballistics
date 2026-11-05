#![forbid(unsafe_code)]
//! Live header-refusal cleanup with a volatile journal, not durable retry safety.

use std::error::Error;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_runtime::{RuntimeError, RuntimeResult, TraceEvent};
use vb_storage::{EventSeq, JournalError, JournalWriterFlushReport};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Header {
    Submitted,
    Admission,
}

impl Header {
    fn of(event: &RuntimeJournalEvent) -> Option<Self> {
        match event {
            RuntimeJournalEvent::RunSubmitted { .. } => Some(Self::Submitted),
            RuntimeJournalEvent::RunAdmission { .. } => Some(Self::Admission),
            _ => None,
        }
    }
}

struct FailHeaderOnce {
    inner: VolatileRuntimeJournal,
    target: Header,
    rejected_run: RunId,
    later_run: RunId,
    source: Arc<JournalError>,
    failed: AtomicBool,
    refusals: AtomicUsize,
    rejected_sequence: AtomicU64,
    later_submitted_sequence: AtomicU64,
    later_admission_sequence: AtomicU64,
    successful_multi_batches: AtomicUsize,
}

impl FailHeaderOnce {
    fn new(target: Header, rejected_run: RunId, later_run: RunId) -> Self {
        Self {
            inner: VolatileRuntimeJournal::new(),
            target,
            rejected_run,
            later_run,
            source: Arc::new(JournalError::WriteLockPoisoned),
            failed: AtomicBool::new(false),
            refusals: AtomicUsize::new(0),
            rejected_sequence: AtomicU64::new(u64::MAX),
            later_submitted_sequence: AtomicU64::new(u64::MAX),
            later_admission_sequence: AtomicU64::new(u64::MAX),
            successful_multi_batches: AtomicUsize::new(0),
        }
    }

    fn refuse_target_once(&self, event: &RuntimeJournalEvent) -> RuntimeResult<()> {
        if event.run_id() == self.rejected_run
            && Header::of(event) == Some(self.target)
            && !self.failed.swap(true, Ordering::SeqCst)
        {
            self.refusals.fetch_add(1, Ordering::SeqCst);
            return Err(RuntimeError::StorageJournalAppend {
                source: self.source.clone(),
            });
        }
        Ok(())
    }
}

impl RuntimeJournal for FailHeaderOnce {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        self.refuse_target_once(&event)?;
        self.inner.append(event)
    }

    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        let header = Header::of(&event);
        let later = event.run_id() == self.later_run;
        if event.run_id() == self.rejected_run && header == Some(self.target) {
            self.rejected_sequence.store(seq.get(), Ordering::SeqCst);
        }
        self.refuse_target_once(&event)?;
        self.inner.append_sequenced(event, seq)?;
        if later {
            match header {
                Some(Header::Submitted) => {
                    self.later_submitted_sequence
                        .store(seq.get(), Ordering::SeqCst);
                }
                Some(Header::Admission) => {
                    self.later_admission_sequence
                        .store(seq.get(), Ordering::SeqCst);
                }
                None => {}
            }
        }
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        // Refuse before forwarding any event; preserve the inner atomic batch.
        for event in events {
            self.refuse_target_once(event)?;
        }
        self.inner.append_sequenced_batch(events, start_seq)?;
        if events.len() > 1 {
            self.successful_multi_batches.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.inner.drain_for_shutdown()
    }
}

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut secret = CompiledInputConstraint::required();
    secret.secret = true;
    let mut optional_secret = secret;
    optional_secret.optional = true;
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        name: "submission-header-frame-reuse".into(),
        digest: WorkflowDigest::from_bytes([0x7c; 32]),
        nodes: Box::new([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: None,
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(5),
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(5),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 6,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::ZERO,
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::required(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
                constraint: secret,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(3),
                kind: InputSlotKind::I64,
                constraint: optional_secret,
            },
        ]),
    })?)
}

fn assert_header_refusal_reclaims_frame(target: Header) -> TestResult {
    let rejected_run = RunId::new(8301);
    let later_run = RunId::new(8302);
    let journal = Arc::new(FailHeaderOnce::new(target, rejected_run, later_run));
    let config = ShardConfig::new(4, 32, 16, 1, RuntimePolicy::Relaxed)?;
    let shards = NonZeroUsize::new(1).ok_or("nonzero shard count")?;
    let mut runtime = Runtime::new(shards, config, journal.clone());
    let workflow = workflow()?;
    let digest = workflow.digest();
    runtime.submit_compiled_with_inputs(
        rejected_run,
        workflow.clone(),
        Box::new([
            (SlotIdx::ZERO, SlotValue::I64(11)),
            (SlotIdx::new(2), SlotValue::I64(199)),
            (SlotIdx::new(3), SlotValue::I64(99)),
            (SlotIdx::new(4), SlotValue::I64(222)),
            (SlotIdx::new(1), SlotValue::I64(7)),
        ]),
    )?;
    match runtime.tick_all() {
        Err(RuntimeError::AdmissionHeaderPersistenceFailed { source }) => {
            assert!(Arc::ptr_eq(&source, &journal.source));
        }
        other => return Err(format!("expected the original header failure, got {other:?}").into()),
    }
    assert_eq!(journal.refusals.load(Ordering::SeqCst), 1);
    let expected_sequence = match target {
        Header::Submitted => 0,
        Header::Admission => 1,
    };
    assert_eq!(
        journal.rejected_sequence.load(Ordering::SeqCst),
        expected_sequence
    );
    let prefix = journal.inner.snapshot()?;
    let expected_prefix = match target {
        Header::Submitted => vec![],
        Header::Admission => vec![RuntimeJournalEvent::RunSubmitted {
            run: rejected_run,
            workflow: digest,
        }],
    };
    assert_eq!(prefix, expected_prefix);
    assert!(matches!(
        runtime.inspect_run_state(rejected_run),
        Err(RuntimeError::RunNotFound)
    ));
    assert!(
        runtime
            .lookup_pending_ask_ticket(rejected_run, StepIdx::ZERO)
            .is_none()
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert_eq!(
        runtime.drain_trace(),
        vec![TraceEvent::RunSubmitted { run: rejected_run }]
    );

    // A distinct run checks pool capacity; this does not retry the partial prefix.
    runtime.submit_compiled_with_inputs(
        later_run,
        workflow,
        Box::new([
            (SlotIdx::new(2), SlotValue::I64(299)),
            (SlotIdx::new(1), SlotValue::I64(8)),
            (SlotIdx::ZERO, SlotValue::I64(33)),
        ]),
    )?;
    assert!(runtime.tick_all()?);
    let ticket = runtime
        .lookup_pending_ask_ticket(later_run, StepIdx::ZERO)
        .ok_or("later submission must reach the untimed Ask")?;
    assert_eq!(ticket.run(), later_run);
    assert_eq!(ticket.ask_step(), StepIdx::ZERO);
    let frame = &runtime.inspect_run_state(later_run)?.frame;
    assert_eq!(frame.run_id(), later_run);
    assert_eq!(
        frame.slots_snapshot(),
        vec![
            Some(SlotValue::I64(33)),
            Some(SlotValue::I64(8)),
            Some(SlotValue::I64(299)),
            None,
            None,
            None,
        ]
    );
    assert_eq!(
        frame.taint_snapshot(),
        vec![
            Taint::Clean,
            Taint::Clean,
            Taint::Secret,
            Taint::Clean,
            Taint::Clean,
            Taint::Clean
        ]
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    assert_eq!(journal.refusals.load(Ordering::SeqCst), 1);
    assert_eq!(journal.later_submitted_sequence.load(Ordering::SeqCst), 0);
    assert_eq!(journal.later_admission_sequence.load(Ordering::SeqCst), 1);
    assert!(journal.successful_multi_batches.load(Ordering::SeqCst) > 0);
    let history = journal.inner.snapshot()?;
    assert_eq!(
        history.get(..prefix.len()).ok_or("prefix missing")?,
        prefix.as_slice()
    );
    assert_eq!(
        history
            .iter()
            .filter(|event| event.run_id() == rejected_run)
            .cloned()
            .collect::<Vec<_>>(),
        prefix
    );
    Ok(())
}

#[test]
fn submitted_header_failure_reclaims_frame_for_a_distinct_run() -> TestResult {
    assert_header_refusal_reclaims_frame(Header::Submitted)
}

#[test]
fn admission_header_failure_preserves_prefix_and_reclaims_frame_for_a_distinct_run() -> TestResult {
    assert_header_refusal_reclaims_frame(Header::Admission)
}

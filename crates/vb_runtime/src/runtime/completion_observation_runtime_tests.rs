use super::*;
use crate::RuntimeError;
use crate::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use crate::runtime::Runtime;
use crate::shard::{ShardCommand, ShardConfig};
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};

type TestResult<T = ()> = Result<T, String>;

fn checked<T, E: core::fmt::Debug>(result: Result<T, E>) -> TestResult<T> {
    result.map_err(|error| format!("fixture or operation failed: {error:?}"))
}

fn finish_workflow() -> TestResult<CompiledWorkflow> {
    checked(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("observed_tick_phases"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(99)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    }))
}

fn config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 32,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::RuntimePolicy::Relaxed,
    }
}

fn runtime() -> TestResult<(Runtime, Arc<VolatileRuntimeJournal>)> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let runtime = Runtime::new(
        NonZeroUsize::new(3).ok_or("invalid shard count")?,
        config(),
        journal.clone(),
    );
    Ok((runtime, journal))
}

fn submit(runtime: &Runtime, run: u64) -> TestResult {
    checked(runtime.submit_direct(RunId::new(run), finish_workflow()?))
}

fn receipt(run: u64) -> CompletionObservationSlot {
    CompletionObservationSlot {
        first: Some(CompletionObservation {
            run: RunId::new(run),
            attempt: NonZeroU16::MIN,
            result: CompletionValueObservation::Available(OwnedCompletionScalar::I64(99)),
        }),
        conflict: false,
    }
}

fn conflict_only() -> CompletionObservationSlot {
    CompletionObservationSlot {
        first: None,
        conflict: true,
    }
}

fn assert_queues(runtime: &Runtime, expected: [usize; 3]) {
    let actual: Vec<_> = runtime
        .shards
        .iter()
        .map(|shard| shard.command_queue.len())
        .collect();
    assert_eq!(actual, expected);
}

fn assert_capture_reset(runtime: &Runtime) {
    for shard in &runtime.shards {
        assert!(!shard.completion_capture.enabled);
        assert_eq!(
            shard.completion_capture.slot,
            CompletionObservationSlot::default(),
        );
    }
}

fn finished_runs(journal: &VolatileRuntimeJournal) -> TestResult<Vec<RunId>> {
    Ok(checked(journal.snapshot())?
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::RunFinished {
                run,
                result,
                attempt,
            } => {
                assert_eq!(*result, SlotIdx::ZERO);
                assert_eq!(*attempt, 1);
                Some(*run)
            }
            _ => None,
        })
        .collect())
}

#[test]
fn short_buffer_wins_over_occupied_slot_without_ticking() -> TestResult {
    let (mut runtime, journal) = runtime()?;
    for run in [3, 4, 5] {
        submit(&runtime, run)?;
    }
    assert_queues(&runtime, [1, 1, 1]);
    let mut short = [receipt(901), conflict_only()];
    let original = short;
    assert!(matches!(
        runtime.tick_all_observed(&mut short),
        Err(ObservationPreflightError::BufferTooShort {
            required: 3,
            provided: 2,
        })
    ));
    assert_eq!(short, original);
    assert_queues(&runtime, [1, 1, 1]);
    assert!(checked(journal.snapshot())?.is_empty());
    assert_capture_reset(&runtime);

    // Occupied tail slots are caller data, not completion evidence for this tick.
    let tail = [receipt(902), conflict_only()];
    let mut slots = [CompletionObservationSlot::default(); 5];
    slots[3..].copy_from_slice(&tail);
    let outcome = checked(runtime.tick_all_observed(&mut slots))?;
    assert_eq!(outcome.execution, Ok(true));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots[..3], [receipt(3), receipt(4), receipt(5)]);
    assert_eq!(slots[3..], tail);
    assert_eq!(
        finished_runs(&journal)?,
        [RunId::new(3), RunId::new(4), RunId::new(5)],
    );
    assert_queues(&runtime, [0, 0, 0]);
    assert_capture_reset(&runtime);
    Ok(())
}

#[test]
fn lowest_occupied_required_slot_rejects_before_the_first_shard() -> TestResult {
    let (mut runtime, journal) = runtime()?;
    for run in [3, 4, 5] {
        submit(&runtime, run)?;
    }
    let mut slots = [
        CompletionObservationSlot::default(),
        conflict_only(),
        receipt(903),
    ];
    let original = slots;
    assert!(matches!(
        runtime.tick_all_observed(&mut slots),
        Err(ObservationPreflightError::SlotNotEmpty { index: 1 })
    ));
    assert_eq!(slots, original);
    assert_queues(&runtime, [1, 1, 1]);
    assert!(checked(journal.snapshot())?.is_empty());
    assert_capture_reset(&runtime);

    slots[1] = CompletionObservationSlot::default();
    let original = slots;
    assert!(matches!(
        runtime.tick_all_observed(&mut slots),
        Err(ObservationPreflightError::SlotNotEmpty { index: 2 })
    ));
    assert_eq!(slots, original);
    assert_queues(&runtime, [1, 1, 1]);
    assert!(checked(journal.snapshot())?.is_empty());
    assert_capture_reset(&runtime);

    slots[2] = CompletionObservationSlot::default();
    let outcome = checked(runtime.tick_all_observed(&mut slots))?;
    assert_eq!(outcome.execution, Ok(true));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots, [receipt(3), receipt(4), receipt(5)]);
    assert_queues(&runtime, [0, 0, 0]);
    assert_capture_reset(&runtime);
    Ok(())
}

#[test]
fn execution_error_keeps_earlier_receipt_and_leaves_later_shard_pending() -> TestResult {
    let (mut runtime, journal) = runtime()?;
    submit(&runtime, 3)?;
    checked(runtime.resume_run(RunId::new(4)))?;
    submit(&runtime, 5)?;
    assert_queues(&runtime, [1, 1, 1]);
    let tail = receipt(904);
    let mut slots = [CompletionObservationSlot::default(); 4];
    slots[3] = tail;
    let outcome = checked(runtime.tick_all_observed(&mut slots))?;
    assert_eq!(outcome.execution, Err(RuntimeError::RunNotFound));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots[0], receipt(3));
    assert_eq!(slots[1], CompletionObservationSlot::default());
    assert_eq!(slots[2], CompletionObservationSlot::default());
    assert_eq!(slots[3], tail);
    assert_queues(&runtime, [0, 0, 1]);
    assert_eq!(finished_runs(&journal)?, [RunId::new(3)]);
    // Inspect before another tick could reset either visited shard's capture.
    assert_capture_reset(&runtime);

    slots[..3].fill(CompletionObservationSlot::default());
    let outcome = checked(runtime.tick_all_observed(&mut slots))?;
    assert_eq!(outcome.execution, Ok(true));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots[0], CompletionObservationSlot::default());
    assert_eq!(slots[1], CompletionObservationSlot::default());
    assert_eq!(slots[2], receipt(5));
    assert_eq!(slots[3], tail);
    assert_eq!(finished_runs(&journal)?, [RunId::new(3), RunId::new(5)]);
    assert_queues(&runtime, [0, 0, 0]);
    assert_capture_reset(&runtime);
    Ok(())
}

#[test]
fn shutdown_false_does_not_skip_later_shard_completions() -> TestResult {
    let (mut runtime, journal) = runtime()?;
    checked(
        runtime
            .shards
            .first()
            .ok_or("missing first shard")?
            .enqueue(ShardCommand::Shutdown),
    )?;
    submit(&runtime, 4)?;
    submit(&runtime, 5)?;
    let mut slots = [CompletionObservationSlot::default(); 3];
    let outcome = checked(runtime.tick_all_observed(&mut slots))?;
    assert_eq!(outcome.execution, Ok(false));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(
        slots,
        [CompletionObservationSlot::default(), receipt(4), receipt(5)],
    );
    assert_eq!(finished_runs(&journal)?, [RunId::new(4), RunId::new(5)]);
    assert_queues(&runtime, [0, 0, 0]);
    assert_capture_reset(&runtime);
    Ok(())
}

#[cfg(panic = "unwind")]
#[test]
fn injected_journal_unwind_resets_capture_before_any_following_tick() -> TestResult {
    use crate::RuntimeResult;
    use crate::journal::RuntimeJournal;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Typed identity keeps unrelated unwinds from satisfying this cleanup test.
    struct TypedTestFault;

    struct PanickingJournal {
        inner: VolatileRuntimeJournal,
        triggered: AtomicBool,
    }

    impl RuntimeJournal for PanickingJournal {
        fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
            self.inner.append(event)
        }

        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            start_seq: vb_storage::EventSeq,
        ) -> RuntimeResult<()> {
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::RunFinished { .. }))
            {
                self.triggered.store(true, Ordering::SeqCst);
                // Intentional test-only unwind exercises pre-capture cleanup.
                std::panic::resume_unwind(Box::new(TypedTestFault));
            }
            self.inner.append_sequenced_batch(events, start_seq)
        }

        fn probe(&self) -> RuntimeResult<()> {
            self.inner.probe()
        }
    }

    let journal = Arc::new(PanickingJournal {
        inner: VolatileRuntimeJournal::new(),
        triggered: AtomicBool::new(false),
    });
    let mut runtime = Runtime::new(
        NonZeroUsize::new(3).ok_or("invalid shard count")?,
        config(),
        journal.clone(),
    );
    for run in [3, 4, 5] {
        submit(&runtime, run)?;
    }
    let mut slots = [CompletionObservationSlot::default(); 4];
    slots[3] = receipt(905);
    let original = slots;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.tick_all_observed(&mut slots)
    }));
    assert!(result.is_err());
    let payload = result.err().ok_or("expected injected journal unwind")?;
    assert!(payload.is::<TypedTestFault>());
    assert!(journal.triggered.load(Ordering::SeqCst));
    assert_capture_reset(&runtime);
    assert_eq!(slots, original);
    assert_queues(&runtime, [0, 1, 1]);
    assert!(finished_runs(&journal.inner)?.is_empty());
    // This is a pre-capture provider fault, not a receipt or rollback guarantee.
    Ok(())
}

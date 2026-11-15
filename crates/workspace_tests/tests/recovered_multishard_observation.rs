#![forbid(unsafe_code)]

use std::{
    num::{NonZeroU16, NonZeroUsize},
    sync::Arc,
};
use vb_core::{RunId, RuntimePolicy};
use vb_runtime::{
    RuntimeError,
    journal::StorageRuntimeJournal,
    runtime::{
        Runtime,
        completion_observation::{
            CompletionObservation, CompletionObservationSlot as Slot,
            CompletionValueObservation as Value, ObservationPreflightError,
            OwnedCompletionScalar as Scalar,
        },
    },
    shard::{ShardConfig, ShardDirective},
};
use vb_storage::{FjallJournal, JournalEvent};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _directory: tempfile::TempDir,
    journal: Arc<FjallJournal>,
    runtime: Runtime,
    workflow: vb_core::CompiledWorkflow,
}
impl Fixture {
    fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: observed_shards\nwhen: {manual: {}}\nsteps:\n  - id: answer\n    set: {output: answer, value: '99'}\n  - id: done\n    finish: {result: answer}\n")?;
        vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
        let runtime = Runtime::new(
            NonZeroUsize::new(3).ok_or("invalid shard count")?,
            ShardConfig {
                command_queue_capacity: 8,
                trace_capacity: 32,
                step_budget_per_tick: 4,
                max_active_runs: 4,
                max_open_gates: 4,
                policy: RuntimePolicy::Strict,
            },
            StorageRuntimeJournal::shared_journaled(journal.clone()),
        )?;
        Ok(Self {
            _directory: directory,
            journal,
            runtime,
            workflow,
        })
    }
    fn submit(&self, run: u64) -> TestResult {
        Ok(self
            .runtime
            .submit_direct(RunId::new(run), self.workflow.clone())?)
    }
    fn queues(&self) -> Vec<u32> {
        self.runtime
            .collect_metrics()
            .shards
            .iter()
            .map(|shard| shard.command_queue_depth)
            .collect()
    }
    fn events(&self, run: u64) -> TestResult<Vec<JournalEvent>> {
        Ok(self.journal.events_for_run(RunId::new(run))?)
    }
}
fn receipt(run: u64) -> Slot {
    Slot {
        first: Some(CompletionObservation {
            run: RunId::new(run),
            attempt: NonZeroU16::MIN,
            result: Value::Available(Scalar::I64(99)),
        }),
        conflict: false,
    }
}

#[test]
fn preflight_preserves_all_shards_and_reports_lowest_occupied_slot() -> TestResult {
    let mut fixture = Fixture::new()?;
    for run in [3, 4, 5] {
        fixture.submit(run)?;
    }
    assert_eq!(fixture.queues(), [1, 1, 1]);
    let counters = fixture.runtime.counters_snapshot();
    let mut short = [
        receipt(901),
        Slot {
            first: None,
            conflict: true,
        },
    ];
    let original = short;
    assert!(matches!(
        fixture.runtime.tick_all_observed(&mut short),
        Err(ObservationPreflightError::BufferTooShort {
            required: 3,
            provided: 2
        })
    ));
    assert_eq!(short, original);
    for occupied in [1, 2] {
        let mut slots = [
            Slot::default(),
            Slot {
                first: None,
                conflict: true,
            },
            receipt(903),
        ];
        if occupied == 2 {
            slots.get_mut(1).ok_or("missing slot")?.conflict = false;
        }
        let original = slots;
        assert!(
            matches!(fixture.runtime.tick_all_observed(&mut slots), Err(ObservationPreflightError::SlotNotEmpty { index }) if index == occupied)
        );
        assert_eq!(slots, original);
        assert_eq!(fixture.queues(), [1, 1, 1]);
        assert_eq!(fixture.runtime.counters_snapshot(), counters);
        for run in [3, 4, 5] {
            assert!(fixture.events(run)?.is_empty());
        }
    }
    let mut slots = [
        Slot::default(),
        Slot::default(),
        Slot::default(),
        receipt(902),
        Slot {
            first: None,
            conflict: true,
        },
    ];
    let outcome = fixture
        .runtime
        .tick_all_observed(&mut slots)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.execution, Ok(true));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(
        slots,
        [
            receipt(3),
            receipt(4),
            receipt(5),
            receipt(902),
            Slot {
                first: None,
                conflict: true
            }
        ]
    );
    assert_eq!(fixture.queues(), [0, 0, 0]);
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 3);
    Ok(())
}

#[test]
fn execution_error_retains_earlier_receipt_and_later_pending_work() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.submit(3)?;
    fixture.runtime.resume_run(RunId::new(4))?;
    fixture.submit(5)?;
    let tail = receipt(904);
    let mut slots = [Slot::default(), Slot::default(), Slot::default(), tail];
    let outcome = fixture
        .runtime
        .tick_all_observed(&mut slots)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.execution, Err(RuntimeError::RunNotFound));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots, [receipt(3), Slot::default(), Slot::default(), tail]);
    assert_eq!(fixture.queues(), [0, 0, 1]);
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 1);
    assert!(
        fixture
            .events(3)?
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { .. }))
    );
    assert!(fixture.events(4)?.is_empty());
    assert!(fixture.events(5)?.is_empty());
    slots
        .get_mut(..3)
        .ok_or("missing receipt prefix")?
        .fill(Slot::default());
    let outcome = fixture
        .runtime
        .tick_all_observed(&mut slots)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.execution, Ok(true));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots, [Slot::default(), Slot::default(), receipt(5), tail]);
    assert_eq!(fixture.queues(), [0, 0, 0]);
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 2);
    Ok(())
}

#[test]
fn first_shard_shutdown_still_processes_later_shard_completions() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert!(!fixture.runtime.tick_shard(0, ShardDirective::Shutdown)?);
    fixture.submit(4)?;
    fixture.submit(5)?;
    let mut slots = [Slot::default(); 3];
    let outcome = fixture
        .runtime
        .tick_all_observed(&mut slots)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.execution, Ok(false));
    assert_eq!(outcome.observation, Ok(()));
    assert_eq!(slots, [Slot::default(), receipt(4), receipt(5)]);
    assert_eq!(fixture.queues(), [0, 0, 0]);
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 2);
    Ok(())
}

use super::{SlotValue, TestResult};
use std::{num::NonZeroUsize, sync::Arc, time::Instant};
use vb_core::{RunId, RuntimePolicy, Taint, ValueStore, WorkflowDigest};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{FjallJournal, JournalEvent, RunSubmission};
pub(super) fn execute_owned(source: &str, quantum: u64) -> TestResult<SlotValue> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let slot = workflow.input_slots().first().ok_or("list input")?.slot;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list(Box::from([SlotValue::I64(3), SlotValue::I64(5)]))?;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(slot, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(*blake3::hash(source.as_bytes()).as_bytes()),
        source: source.as_bytes().to_vec(),
    };
    let run = RunId::new(1);
    let record = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &bundle,
        &workflow,
    )?;
    let submission = RunSubmission::owned(
        source.clone(),
        artifact,
        record,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            step_budget_per_tick: quantum,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.activate_owned_submission(run, source.digest)?;
    let mut waits = 0;
    for _ in 0..512 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if let Ok(timer) = runtime.capture_timer_entry(run) {
            std::thread::sleep(timer.deadline.saturating_duration_since(Instant::now()));
            runtime.timer_entry_fired(timer)?;
            waits += 1;
        }
    }
    assert_eq!(waits, 2);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(run)?;
    let results = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1);
    let result = *results.first().ok_or("finish")?;
    let value = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, value, .. } if *slot == result => Some(value),
            _ => None,
        })
        .ok_or("result value")?;
    Ok(postcard::from_bytes(
        value.as_deref().ok_or("missing result payload")?,
    )?)
}

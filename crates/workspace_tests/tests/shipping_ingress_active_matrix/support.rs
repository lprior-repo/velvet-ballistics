#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::journal::{SharedRuntimeJournal, StorageRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::admission::AcceptedArtifact;
use vb_storage::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};
use vb_storage::{FjallJournal, JournalEvent};

#[path = "../shipping_ingress_matrix/support/ordering.rs"]
mod ordering;
pub(super) use ordering::assert_ordering;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Durability {
    Strict,
}

pub(super) struct OpenRuntime {
    pub(super) journal: Arc<FjallJournal>,
    pub(super) runtime: Runtime,
}

pub(super) fn compile(yaml: &str) -> TestResult<CompiledWorkflow> {
    vb_compile::compile_workflow(yaml.as_bytes()).map_err(|error| Box::new(error) as _)
}

pub(super) fn open_runtime(
    path: &Path,
    policy: RuntimePolicy,
    durability: Durability,
) -> TestResult<OpenRuntime> {
    open_runtime_with_config(
        path,
        ShardConfig {
            command_queue_capacity: 32,
            trace_capacity: 64,
            step_budget_per_tick: 16,
            max_active_runs: 8,
            policy,
        },
        durability,
    )
}

pub(super) fn open_runtime_with_config(
    path: &Path,
    config: ShardConfig,
    _durability: Durability,
) -> TestResult<OpenRuntime> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let shared: SharedRuntimeJournal = StorageRuntimeJournal::shared_strict(Arc::clone(&journal));
    let runtime = Runtime::new(std::num::NonZeroUsize::MIN, config, shared);
    Ok(OpenRuntime { journal, runtime })
}

pub(super) fn persist_artifact(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[vb_core::action::ActionContract],
) -> TestResult<AcceptedArtifact> {
    Ok(vb_storage::admission::submit_artifact_with_contracts(
        journal, workflow, policy, contracts,
    )?)
}

pub(super) fn events(journal: &FjallJournal, run: RunId) -> TestResult<Vec<JournalEvent>> {
    Ok(journal.events_for_run_full(run)?)
}

pub(super) fn tick_until_terminal(
    runtime: &mut Runtime,
    run: RunId,
    max_ticks: usize,
) -> TestResult {
    let before = runtime.counters_snapshot();
    for _ in 0..max_ticks {
        runtime.tick_all()?;
        let now = runtime.counters_snapshot();
        let terminal = now
            .runs_completed
            .saturating_add(now.runs_failed)
            .saturating_sub(before.runs_completed)
            .saturating_sub(before.runs_failed);
        if terminal > 0 {
            return Ok(());
        }
        if runtime.list_active_runs(64, None).is_empty()
            && runtime.snapshot_run(run, 0).is_ok_and(|response| {
                matches!(
                    response,
                    vb_runtime::shard::InspectResponse::NotFound { .. }
                )
            })
        {
            return Ok(());
        }
    }
    Err(format!("run {run:?} did not reach a terminal state").into())
}

pub(super) struct Terminal {
    pub(super) value: SlotValue,
    pub(super) taint: Taint,
}

pub(super) fn terminal(events: &[JournalEvent], run: RunId) -> TestResult<Terminal> {
    let slot = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished {
                run: actual,
                result,
                ..
            } if *actual == run => Some(*result),
            _ => None,
        })
        .ok_or_else(|| format!("run {run:?} has no RunFinished"))?;
    if let Some((value, extra)) = events.iter().rev().find_map(|event| match event {
        JournalEvent::SlotWrittenEvent {
            run: actual,
            slot: written,
            value,
            extra,
            ..
        } if *actual == run && *written == slot => Some((value, extra)),
        _ => None,
    }) {
        let value = postcard::from_bytes::<SlotValue>(
            value
                .as_deref()
                .ok_or_else(|| format!("result slot {slot:?} has no value payload"))?,
        )?;
        let extra = decode_slot_written_extra(
            extra
                .as_deref()
                .ok_or_else(|| format!("result slot {slot:?} has no taint envelope"))?,
        )
        .map_err(|error| format!("result slot {slot:?} taint envelope invalid: {error:?}"))?;
        return match extra {
            DecodedSlotWrittenExtra::Envelope(envelope) => Ok(Terminal {
                value,
                taint: envelope.taint,
            }),
            DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
                Err(format!("result slot {slot:?} carries legacy taint").into())
            }
        };
    }
    Err(format!("run {run:?} has no write for result slot {slot:?}").into())
}

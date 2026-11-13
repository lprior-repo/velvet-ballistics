//! Real-pipeline support for the source-primitive shipping matrix.
//!
//! Every scenario compiles authored YAML, persists an accepted artifact in a
//! real Fjall journal, drives a real shard runtime, and reads terminal claims
//! from durable journal events. Cold reopen drops both handles and reopens
//! the same directory.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint};
use vb_runtime::journal::{SharedRuntimeJournal, StorageRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::admission::AcceptedArtifact;
use vb_storage::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};
use vb_storage::{FjallJournal, JournalEvent};
#[path = "support/ordering.rs"]
mod ordering;
pub(super) use ordering::assert_ordering;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Durability adapter selected for a journal-backed runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Durability {
    Strict,
    Journaled,
}

/// One open, journal-backed runtime handle.
pub(super) struct OpenRuntime {
    pub(super) journal: Arc<FjallJournal>,
    pub(super) runtime: Runtime,
}

/// Compiles authored YAML through the production compiler.
pub(super) fn compile(yaml: &str) -> TestResult<CompiledWorkflow> {
    vb_compile::compile_workflow(yaml.as_bytes()).map_err(|error| Box::new(error) as _)
}

/// Opens the journal and a one-shard runtime over it.
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

/// Opens the journal and runtime with an explicit shard configuration.
pub(super) fn open_runtime_with_config(
    path: &Path,
    config: ShardConfig,
    durability: Durability,
) -> TestResult<OpenRuntime> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let shared: SharedRuntimeJournal = match durability {
        Durability::Strict => StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
        Durability::Journaled => StorageRuntimeJournal::shared_journaled(Arc::clone(&journal)),
    };
    let runtime = Runtime::new(std::num::NonZeroUsize::MIN, config, shared);
    Ok(OpenRuntime { journal, runtime })
}

/// Persists the accepted artifact for a compiled workflow.
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

/// Reads every durable event for one run.
pub(super) fn events(journal: &FjallJournal, run: RunId) -> TestResult<Vec<JournalEvent>> {
    Ok(journal.events_for_run_full(run)?)
}

/// Drives ticks until the run is terminal or the bounded tick budget expires.
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

/// Drives ticks until the pending ask ticket exists.
pub(super) fn tick_until_ask(
    runtime: &mut Runtime,
    run: RunId,
    step: vb_core::StepIdx,
    max_ticks: usize,
) -> TestResult<vb_runtime::shard::AskTicket> {
    for _ in 0..max_ticks {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(run, step) {
            return Ok(ticket);
        }
    }
    Err(format!("ask step {step:?} did not suspend").into())
}

/// Drives ticks until the pending action ticket exists.
pub(super) fn tick_until_action(
    runtime: &mut Runtime,
    run: RunId,
    step: u64,
    max_ticks: usize,
) -> TestResult<vb_core::action::ActionTicket> {
    for _ in 0..max_ticks {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, step) {
            return Ok(ticket);
        }
    }
    Err(format!("action step {step} did not suspend").into())
}

/// Terminal result slot, decoded value, and decoded taint envelope.
pub(super) struct Terminal {
    pub(super) slot: SlotIdx,
    pub(super) value: SlotValue,
    pub(super) taint: Taint,
}

/// Reads the exact durable terminal result for one completed run.
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
        let taint = match extra {
            DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
            DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
                return Err(format!("result slot {slot:?} carries legacy taint").into());
            }
        };
        return Ok(Terminal { slot, value, taint });
    }
    if let Some((value, taint)) = events.iter().rev().find_map(|event| match event {
        JournalEvent::ActionCompletedEnvelope {
            run: actual,
            output,
            value,
            taint,
            ..
        } if *actual == run && *output == slot => Some((value, *taint)),
        _ => None,
    }) {
        return Ok(Terminal {
            slot,
            value: postcard::from_bytes::<SlotValue>(value)?,
            taint,
        });
    }
    Err(format!("run {run:?} has no write for result slot {slot:?}").into())
}

/// Counts a journal event kind for exact occurrence assertions.
pub(super) fn count_where(
    events: &[JournalEvent],
    mut predicate: impl FnMut(&JournalEvent) -> bool,
) -> usize {
    events.iter().filter(|event| predicate(event)).count()
}

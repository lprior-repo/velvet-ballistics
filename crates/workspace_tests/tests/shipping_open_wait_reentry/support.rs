#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use vb_core::{CompiledWorkflow, RunId, RuntimePolicy};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

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
    _durability: Durability,
) -> TestResult<OpenRuntime> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let config = ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        max_open_gates: 1024,
        policy,
    };
    let runtime = Runtime::new(
        std::num::NonZeroUsize::MIN,
        config,
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    Ok(OpenRuntime { journal, runtime })
}

pub(super) fn events(journal: &FjallJournal, run: RunId) -> TestResult<Vec<JournalEvent>> {
    Ok(journal.events_for_run_full(run)?)
}

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

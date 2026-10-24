//! Exact shared F1 helpers, limited to this target to avoid unused test code.
use std::sync::Arc;
use vb_compile::compile_workflow;
use vb_core::{CompiledWorkflow, SlotValue};
use vb_runtime::journal::VolatileRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

pub(crate) fn compile_f1_workflow(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes())
        .map_err(|error| format!("F1 canonical source compilation failed: {error:?}"))
}

pub(crate) fn test_runtime_with_journal() -> Result<(Runtime, Arc<VolatileRuntimeJournal>), String>
{
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 8,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count must be non-zero")?;
    // Coerce Arc<VolatileRuntimeJournal> to Arc<dyn RuntimeJournal>
    let shared: Arc<dyn vb_runtime::journal::RuntimeJournal> = journal.clone();
    let runtime = Runtime::new(shard_count, config, shared);
    Ok((runtime, journal))
}

pub(crate) fn decode_slot_value(encoded: &[u8]) -> Result<SlotValue, String> {
    postcard::from_bytes::<SlotValue>(encoded)
        .map_err(|e| format!("failed to decode slot value: {e:?}"))
}

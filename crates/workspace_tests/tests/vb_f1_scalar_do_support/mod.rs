//! F1 bounded slice execution helpers for vb-8eocj.2.
//! Grounded in actual Runtime API and drive_deterministic_full harness.

use std::sync::Arc;

use vb_compile::compile_workflow;
use vb_core::{
    new_run_frame, CapabilitySet, CompiledWorkflow, RunFrame, RunId, SlotIdx, SlotValue, Taint,
    ValueStore,
};
use vb_runtime::engine::{drive_deterministic_full, EvidenceCollector, RuntimeSignal};
use vb_runtime::journal::VolatileRuntimeJournal;
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

/// Compiles a canonical scalar Do workflow from inline YAML.
pub(crate) fn compile_f1_workflow(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes())
        .map_err(|error| format!("F1 canonical source compilation failed: {error:?}"))
}

/// Builds the F1 deterministic test harness from YAML source.
pub(crate) struct F1SourceRun {
    pub workflow: CompiledWorkflow,
    pub frame: RunFrame,
    pub store: ValueStore,
    pub evidence: EvidenceCollector,
    pub collect_states: CollectStates,
}

impl F1SourceRun {
    /// Creates a new F1 source-to-runtime test harness from YAML source.
    pub(crate) fn new(source: &str) -> Result<Self, String> {
        let workflow = compile_f1_workflow(source)?;
        let frame = new_run_frame(RunId::new(1), &workflow)
            .map_err(|error| format!("F1 run frame creation failed: {error:?}"))?;
        Ok(Self {
            workflow,
            frame,
            store: ValueStore::new(),
            evidence: EvidenceCollector::new(),
            collect_states: CollectStates::new(),
        })
    }

    /// Drives the compiled workflow for the specified number of steps.
    pub(crate) fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, String> {
        drive_deterministic_full(
            &self.workflow,
            &mut self.frame,
            &mut vb_core::engine::StepBudget::new(steps),
            &mut self.store,
            &[],
            vb_runtime::engine::RetryPolicy::NEVER,
            &mut self.evidence,
            &mut self.collect_states,
            &CapabilitySet::empty(),
            1,
        )
        .map_err(|error| format!("F1 unchanged source IR execution failed: {error:?}"))
    }
}

/// Asserts that a F1 source run finished with the expected SlotValue.
pub(crate) fn assert_finished_with_value(
    run: &mut F1SourceRun,
    expected: SlotValue,
) -> Result<(), String> {
    let signal = run.drive(256)?;
    match signal {
        RuntimeSignal::Finished(value) => {
            if value != expected {
                return Err(format!(
                    "F1 run finished with {value:?} but expected {expected:?}"
                ));
            }
            Ok(())
        }
        other => Err(format!("F1 run did not finish: {other:?}")),
    }
}

/// Creates a test Runtime with relaxed config for action suspension tests.
/// Returns the Runtime plus a cloned Arc of the volatile journal for inspection.
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

/// Creates a deterministic action output with the given value.
/// Computes the actual encoded length using postcard serialization.
pub(crate) fn action_output(
    value: SlotValue,
) -> Result<vb_core::action::ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value)
        .map_err(|e| format!("failed to encode action output: {e:?}"))?;
    let encoded_len = u32::try_from(encoded.len())
        .map_err(|e| format!("encoded length does not fit u32: {e:?}"))?;
    Ok(vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len,
    })
}

/// Decodes a postcard-encoded SlotValue from journal event bytes.
pub(crate) fn decode_slot_value(encoded: &[u8]) -> Result<SlotValue, String> {
    postcard::from_bytes::<SlotValue>(encoded)
        .map_err(|e| format!("failed to decode slot value: {e:?}"))
}

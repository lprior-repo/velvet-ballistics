//! F1 bounded slice execution helpers for vb-8eocj.2.
//! Grounded in actual Runtime API and drive_deterministic_full harness.

use std::sync::Arc;

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledWorkflow, RunFrame, RunId, SlotIdx, SlotValue, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::journal::VolatileRuntimeJournal;
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

/// Compiles a canonical scalar Do workflow from inline YAML.
pub fn compile_f1_workflow(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes())
        .map_err(|error| format!("F1 canonical source compilation failed: {error:?}"))
}

/// Builds the F1 deterministic test harness from YAML source.
pub struct F1SourceRun {
    pub workflow: CompiledWorkflow,
    pub frame: RunFrame,
    pub store: ValueStore,
    pub evidence: EvidenceCollector,
    pub collect_states: CollectStates,
}

impl F1SourceRun {
    /// Creates a new F1 source-to-runtime test harness from YAML source.
    pub fn new(source: &str) -> Result<Self, String> {
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
    pub fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, String> {
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
pub fn assert_finished_with_value(run: &mut F1SourceRun, expected: SlotValue) -> Result<(), String> {
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
pub fn test_runtime() -> Result<Runtime, String> {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 8,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shard_count = std::num::NonZeroUsize::new(1)
        .ok_or("shard count must be non-zero")?;
    Ok(Runtime::new(shard_count, config, journal))
}

/// Creates a deterministic action output with the given value.
pub fn action_output(value: SlotValue) -> vb_core::action::ActionOutputReady {
    vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: 8,
    }
}

use std::{num::NonZeroUsize, sync::Arc};
use vb_compile::compile_workflow;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{ActionId, CapabilitySet, CompiledWorkflow, RunId};
use vb_runtime::{journal::VolatileRuntimeJournal, runtime::Runtime, shard::ShardConfig};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(crate) fn workflow() -> TestResult<CompiledWorkflow> {
    Ok(compile_workflow(b"version: velvet-ballistics/v1\nname: secret_reference\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: invoke\n    do:\n      action: '0'\n      with: {auth: '$secrets.token', nested: {auth: '$secrets.token'}}\n  - id: done\n    finish: {result: '$step.invoke'}\n")?)
}
pub(crate) fn runtime() -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    (
        Runtime::new(
            NonZeroUsize::new(2).unwrap_or(NonZeroUsize::MIN),
            ShardConfig {
                policy: vb_core::RuntimePolicy::Relaxed,
                ..ShardConfig::default()
            },
            journal.clone(),
        ),
        journal,
    )
}
pub(crate) fn contracts() -> TestResult<Box<[ActionContract]>> {
    Ok(Box::new([ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("opaque-reference")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Sends,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    }]))
}
pub(crate) fn submit(runtime: &mut Runtime, run: RunId, workflow: CompiledWorkflow) -> TestResult {
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::new([]),
        CapabilitySet::empty(),
        contracts()?,
    )?;
    runtime.tick_all()?;
    Ok(())
}

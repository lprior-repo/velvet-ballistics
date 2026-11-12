use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{
    ActionId, CapabilitySet, CompiledWorkflow, RunId, SlotIdx, SlotValue, Taint,
    action::{ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect},
};
use vb_runtime::{
    journal::{SharedRuntimeJournal, StorageRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};
use vb_storage::FjallJournal;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(crate) const RUN: RunId = RunId::new(31);

pub(crate) fn source(attempts: u16, delay: &str, handler: bool) -> String {
    let on_error = if handler {
        "    on_error: {handler: done}\n"
    } else {
        ""
    };
    format!(
        "version: velvet-ballistics/v1\nname: retry\nwhen: {{manual: {{}}}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: work\n{on_error}    try_again: {{max_attempts: {attempts}, delay: '{delay}'}}\n    do: {{action: '0', input: '$seed'}}\n  - id: done\n    finish: {{result: seed}}\n"
    )
}
pub(crate) fn compile(attempts: u16, delay: &str, handler: bool) -> TestResult<CompiledWorkflow> {
    Ok(vb_compile::compile_workflow(
        source(attempts, delay, handler).as_bytes(),
    )?)
}
pub(crate) fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("retry-test")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}
pub(crate) fn runtime(journal: SharedRuntimeJournal) -> TestResult<Runtime> {
    runtime_with_policy(journal, vb_core::policy::RuntimePolicy::Relaxed)
}
pub(crate) fn runtime_with_policy(
    journal: SharedRuntimeJournal,
    policy: vb_core::policy::RuntimePolicy,
) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("zero shards")?,
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 32,
            step_budget_per_tick: 16,
            max_active_runs: 4,
            policy,
        },
        journal,
    ))
}
pub(crate) fn durable() -> TestResult<(tempfile::TempDir, Arc<FjallJournal>, Runtime)> {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let runtime = runtime(StorageRuntimeJournal::shared_journaled(journal.clone()))?;
    Ok((directory, journal, runtime))
}
pub(crate) fn submit(runtime: &mut Runtime, workflow: CompiledWorkflow) -> TestResult {
    runtime.submit_direct_with_inputs_grants_and_contracts(
        RUN,
        workflow,
        Box::new([]),
        CapabilitySet::empty(),
        Box::from([contract()?]),
    )?;
    runtime.tick_all()?;
    Ok(())
}
pub(crate) fn ticket(runtime: &Runtime) -> TestResult<vb_core::action::ActionTicket> {
    Ok(runtime
        .lookup_pending_action_ticket(RUN, 1)
        .ok_or("missing action ticket")?)
}
pub(crate) fn succeed(runtime: &mut Runtime) -> TestResult {
    let tick_bound = runtime.inspect_run_state(RUN)?.workflow.node_count();
    let completed = runtime
        .counters_snapshot()
        .runs_completed
        .checked_add(1)
        .ok_or("completed counter overflow")?;
    let value = SlotValue::I64(99);
    let encoded_len = u32::try_from(postcard::to_allocvec(&value)?.len())?;
    runtime.complete_action_with_output(
        ticket(runtime)?,
        ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value,
            encoded_len,
            taint: Taint::Clean,
        },
    )?;
    // These fixtures have one run with an acyclic post-action tail.
    for _ in 0..tick_bound {
        if !runtime.tick_all()? {
            return Err("runtime stopped before retry fixture completion".into());
        }
        if runtime.counters_snapshot().runs_completed == completed {
            return Ok(());
        }
    }
    Err("retry fixture did not finish within its workflow-node tick bound".into())
}
pub(crate) fn fail(runtime: &mut Runtime, code: vb_core::action::ActionFailureCode) -> TestResult {
    runtime.fail_action(ticket(runtime)?, code.into())?;
    runtime.tick_all()?;
    Ok(())
}

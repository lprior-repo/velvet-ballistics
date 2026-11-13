//! Shared fixtures for the `shipping_suspension_recovery` scenarios.
//!
//! Workflows are compiled from real source YAML and driven by the production
//! `Runtime` against a real Fjall store, so every durable prefix asserted
//! below is evidence the runtime actually wrote.

use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskTicket, ShardConfig};
use vb_storage::{FjallJournal, JournalEvent};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const RUN: RunId = RunId::new(1909);

/// Failure on `work` routes to the `recovered` ask, so recovery has to restore
/// the handler jump before the ask suspension can continue.
pub(crate) fn routed_failure_source() -> &'static str {
    "version: velvet-ballistics/v1\nname: shipping_routing\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: work\n    on_error: {handler: recovered}\n    do: {action: '0', input: '$seed'}\n  - id: recovered\n    ask: {prompt: '$seed'}\n  - id: done\n    finish: {result: seed}\n"
}

pub(crate) fn ask_source() -> &'static str {
    "version: velvet-ballistics/v1\nname: shipping_ask\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: response\n    ask: {prompt: '$seed'}\n  - id: done\n    finish: {result: seed}\n"
}

pub(crate) fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("shipping-suspension-recovery")?,
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

pub(crate) fn config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 32,
        step_budget_per_tick: 16,
        max_active_runs: 4,
        policy: RuntimePolicy::Journaled,
    }
}

pub(crate) fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        config(),
        StorageRuntimeJournal::shared_journaled(Arc::clone(&journal)),
    );
    Ok((journal, runtime))
}

pub(crate) fn submit(
    journal: &FjallJournal,
    runtime: &mut Runtime,
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> TestResult {
    vb_storage::submit_artifact_with_contracts(
        journal,
        workflow,
        RuntimePolicy::Journaled,
        contracts,
    )?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        RUN,
        workflow.clone(),
        Box::new([]),
        CapabilitySet::empty(),
        contracts.to_vec().into_boxed_slice(),
    )?;
    runtime.tick_all()?;
    Ok(())
}

pub(crate) fn drain_until_completed(runtime: &mut Runtime) -> TestResult {
    for _ in 0..8 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed > 0 {
            return Ok(());
        }
    }
    Err(String::from("run did not complete within the tick bound").into())
}

pub(crate) fn pending_ask(runtime: &mut Runtime, ask: StepIdx) -> TestResult<AskTicket> {
    for _ in 0..128 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            return Ok(ticket);
        }
    }
    Err(String::from("ask boundary not reached").into())
}

pub(crate) fn node_with(
    workflow: &CompiledWorkflow,
    predicate: impl Fn(&CompiledNodeKind) -> bool,
) -> TestResult<StepIdx> {
    (0..workflow.node_count())
        .find_map(|raw| {
            let node = workflow.node(StepIdx::new(raw))?;
            predicate(&node.kind).then_some(node.id)
        })
        .ok_or_else(|| String::from("compiled node absent").into())
}

pub(crate) fn action_step(workflow: &CompiledWorkflow) -> TestResult<StepIdx> {
    node_with(workflow, |kind| matches!(kind, CompiledNodeKind::Do { .. }))
}

pub(crate) fn ask_step(workflow: &CompiledWorkflow) -> TestResult<StepIdx> {
    node_with(workflow, |kind| {
        matches!(kind, CompiledNodeKind::Ask { .. })
    })
}

pub(crate) fn finish_result_slot(workflow: &CompiledWorkflow) -> TestResult<SlotIdx> {
    (0..workflow.node_count())
        .find_map(|raw| match workflow.node(StepIdx::new(raw))?.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or_else(|| String::from("compiled finish node absent").into())
}

/// Runs a routing workflow live until the handler ask suspends, then returns
/// the durable prefix ending at the `ActionFailed` append plus the live run's
/// ask ticket.
pub(crate) fn live_routed_failure_prefix(
    workflow: &CompiledWorkflow,
    ask: StepIdx,
) -> TestResult<(Vec<JournalEvent>, AskTicket)> {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    submit(&journal, &mut runtime, workflow, &[contract()?])?;
    let work = action_step(workflow)?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("scheduled action ticket")?;
    runtime.fail_action(
        ticket,
        vb_core::action::ActionFailureCode::InvalidInput.into(),
    )?;
    let ask_ticket = pending_ask(&mut runtime, ask)?;
    let events = journal.events_for_run_full(RUN)?;
    let failed = events
        .iter()
        .position(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
        .ok_or("ActionFailed evidence")?;
    Ok((
        events.get(..=failed).ok_or("prefix range")?.to_vec(),
        ask_ticket,
    ))
}

/// Computes the canonical workflow digest exactly as artifact admission does:
/// hash the postcard encoding of the parts with the digest field zeroed.
pub(crate) fn content_digest(
    parts: &mut vb_core::workflow::WorkflowParts,
) -> TestResult<vb_core::WorkflowDigest> {
    parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
    let bytes = postcard::to_allocvec(parts)?;
    Ok(vb_core::WorkflowDigest::from_bytes(
        blake3::hash(&bytes).into(),
    ))
}

/// Opens a scratch store carrying only `prefix` and the compiled artifact.
pub(crate) fn scratch_store(
    workflow: &CompiledWorkflow,
    prefix: &[JournalEvent],
    contracts: &[ActionContract],
) -> TestResult<(tempfile::TempDir, Arc<FjallJournal>, Runtime)> {
    let directory = tempfile::tempdir()?;
    let (journal, runtime) = open(directory.path())?;
    vb_storage::submit_artifact_with_contracts(
        journal.as_ref(),
        workflow,
        RuntimePolicy::Journaled,
        contracts,
    )?;
    for event in prefix {
        journal.append_strict(event)?;
    }
    Ok((directory, journal, runtime))
}

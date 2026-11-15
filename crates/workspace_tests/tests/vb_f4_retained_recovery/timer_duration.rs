use super::*;
use vb_runtime::RuntimeError;
use vb_storage::{CompiledIrRecord, JournalEvent, RunInputsRecord, WorkflowSourceRecord};

const HUGE_PREFIX: &[u8] = include_bytes!("huge_pending_prefix_hardcaps35.postcard");
const HUGE_SOURCE: &[u8] = include_bytes!("huge_pending_workflow.yaml");
const HUGE_RUN: RunId = RunId::new(51_001);
const ORDINARY_RUN: RunId = RunId::new(51_002);
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: cold_timer\nwhen: {manual: {}}\nvars: {prompt: question}\nsteps:\n  - id: prompt\n    set: {output: prompt_text, value: \"$vars.prompt\"}\n  - id: duration\n    set: {output: duration_ms, value: \"600000\"}\n  - id: question\n    ask: {prompt: \"$prompt\", timeout: \"$duration\"}\n  - id: done\n    finish: {result: \"$question\"}\n";

#[path = "timer_duration/ipc.rs"]
mod ipc;

type StoredPrefix = (
    Vec<Option<Vec<u8>>>,
    Option<vb_storage::RunHeaderRecord>,
    RunInputsRecord,
    Option<WorkflowSourceRecord>,
    Option<CompiledIrRecord>,
);

fn strict_runtime(journal: Arc<FjallJournal>, step_budget: u64) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: step_budget,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )?)
}

fn baseline_events(raw: &[Option<Vec<u8>>]) -> TestResult<Vec<JournalEvent>> {
    let events = raw
        .iter()
        .map(|record| -> TestResult<_> {
            Ok(vb_storage::journal::parse_event(
                record.as_deref().ok_or("fixture event")?,
            )?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if events.len() != 15 {
        return Err("fixture event count".into());
    }
    Ok(events)
}

fn current_huge_workflow(journal: &FjallJournal) -> TestResult<vb_core::CompiledWorkflow> {
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(HUGE_SOURCE).into()),
        source: HUGE_SOURCE.to_vec(),
    };
    journal.put_workflow_source(&source)?;
    let workflow = vb_compile::compile_workflow(HUGE_SOURCE)?;
    let artifact = vb_storage::submit_artifact(journal, &workflow, RuntimePolicy::Strict)?;
    let issued = journal
        .compiled_ir(artifact.digest)?
        .ok_or("issued artifact")?;
    let (_, _, _, _, expected): StoredPrefix = postcard::from_bytes(HUGE_PREFIX)?;
    if Some(issued) != expected {
        return Err("fixture artifact differs from current strict issuance".into());
    }
    Ok(workflow)
}

fn run_to_huge_wait(journal: Arc<FjallJournal>, workflow: vb_core::CompiledWorkflow) -> TestResult {
    let mut runtime = strict_runtime(journal.clone(), 1)?;
    runtime.submit_compiled(HUGE_RUN, workflow)?;
    for _ in 0..2 {
        runtime.tick_all()?;
    }
    Ok(())
}

fn validate_current_prefix(current: &[JournalEvent], baseline: &[JournalEvent]) -> TestResult {
    if current.len() != 10 || current.get(..3) != baseline.get(..3) {
        return Err("strict runtime did not produce the expected current prefix".into());
    }
    if current.get(4..) != baseline.get(4..10) {
        return Err("runtime prefix differs from genuine historical steps".into());
    }
    if !matches!(
        (current.get(3), baseline.get(3)),
        (
            Some(JournalEvent::RunBudgetOriginEvent { run: current_run, .. }),
            Some(JournalEvent::RunBudgetOriginEvent { run: baseline_run, .. })
        ) if current_run == baseline_run
    ) {
        return Err("strict runtime budget-origin event is invalid".into());
    }
    Ok(())
}

fn append_historical_suffix(journal: &FjallJournal, events: &[JournalEvent]) -> TestResult {
    for event in events.iter().skip(10) {
        journal.append_strict(event)?;
    }
    Ok(())
}

fn retained_event_bytes(
    journal: &FjallJournal,
    events: &[JournalEvent],
    raw: &[Option<Vec<u8>>],
) -> TestResult<Vec<Vec<u8>>> {
    let mut retained = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let actual = journal
            .get_event_bytes(HUGE_RUN, event.seq())?
            .ok_or("stored event")?;
        if index >= 4 && Some(&actual) != raw.get(index).and_then(Option::as_ref) {
            return Err("fixture bytes changed during reconstruction".into());
        }
        retained.push(actual);
    }
    Ok(retained)
}

fn seed_current_huge_prefix(
    journal: Arc<FjallJournal>,
) -> TestResult<(Vec<Vec<u8>>, RunInputsRecord)> {
    let (raw, _, _, _, _): StoredPrefix = postcard::from_bytes(HUGE_PREFIX)?;
    let baseline = baseline_events(&raw)?;
    let workflow = current_huge_workflow(&journal)?;
    run_to_huge_wait(journal.clone(), workflow)?;
    let current = journal.events_for_run_full(HUGE_RUN)?;
    validate_current_prefix(&current, &baseline)?;
    append_historical_suffix(&journal, &baseline)?;
    let events = journal.events_for_run_full(HUGE_RUN)?;
    let retained = retained_event_bytes(&journal, &events, &raw)?;
    let inputs = journal.run_inputs(HUGE_RUN)?.ok_or("retained inputs")?;
    Ok((retained, inputs))
}

fn assert_timer_refusal(result: &vb_runtime::RuntimeResult<bool>) -> TestResult {
    if matches!(
        result,
        Err(RuntimeError::Core { source })
            if matches!(source.as_ref(), vb_core::errors::CoreError::ResourceLimitExceeded {
                resource: "timer_duration"
            })
    ) {
        Ok(())
    } else {
        Err(format!("expected typed duration refusal, got {result:?}").into())
    }
}

fn assert_no_ask_authority(runtime: &Runtime, run: RunId) -> TestResult {
    if runtime
        .lookup_pending_ask_ticket(run, vb_core::StepIdx::new(3))
        .is_some()
    {
        return Err("huge timeout installed Ask authority".into());
    }
    Ok(())
}

fn assert_retained_bytes(journal: &FjallJournal, retained: &[Vec<u8>]) -> TestResult {
    let events = journal.events_for_run_full(HUGE_RUN)?;
    if events.len() != retained.len() {
        return Err("cold recovery changed the retained event count".into());
    }
    for (index, expected) in retained.iter().enumerate() {
        let seq = u64::try_from(index)?;
        let actual = journal
            .get_event_bytes(HUGE_RUN, vb_storage::EventSeq::new(seq))?
            .ok_or("retained event disappeared")?;
        if &actual != expected {
            return Err("recovery changed retained event bytes".into());
        }
    }
    Ok(())
}

fn assert_recovery_unpublished(runtime: &Runtime, run: RunId, queue_depth: usize) -> TestResult {
    if !matches!(
        runtime.inspect_run_state(run),
        Err(RuntimeError::RunNotFound)
    ) {
        return Err("failed timer recovery published run state".into());
    }
    let pending = runtime.pending_boundary_snapshot(8);
    if pending.command_queue_depth() != queue_depth
        || pending.active_run_count() != 0
        || pending.pending_timer_count() != 0
        || pending.pending_action_count() != 0
        || pending.pending_ask_count() != 0
        || runtime.global_cost_usage() != 0
    {
        return Err("failed timer recovery published boundary or quota state".into());
    }
    Ok(())
}

#[test]
fn huge_retained_ask_refuses_cold_restore_without_authority_or_prefix_writes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let (retained, inputs) = seed_current_huge_prefix(journal.clone())?;
    drop(journal);

    let reopened = Arc::new(FjallJournal::open(directory.path(), None)?);
    let workflow = vb_compile::compile_workflow(HUGE_SOURCE)?;
    let budget = vb_core::AggregateResourceBudget::from_workflow(&workflow)?;
    let mut runtime = strict_runtime(reopened.clone(), 10_000)?;
    runtime.set_resource_quotas(budget.max_cost_units, budget.max_cost_units)?;
    runtime.recover_and_resume(HUGE_RUN)?;
    assert_recovery_unpublished(&runtime, HUGE_RUN, 1)?;
    let result = runtime.tick_all();
    assert_timer_refusal(&result)?;
    assert_recovery_unpublished(&runtime, HUGE_RUN, 0)?;
    assert_no_ask_authority(&runtime, HUGE_RUN)?;
    assert_retained_bytes(&reopened, &retained)?;
    if reopened.run_inputs(HUGE_RUN)? != Some(inputs) {
        return Err("cold recovery changed retained input bytes".into());
    }
    Ok(())
}

fn start_ordinary_ask(directory: &std::path::Path) -> TestResult<()> {
    let journal = Arc::new(FjallJournal::open(directory, None)?);
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    journal.put_workflow_source(&WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
        source: SOURCE.to_vec(),
    })?;
    vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
    let mut runtime = strict_runtime(journal.clone(), 10_000)?;
    runtime.submit_compiled(ORDINARY_RUN, workflow)?;
    runtime.tick_all()?;
    if runtime
        .lookup_pending_ask_ticket(ORDINARY_RUN, vb_core::StepIdx::new(3))
        .is_none()
    {
        return Err("ordinary Ask was not issued".into());
    }
    Ok(())
}

fn recover_ordinary_ask(directory: &std::path::Path) -> TestResult<()> {
    let reopened = Arc::new(FjallJournal::open(directory, None)?);
    let mut recovered = strict_runtime(reopened, 10_000)?;
    recovered.recover_and_resume(ORDINARY_RUN)?;
    recovered.tick_all()?;
    if recovered
        .lookup_pending_ask_ticket(ORDINARY_RUN, vb_core::StepIdx::new(3))
        .is_none()
    {
        return Err("ordinary Ask authority was not restored".into());
    }
    Ok(())
}

#[test]
fn ordinary_strict_ask_survives_cold_recovery() -> TestResult {
    let directory = tempfile::tempdir()?;
    start_ordinary_ask(directory.path())?;
    recover_ordinary_ask(directory.path())
}

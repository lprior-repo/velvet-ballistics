#![forbid(unsafe_code)]
use std::{num::NonZeroUsize, path::Path, process::Command, sync::Arc};
use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, Idempotency, RetryPolicy,
    RetrySafety, SideEffect,
};
use vb_core::{ActionId, CapabilitySet, CompiledNodeKind, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{FjallJournal, JournalEvent};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const KEY: &str = "VB_F1_FAKE_RECOVERY_SECRET_20261110";
const VALUE: &str = "fixture-only-recovery-value-77453199-0ccc-4963-98de-bb517eb5cd13";
const RUN: RunId = RunId::new(91_104);

fn source(retry: bool) -> String {
    let action = if retry {
        "  - id: invoke\n    do: {action: '0', with: 7}\n"
    } else {
        ""
    };
    format!(
        "version: velvet-ballistics/v1\nname: secret_recovery\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY}}}\nsteps:\n{action}  - id: done\n    finish: {{result: '$secrets.token'}}\n"
    )
}

fn command(name: &str, db: &Path, present: bool) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args([name, &RUN.get().to_string(), "--db"]).arg(db);
    command.env_remove(KEY);
    if present {
        command.env(KEY, VALUE);
    }
    command
}

fn no_raw_value(bytes: &[u8]) {
    assert!(
        !bytes
            .windows(VALUE.len())
            .any(|part| part == VALUE.as_bytes())
    );
}

fn events(db: &Path) -> TestResult<Vec<JournalEvent>> {
    let journal = FjallJournal::open(db, None)?;
    let events = journal.events_for_run_full(RUN)?;
    no_raw_value(&postcard::to_allocvec(&events)?);
    let digest = vb_runtime::recovery::hydrate_run_admission_from_events(&events)
        .ok_or("admission")?
        .artifact_digest();
    let record = journal.compiled_ir(digest)?.ok_or("artifact")?;
    no_raw_value(&postcard::to_allocvec(&record)?);
    Ok(events)
}

fn failed_run(db: &Path) -> TestResult<vb_core::StepIdx> {
    let workflow = vb_compile::compile_workflow(source(true).as_bytes())?;
    let step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .ok_or("do")?
        .id;
    let contracts: Box<[ActionContract]> = Box::new([ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("recovery-fixture")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }]);
    let journal = Arc::new(FjallJournal::open(db, None)?);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig::default(),
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.set_available_secret_names(Box::new([KEY.into()]));
    runtime.submit_direct_with_inputs_grants_and_contracts(
        RUN,
        workflow,
        Box::new([]),
        CapabilitySet::empty(),
        contracts,
    )?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(step.get()))
        .ok_or("action ticket")?;
    runtime.fail_action(
        ticket,
        ActionFailure {
            code: ActionFailureCode::Rejected,
            retry_policy: RetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        },
    )?;
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    assert_eq!(
        journal
            .events_for_run_full(RUN)?
            .iter()
            .filter(|event| matches!(event, JournalEvent::RetryCheckpoint { .. }))
            .count(),
        1
    );
    Ok(step)
}

fn admitted_run(db: &Path) -> TestResult<vb_core::SlotIdx> {
    let text = source(false);
    let workflow = vb_compile::compile_workflow(text.as_bytes())?;
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into()),
        source: text.into_bytes(),
    };
    let record = vb_storage::RunInputsRecord::new(
        RUN,
        artifact.digest,
        source.digest,
        &workflow,
        Box::new([(
            binding.slot().ok_or("slot")?,
            SlotValue::Symbol(binding.requirement()),
        )]),
    )?;
    let submission = vb_storage::RunSubmission::new(
        source,
        artifact,
        record,
        vb_storage::RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    let journal = FjallJournal::open(db, None)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(journal.events_for_run_full(RUN)?.len(), 3);
    Ok(binding.slot().ok_or("slot")?)
}

#[test]
fn retry_uses_stored_declared_names_and_reexecutes_with_host_presence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let step = failed_run(directory.path())?;
    let before = events(directory.path())?;
    let output = command("retry", directory.path(), true)
        .args(["--step", &step.get().to_string()])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "run {}: retried step {} completed=0 failed=0\nrun accepted but not terminal after one runtime tick\n",
            RUN.get(),
            step.get()
        )
    );
    no_raw_value(&output.stdout);
    let after = events(directory.path())?;
    assert_eq!(after.get(..before.len()), Some(before.as_slice()));
    let tickets = after
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } => Some(ticket),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [first, second] = tickets.as_slice() else {
        return Err(format!("expected original and retried tickets: {tickets:?}").into());
    };
    assert_eq!(first.run, RUN);
    assert_eq!(second.run, RUN);
    assert_eq!(second.step, step);
    assert_eq!(first.run_attempt, 1);
    assert_eq!(first.attempt, 1);
    assert_eq!(second.run_attempt, 2);
    assert_eq!(second.attempt, 1);
    assert_eq!(second.action, first.action);
    Ok(())
}

#[test]
fn retry_without_host_presence_is_typed_and_does_not_append() -> TestResult {
    let directory = tempfile::tempdir()?;
    let step = failed_run(directory.path())?;
    let before = events(directory.path())?;
    let output = command("retry", directory.path(), false)
        .args(["--step", &step.get().to_string()])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "runtime tick error: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    assert_eq!(events(directory.path())?, before);
    Ok(())
}

#[test]
fn pristine_resume_uses_stored_declared_names_and_completes_with_host_presence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let result = admitted_run(directory.path())?;
    let before = events(directory.path())?;
    let output = command("resume", directory.path(), true).output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    assert!(String::from_utf8_lossy(&output.stdout).ends_with("run completed\n"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("run result: restricted\n"));
    no_raw_value(&output.stdout);
    let after = events(directory.path())?;
    assert_eq!(after.get(..before.len()), Some(before.as_slice()));
    assert_eq!(
        after
            .iter()
            .filter_map(|event| match event {
                JournalEvent::RunFinished { result, .. } => Some(*result),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![result]
    );
    assert_eq!(
        after
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}

#[test]
fn pristine_resume_without_host_presence_is_typed_and_does_not_append() -> TestResult {
    let directory = tempfile::tempdir()?;
    admitted_run(directory.path())?;
    let before = events(directory.path())?;
    let output = command("resume", directory.path(), false).output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "resume runtime: recovery: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    assert_eq!(events(directory.path())?, before);
    Ok(())
}

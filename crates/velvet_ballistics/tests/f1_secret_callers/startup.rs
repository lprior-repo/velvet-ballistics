use super::support::*;
use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{CompiledNodeKind, CompiledWorkflow, RunId};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};

fn pending(db: &Path, workflow: &CompiledWorkflow, run: RunId) -> TestResult {
    let journal = Arc::new(vb_storage::FjallJournal::open(db, None)?);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig::default(),
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.set_available_secret_names(Box::new([KEY_A.into(), KEY_B.into()]));
    runtime.submit_compiled(run, workflow.clone())?;
    runtime.tick_all()?;
    let step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .ok_or("ask")?
        .id;
    assert!(runtime.lookup_pending_ask_ticket(run, step).is_some());
    journal.put_run_header(&vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    })?;
    Ok(())
}

#[test]
fn actual_startup_checks_present_names_and_retains_them_for_dynamic_workflow() -> TestResult {
    let directory = tempfile::tempdir()?;
    let first = workflow(KEY_A, true)?;
    let second = workflow(KEY_B, true)?;
    assert_ne!(first.digest(), second.digest());
    let run = RunId::new(85);
    pending(directory.path(), &first, run)?;
    pending(directory.path(), &second, RunId::new(86))?;
    let before = journal_events(directory.path(), run)?;
    let before_second = journal_events(directory.path(), RunId::new(86))?;
    let server = Server::start(directory.path(), &[KEY_A, KEY_B])?;
    let mut stream = server.connect()?;
    assert_eq!(
        submit(&mut stream, &first, RunId::new(95), true)?,
        vb_ipc::server::IpcResponse::AcceptedRun { run_id: 95 }
    );
    answer(&mut stream, &first, run, 85)?;
    answer(&mut stream, &second, RunId::new(86), 86)?;
    answer(&mut stream, &first, RunId::new(95), 95)?;
    server.stop()?;
    let after = journal_events(directory.path(), run)?;
    assert_eq!(after.get(..before.len()), Some(before.as_slice()));
    let after_second = journal_events(directory.path(), RunId::new(86))?;
    assert_eq!(
        after_second.get(..before_second.len()),
        Some(before_second.as_slice())
    );
    for run in [85, 86, 95] {
        let events = journal_events(directory.path(), RunId::new(run))?;
        assert_answer_result(&events, i64::try_from(run)?)?;
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
                .count(),
            0
        );
    }
    assert_artifact(directory.path(), &first)?;
    assert_artifact(directory.path(), &second)?;
    Ok(())
}

#[test]
fn actual_startup_missing_name_rejects_without_new_journal_evidence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = workflow(KEY_A, true)?;
    let run = RunId::new(87);
    pending(directory.path(), &workflow, run)?;
    let before = journal_events(directory.path(), run)?;
    let socket = directory.path().join("ipc.sock");
    let output = command()
        .args(["ipc-serve", "--db"])
        .arg(directory.path())
        .arg("--socket")
        .arg(&socket)
        .output()?;
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "error recovering startup runs: startup discovery visitor failure: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    assert!(!socket.exists());
    assert_eq!(journal_events(directory.path(), run)?, before);
    Ok(())
}

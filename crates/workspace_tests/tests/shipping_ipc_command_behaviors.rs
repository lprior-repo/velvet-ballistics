#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "artifact_evidence_support/mod.rs"]
mod artifact_fixture;

use std::{
    io::{Read, Write},
    num::NonZeroUsize,
    path::Path,
    sync::Arc,
    time::Duration,
};
use vb_core::{
    RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, capability::CapabilitySet,
};
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{
    IPC_HEADER_LEN, IpcActionOutputPayload, IpcCommand, IpcFrameHeader, IpcPayload, MaxPayloadBytes,
};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{FjallJournal, JournalEvent};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    Ok((journal, runtime))
}

fn submit_pending(runtime: &Runtime, journal: &FjallJournal, run: RunId) -> TestResult {
    let workflow = artifact_fixture::action_workflow()?;
    let mut contract = artifact_fixture::contract()?;
    contract.side_effect = vb_core::SideEffect::None;
    contract.idempotency = vb_core::Idempotency::DeterministicPure;
    contract.retry_safety = vb_core::RetrySafety::Safe;
    let contracts = Box::from([contract]);
    vb_storage::submit_artifact_with_contracts(
        journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    runtime.submit_direct_with_grants_and_contracts(
        run,
        workflow,
        CapabilitySet::empty(),
        contracts,
    )?;
    Ok(())
}

fn tick_pending(runtime: &mut Runtime, run: RunId) -> TestResult<vb_core::action::ActionTicket> {
    for _ in 0..12 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, 1) {
            return Ok(ticket);
        }
    }
    Err(format!("run {run:?} did not reach pending Do").into())
}

fn wire(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    path: &Path,
    command: IpcCommand,
    correlation: u64,
    payload: &[u8],
) -> TestResult<IpcResponse> {
    let mut stream = std::os::unix::net::UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let length = u32::try_from(payload.len())?;
    let header = IpcFrameHeader::new(command, 0, correlation, length);
    stream.write_all(&header.encode()?)?;
    stream.write_all(payload)?;
    for _ in 0..8 {
        server.poll_once(runtime, Some(Duration::ZERO))?;
    }
    let mut header_bytes = [0; IPC_HEADER_LEN];
    stream.read_exact(&mut header_bytes)?;
    let response_header = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)?;
    assert_eq!(response_header.command, command);
    assert_eq!(response_header.correlation, correlation);
    let mut response_bytes = vec![0; usize::try_from(response_header.payload_len)?];
    stream.read_exact(&mut response_bytes)?;
    Ok(postcard::from_bytes(&response_bytes)?)
}

fn request(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    path: &Path,
    command: IpcCommand,
    correlation: u64,
    payload: IpcPayload,
) -> TestResult<IpcResponse> {
    wire(
        server,
        runtime,
        path,
        command,
        correlation,
        &postcard::to_allocvec(&payload)?,
    )
}

fn raw_rows(path: &Path) -> TestResult<Vec<(String, Vec<u8>, Vec<u8>)>> {
    let db = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = db.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

fn unchanged_after(
    runtime: &Runtime,
    journal: &FjallJournal,
    run: RunId,
    before_events: &[JournalEvent],
    before_state: &vb_runtime::shard::types::RunState,
    before_pending: &vb_runtime::runtime::RuntimePendingBoundarySnapshot,
) -> TestResult {
    assert_eq!(journal.events_for_run_full(run)?, before_events);
    assert_eq!(runtime.inspect_run_state(run)?, before_state);
    assert_eq!(&runtime.pending_boundary_snapshot(16), before_pending);
    Ok(())
}

fn reopen_unchanged(
    runtime: Runtime,
    journal: Arc<FjallJournal>,
    db: &Path,
    rows: &[(String, Vec<u8>, Vec<u8>)],
    runs: &[RunId],
) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    drop(runtime);
    drop(journal);
    assert_eq!(raw_rows(db)?, rows);
    let (journal, mut runtime) = open(db)?;
    for run in runs {
        runtime.recover_and_resume(*run)?;
    }
    for run in runs {
        tick_pending(&mut runtime, *run)?;
    }
    Ok((journal, runtime))
}

fn events(journal: &FjallJournal, run: RunId) -> TestResult<Vec<JournalEvent>> {
    Ok(journal.events_for_run_full(run)?)
}

fn socket(dir: &Path) -> TestResult<(IpcServer, std::path::PathBuf)> {
    let path = Path::new("../../.evidence/omp")
        .join(dir.file_name().ok_or("missing scratch directory name")?)
        .join("command.sock");
    assert!(
        path.as_os_str().len() < 108,
        "Unix socket path exceeds sun_path"
    );
    Ok((IpcServer::bind(&path)?, path))
}

#[test]
fn complete_action_over_socket_persists_exact_numeric_outcome_after_cold_pending_recovery()
-> TestResult {
    // Given a durable pending Do with a genuine accepted contract and ticket.
    let dir = tempfile::Builder::new()
        .prefix("ipc-complete-")
        .tempdir_in("../../.evidence/omp")?;
    let run = RunId::new(6106);
    let (journal, mut runtime) = open(dir.path())?;
    submit_pending(&runtime, &journal, run)?;
    let ticket = tick_pending(&mut runtime, run)?;
    assert_eq!(ticket.run_attempt, 1);
    assert_eq!(ticket.step, StepIdx::new(1));
    drop(runtime);
    drop(journal);
    let (journal, mut runtime) = open(dir.path())?;
    runtime.recover_and_resume(run)?;
    let recovered = tick_pending(&mut runtime, run)?;
    assert_eq!(recovered, ticket);
    let (mut server, path) = socket(dir.path())?;
    let output = postcard::to_allocvec(&IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
    })?;
    // When the real Unix IPC server receives CompleteAction.
    let response = request(
        &mut server,
        &mut runtime,
        &path,
        IpcCommand::CompleteAction,
        61061,
        IpcPayload::CompleteAction {
            run_id: run,
            ticket: u64::from(ticket.step.get()),
            output,
        },
    )?;
    // Then its durable completion includes the literal output and a terminal result.
    assert_eq!(response, IpcResponse::AcceptedRun { run_id: run.get() });
    for _ in 0..8 {
        runtime.tick_all()?;
    }
    let recorded = events(&journal, run)?;
    let completed: Vec<_> = recorded
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope {
                ticket: actual,
                output,
                value,
                taint,
                ..
            } => Some((actual, output, value, taint)),
            _ => None,
        })
        .collect();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].0, &ticket);
    assert_eq!(completed[0].1, &SlotIdx::new(1));
    assert_eq!(
        postcard::from_bytes::<SlotValue>(completed[0].2)?,
        SlotValue::I64(42)
    );
    assert_eq!(completed[0].3, &Taint::Clean);
    assert_eq!(recorded.iter().filter(|event| matches!(event, JournalEvent::RunFinished { run: r, result, attempt: 1, .. } if *r == run && *result == SlotIdx::new(1))).count(), 1);
    assert_eq!(runtime.lookup_pending_action_ticket(run, 1), None);
    Ok(())
}

#[test]
fn fail_action_over_socket_records_nonretryable_terminal_failure() -> TestResult {
    // Given a pending action with strict durable admission.
    let dir = tempfile::Builder::new()
        .prefix("ipc-fail-")
        .tempdir_in("../../.evidence/omp")?;
    let run = RunId::new(6107);
    let (journal, mut runtime) = open(dir.path())?;
    submit_pending(&runtime, &journal, run)?;
    let ticket = tick_pending(&mut runtime, run)?;
    let (mut server, path) = socket(dir.path())?;
    // When FailAction carries the v1 opaque failure bytes.
    let response = request(
        &mut server,
        &mut runtime,
        &path,
        IpcCommand::FailAction,
        61071,
        IpcPayload::FailAction {
            run_id: run,
            ticket: u64::from(ticket.step.get()),
            error: b"failure".to_vec(),
        },
    )?;
    // Then the exact run is accepted and the terminal failure is durable exactly once.
    assert_eq!(response, IpcResponse::AcceptedRun { run_id: run.get() });
    for _ in 0..8 {
        runtime.tick_all()?;
    }
    let recorded = events(&journal, run)?;
    assert_eq!(recorded.iter().filter(|event| matches!(event, JournalEvent::ActionFailedEvent { run: r, step, action, attempt: 1, .. } if *r == run && *step == ticket.step && *action == ticket.action)).count(), 1);
    assert_eq!(
        recorded
            .iter()
            .filter(
                |event| matches!(event, JournalEvent::RunFailedEvent { run: r, .. } if *r == run)
            )
            .count(),
        1
    );
    assert_eq!(recorded.iter().filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { run: r, .. } if *r == run)).count(), 0);
    let trace = runtime.drain_trace();
    assert_eq!(trace.iter().filter(|event| matches!(event, vb_runtime::trace::TraceEvent::ActionFailed { run: r, step, code: vb_core::action::ActionFailureCode::Unknown } if *r == run && *step == ticket.step)).count(), 1);
    assert_eq!(trace.iter().filter(|event| matches!(event, vb_runtime::trace::TraceEvent::RunFailed { run: r } if *r == run)).count(), 1);
    assert_eq!(runtime.lookup_pending_action_ticket(run, 1), None);
    Ok(())
}

#[test]
fn inspect_cancel_and_invalid_action_commands_preserve_or_transition_durable_state() -> TestResult {
    // Given two independently suspended actions and a real Fjall database.
    let dir = tempfile::Builder::new()
        .prefix("ipc-control-")
        .tempdir_in("../../.evidence/omp")?;
    let (journal, mut runtime) = open(dir.path())?;
    let target = RunId::new(7107);
    let control = RunId::new(7108);
    submit_pending(&runtime, &journal, target)?;
    submit_pending(&runtime, &journal, control)?;
    let ticket = tick_pending(&mut runtime, target)?;
    let control_ticket = tick_pending(&mut runtime, control)?;
    drop(runtime);
    drop(journal);
    let before_rows = raw_rows(dir.path())?;
    let (journal, mut runtime) = open(dir.path())?;
    runtime.recover_and_resume(target)?;
    runtime.recover_and_resume(control)?;
    tick_pending(&mut runtime, target)?;
    tick_pending(&mut runtime, control)?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(target, 1),
        Some(ticket)
    );
    assert_eq!(
        runtime.lookup_pending_action_ticket(control, 1),
        Some(control_ticket)
    );
    let (mut server, path) = socket(dir.path())?;
    let before_events = events(&journal, target)?;
    let before_state = runtime.inspect_run_state(target)?.clone();
    let control_events = events(&journal, control)?;
    let control_state = runtime.inspect_run_state(control)?.clone();
    let before_pending = runtime.pending_boundary_snapshot(16);
    // When InspectRun is sent, it must return the addressed run without mutation.
    assert_eq!(
        request(
            &mut server,
            &mut runtime,
            &path,
            IpcCommand::InspectRun,
            71071,
            IpcPayload::InspectRun { run_id: target }
        )?,
        IpcResponse::Inspected {
            run_id: target.get()
        }
    );
    unchanged_after(
        &runtime,
        &journal,
        target,
        &before_events,
        &before_state,
        &before_pending,
    )?;
    let (new_journal, new_runtime) = reopen_unchanged(
        runtime,
        journal,
        dir.path(),
        &before_rows,
        &[target, control],
    )?;
    let (mut journal, mut runtime) = (new_journal, new_runtime);
    let output = postcard::to_allocvec(&IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
    })?;
    for (index, payload) in [
        IpcPayload::CompleteAction {
            run_id: target,
            ticket: u64::MAX,
            output: output.clone(),
        },
        IpcPayload::FailAction {
            run_id: RunId::new(9999),
            ticket: u64::from(ticket.step.get()),
            error: b"foreign".to_vec(),
        },
        IpcPayload::CompleteAction {
            run_id: target,
            ticket: u64::from(ticket.step.get()),
            output: vec![0xff],
        },
        IpcPayload::FailAction {
            run_id: target,
            ticket: u64::from(ticket.step.get()) + 1,
            error: b"wrong-step".to_vec(),
        },
        IpcPayload::InspectRun {
            run_id: RunId::new(9999),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let command = match payload {
            IpcPayload::FailAction { .. } => IpcCommand::FailAction,
            IpcPayload::InspectRun { .. } => IpcCommand::InspectRun,
            _ => IpcCommand::CompleteAction,
        };
        let response = request(
            &mut server,
            &mut runtime,
            &path,
            command,
            71100 + u64::try_from(index)?,
            payload,
        )?;
        let expected = match index {
            2 => IpcResponse::PayloadError {
                diagnostic: vb_ipc::IpcError::PayloadDecodeFailed
                    .diagnostic_code()
                    .code(),
                message: vb_ipc::IpcError::PayloadDecodeFailed.to_string(),
            },
            4 => IpcResponse::RuntimeError {
                message: String::from("run not found"),
            },
            _ => IpcResponse::BadRequest,
        };
        assert_eq!(response, expected);
        unchanged_after(
            &runtime,
            &journal,
            target,
            &before_events,
            &before_state,
            &before_pending,
        )?;
        let (new_journal, new_runtime) = reopen_unchanged(
            runtime,
            journal,
            dir.path(),
            &before_rows,
            &[target, control],
        )?;
        (journal, runtime) = (new_journal, new_runtime);
    }
    assert_eq!(
        wire(
            &mut server,
            &mut runtime,
            &path,
            IpcCommand::CancelRun,
            71111,
            &[0xff]
        )?,
        IpcResponse::BadRequest
    );
    unchanged_after(
        &runtime,
        &journal,
        target,
        &before_events,
        &before_state,
        &before_pending,
    )?;
    let (new_journal, new_runtime) = reopen_unchanged(
        runtime,
        journal,
        dir.path(),
        &before_rows,
        &[target, control],
    )?;
    (journal, runtime) = (new_journal, new_runtime);
    // When CancelRun targets only the first pending run.
    assert_eq!(
        request(
            &mut server,
            &mut runtime,
            &path,
            IpcCommand::CancelRun,
            71072,
            IpcPayload::CancelRun { run_id: target }
        )?,
        IpcResponse::AcceptedRun {
            run_id: target.get()
        }
    );
    for _ in 0..8 {
        runtime.tick_all()?;
    }
    let cancelled = events(&journal, target)?;
    assert_eq!(cancelled.iter().filter(|event| matches!(event, JournalEvent::RunCancelled { run, attempt: 1, .. } if *run == target)).count(), 1);
    assert_eq!(cancelled.iter().filter(|event| matches!(event, JournalEvent::ActionAbandoned { ticket: abandoned, .. } if *abandoned == ticket)).count(), 1);
    let abandoned_at = cancelled.iter().position(|event| matches!(event, JournalEvent::ActionAbandoned { ticket: abandoned, .. } if *abandoned == ticket)).ok_or("missing abandoned ticket")?;
    let cancelled_at = cancelled.iter().position(|event| matches!(event, JournalEvent::RunCancelled { run, attempt: 1, reason: None, .. } if *run == target)).ok_or("missing terminal cancellation")?;
    assert!(abandoned_at < cancelled_at);
    assert_eq!(runtime.lookup_pending_action_ticket(target, 1), None);
    assert_eq!(
        runtime.lookup_pending_action_ticket(control, 1),
        Some(control_ticket)
    );
    assert_eq!(events(&journal, control)?, control_events);
    assert_eq!(runtime.inspect_run_state(control)?, &control_state);
    assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    let trace = runtime.drain_trace();
    assert_eq!(trace.iter().filter(|event| matches!(event, vb_runtime::trace::TraceEvent::RunCancelled { run } if *run == target)).count(), 1);
    assert_eq!(trace.iter().filter(|event| matches!(event, vb_runtime::trace::TraceEvent::RunCancelled { run } if *run == control)).count(), 0);
    // A repeat request is acknowledged by v1, but cannot emit a second terminal effect.
    assert_eq!(
        request(
            &mut server,
            &mut runtime,
            &path,
            IpcCommand::CancelRun,
            71073,
            IpcPayload::CancelRun { run_id: target }
        )?,
        IpcResponse::AcceptedRun {
            run_id: target.get()
        }
    );
    for _ in 0..3 {
        runtime.tick_all()?;
    }
    assert_eq!(events(&journal, target)?, cancelled);
    assert_eq!(
        runtime.lookup_pending_action_ticket(control, 1),
        Some(control_ticket)
    );
    drop(runtime);
    drop(journal);
    drop(server);
    std::fs::remove_file(&path)?;
    let terminal_rows = raw_rows(dir.path())?;
    assert_ne!(terminal_rows, before_rows);
    let (mut server, reopened_path) = socket(dir.path())?;
    assert_eq!(path, reopened_path);
    let (journal, mut recovered) = open(dir.path())?;
    let refusal = recovered.recover_and_resume(target);
    assert!(
        matches!(
            refusal,
            Err(vb_runtime::RuntimeError::RecoveryCannotResume { .. })
        ),
        "{refusal:?}"
    );
    assert_eq!(events(&journal, target)?, cancelled);
    assert_eq!(
        request(
            &mut server,
            &mut recovered,
            &path,
            IpcCommand::InspectRun,
            71074,
            IpcPayload::InspectRun { run_id: target }
        )?,
        IpcResponse::RuntimeError {
            message: String::from("run not found")
        }
    );
    assert_eq!(
        request(
            &mut server,
            &mut recovered,
            &path,
            IpcCommand::FailAction,
            71075,
            IpcPayload::FailAction {
                run_id: target,
                ticket: u64::from(ticket.step.get()),
                error: b"duplicate".to_vec()
            }
        )?,
        IpcResponse::BadRequest
    );
    assert_eq!(events(&journal, target)?, cancelled);
    drop(server);
    drop(recovered);
    drop(journal);
    assert_eq!(raw_rows(dir.path())?, terminal_rows);
    Ok(())
}

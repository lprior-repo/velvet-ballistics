//! Binary IPC: submit an untimed WaitEvent, drop the server, and recover the
//! same pending wait without re-driving or fabricating a terminal state.

#![forbid(unsafe_code)]

use std::io::Write;
use std::time::Duration;

use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, StepState};
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};

use super::{Cleanup, HEADER, Resolver, encode_request, read_response, socket_path};
use crate::support::{Durability, TestResult, compile, events, open_runtime, persist_artifact};

#[test]
fn binary_ipc_wait_event_pending_cold_recovery() -> TestResult {
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: signal\n    set: {{output: event_id, value: '1'}}\n  - id: gate\n    wait: {{event: '$step.signal'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let workflow = compile(&yaml)?;
    let wait_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::WaitEvent { .. }))
        .map(|node| node.id)
        .ok_or("missing WaitEvent")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(304);
    let socket = socket_path("wait_recovery")?;
    let _cleanup = Cleanup(socket.clone());
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let mut server = IpcServer::bind(&socket)?;
    let mut resolver = Resolver::new(workflow.clone());
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(Duration::from_millis(100)))?;
    server.poll_once_with_resolver(
        &mut open.runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    )?;
    let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
        run_id: run,
        workflow: workflow.digest(),
        input: Vec::new(),
    });
    let bytes = postcard::to_allocvec(&payload)?;
    client.write_all(&encode_request(
        IpcCommand::SubmitRunInline,
        0x5151,
        &bytes,
    )?)?;
    client.flush()?;
    let (header, response) =
        read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(header.correlation, 0x5151);
    assert_eq!(
        response,
        Some(IpcResponse::AcceptedRun { run_id: run.get() })
    );
    for _ in 0..64 {
        open.runtime.tick_all()?;
        if open
            .runtime
            .inspect_run_state(run)?
            .frame
            .step_state(wait_step)?
            == StepState::Waiting
        {
            break;
        }
    }
    assert_eq!(
        open.runtime
            .inspect_run_state(run)?
            .frame
            .step_state(wait_step)?,
        StepState::Waiting
    );
    let baseline_events = events(&open.journal, run)?;
    drop(client);
    drop(server);
    drop(open);

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    assert_eq!(events(&open.journal, run)?, baseline_events);
    let product = open.runtime.recover_product(run)?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::OpenWaitEvent
            );
        }
        other => return Err(format!("IPC open wait recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    open.runtime.tick_all()?;
    let state = open.runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.pc(), wait_step);
    assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    let counters = open.runtime.counters_snapshot();
    assert_eq!(counters.runs_submitted, 0, "counters: {counters:?}");
    assert_eq!(counters.runs_completed, 0, "counters: {counters:?}");
    assert_eq!(counters.runs_failed, 0, "counters: {counters:?}");
    assert_eq!(counters.steps_executed, 0, "counters: {counters:?}");
    assert_eq!(
        events(&open.journal, run)?,
        baseline_events,
        "IPC recovery must not mutate the journal"
    );
    open.runtime.tick_all()?;
    assert_eq!(
        open.runtime
            .inspect_run_state(run)?
            .frame
            .step_state(wait_step)?,
        StepState::Waiting
    );
    assert_eq!(events(&open.journal, run)?, baseline_events);
    assert_eq!(open.runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

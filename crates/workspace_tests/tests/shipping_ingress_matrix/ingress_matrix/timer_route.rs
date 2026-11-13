//! Timed `WaitUntil` over the real Unix socket, dropped while suspended,
//! recovered as `WaitingTimer`, fired, and terminalized.

#![forbid(unsafe_code)]

use std::io::Write;
use std::time::Duration;

use vb_core::value::Taint;
use vb_core::{RunId, RuntimePolicy, SlotValue};
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};

use super::{Cleanup, HEADER, Resolver, encode_request, read_response, socket_path};
use crate::support::{
    Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[test]
fn binary_ipc_timed_wait_suspends_and_cold_recovers() -> TestResult {
    let yaml = format!(
        "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: deadline\n    set: {{output: delay, value: '0'}}\n  - id: gate\n    wait: {{timeout: '$step.deadline'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(402);
    let socket = socket_path("timer_route")?;
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
    let input = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(2)]))?;
    let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
        run_id: run,
        workflow: workflow.digest(),
        input,
    });
    let bytes = postcard::to_allocvec(&payload)?;
    client.write_all(&encode_request(
        IpcCommand::SubmitRunInline,
        0x6002,
        &bytes,
    )?)?;
    client.flush()?;
    let (_, response) = read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(
        response,
        Some(IpcResponse::AcceptedRun { run_id: run.get() })
    );
    let mut captured = None;
    for _ in 0..128 {
        open.runtime.tick_all()?;
        if let Ok(entry) = open.runtime.capture_timer_entry(run) {
            captured = Some(entry);
            break;
        }
    }
    captured.ok_or("timed wait did not register a timer")?;
    let baseline = events(&open.journal, run)?;
    drop(client);
    drop(server);
    drop(open);

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    assert_eq!(events(&open.journal, run)?, baseline);
    match open.runtime.recover_product(run)? {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::WaitingTimer
            );
        }
        other => return Err(format!("timed wait recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    let entry = {
        let mut found = None;
        for _ in 0..128 {
            open.runtime.tick_all()?;
            if let Ok(entry) = open.runtime.capture_timer_entry(run) {
                found = Some(entry);
                break;
            }
        }
        found.ok_or("recovered timer missing")?
    };
    open.runtime.timer_entry_fired(entry)?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(terminal(&durable, run)?.value, SlotValue::I64(42));
    assert_eq!(terminal(&durable, run)?.taint, Taint::Clean);
    assert_eq!(
        crate::support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::WaitScheduledEvent { run: actual, .. } if *actual == run
        )),
        1,
        "recovery must not duplicate WaitScheduled"
    );
    assert_ordering(&durable, run)?;
    Ok(())
}

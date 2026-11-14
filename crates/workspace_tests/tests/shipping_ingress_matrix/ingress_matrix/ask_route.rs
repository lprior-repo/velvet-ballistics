//! Ask over the real Unix socket, dropped while suspended, recovered as an
//! open ask, answered, and terminalized with taint.

#![forbid(unsafe_code)]

use std::io::Write;
use std::time::Duration;

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue, StepState};
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::shard::AskAnswer;

use super::{Cleanup, HEADER, Resolver, encode_request, read_response, socket_path};
use crate::support::{
    Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[test]
fn binary_ipc_ask_suspends_and_cold_recovers_open_ask() -> TestResult {
    let yaml = format!(
        "{HEADER}vars:\n  prompt_text: 'input?'\ninputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n  - id: question\n    ask: {{prompt: '$vars.prompt_text'}}\n  - id: done\n    finish: {{result: '$step.question'}}\n"
    );
    let workflow = compile(&yaml)?;
    let ask_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .map(|node| node.id)
        .ok_or("missing Ask")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(403);
    let socket = socket_path("ask_route")?;
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
        0x6003,
        &bytes,
    )?)?;
    client.flush()?;
    let (_, response) = read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(
        response,
        Some(IpcResponse::AcceptedRun { run_id: run.get() })
    );
    let ticket = crate::support::tick_until_ask(&mut open.runtime, run, ask_step, 64)?;
    assert_eq!(
        open.runtime
            .inspect_run_state(run)?
            .frame
            .step_state(ask_step)?,
        StepState::Asking
    );
    let baseline = events(&open.journal, run)?;
    drop(client);
    drop(server);
    drop(open);

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    assert_eq!(events(&open.journal, run)?, baseline);
    match open.runtime.recover_product(run)? {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        }
        other => return Err(format!("ask recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    open.runtime.tick_all()?;
    assert_eq!(
        open.runtime
            .inspect_run_state(run)?
            .frame
            .step_state(ask_step)?,
        StepState::Asking
    );
    assert_eq!(events(&open.journal, run)?, baseline);
    let recovered = open
        .runtime
        .lookup_pending_ask_ticket(run, ask_step)
        .ok_or("recovered ask ticket missing")?;
    open.runtime.answer_ask(AskAnswer::new(
        recovered,
        ticket.answer_slot(),
        SlotValue::I64(5),
        Taint::DerivedFromSecret,
    ))?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(5));
    assert_eq!(result.taint, Taint::DerivedFromSecret);
    assert_ordering(&durable, run)?;
    Ok(())
}

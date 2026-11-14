//! Action `do` over the real Unix socket with resolver-supplied contracts,
//! dropped while pending and completed through the production port.

#![forbid(unsafe_code)]

use std::io::Write;
use std::time::Duration;

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolver};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};

use super::{Cleanup, HEADER, Resolver, encode_request, read_response, socket_path};
use crate::support::{
    self, Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[path = "do_route/denial.rs"]
mod denial;

#[test]
fn binary_ipc_do_pending_action_cold_completes_once() -> TestResult {
    let yaml = format!(
        "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '100'}}\n  - id: act\n    do: {{action: '0', input: '$step.seed'}}\n  - id: done\n    finish: {{result: '$step.act'}}\n"
    );
    let workflow = compile(&yaml)?;
    let do_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or("missing Do")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(404);
    let socket = socket_path("do_route")?;
    let _cleanup = Cleanup(socket.clone());
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let mut resolver = Resolver::new(workflow.clone());
    let contracts = resolver.resolve_action_contracts(&workflow);
    assert_eq!(contracts.len(), 1);
    persist_artifact(
        &open.journal,
        &workflow,
        RuntimePolicy::Strict,
        contracts.as_ref(),
    )?;
    let mut server = IpcServer::bind(&socket)?;
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
        0x6004,
        &bytes,
    )?)?;
    client.flush()?;
    let (_, response) = read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(
        response,
        Some(IpcResponse::AcceptedRun { run_id: run.get() })
    );
    let ticket = support::tick_until_action(&mut open.runtime, run, u64::from(do_step.get()), 64)?;
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
                RecoveredRunBoundaryKind::PendingAction
            );
        }
        other => return Err(format!("do recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    open.runtime.tick_all()?;
    assert_eq!(events(&open.journal, run)?, baseline);
    let recovered = open
        .runtime
        .lookup_pending_action_ticket(run, u64::from(do_step.get()))
        .ok_or("recovered action ticket missing")?;
    assert_eq!(recovered, ticket);
    let output_slot = workflow
        .node(do_step)
        .and_then(|node| node.output.slot())
        .ok_or("Do output slot missing")?;
    let value = SlotValue::I64(100);
    let encoded_len = u32::try_from(postcard::to_allocvec(&value)?.len())?;
    open.runtime.complete_action_with_output(
        recovered,
        vb_core::action::ActionOutputReady {
            output_slot,
            value,
            taint: Taint::Clean,
            encoded_len,
        },
    )?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(
        support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::ActionCompletedEnvelope { ticket, .. }
                if ticket.run == run
        )),
        1
    );
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, value);
    assert_eq!(result.taint, Taint::Clean);
    assert_ordering(&durable, run)?;
    Ok(())
}

use std::io::{Read, Write};
use std::time::Duration;

use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue, WorkflowDigest};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_ipc::{
    IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, IpcPayload, MaxPayloadBytes, SubmitRunPayload,
};

use super::super::{Cleanup, HEADER, Resolver, encode_request, socket_path};
use crate::support::{Durability, TestResult, compile, events, open_runtime, persist_artifact};

struct NoGrantResolver(Resolver);

impl WorkflowResolver for NoGrantResolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<vb_core::CompiledWorkflow, WorkflowResolutionError> {
        self.0.resolve_workflow(digest)
    }

    fn resolve_action_contracts(
        &mut self,
        workflow: &vb_core::CompiledWorkflow,
    ) -> Box<[vb_core::action::ActionContract]> {
        self.0.resolve_action_contracts(workflow)
    }
}

#[test]
fn binary_ipc_do_without_host_grant_refuses_without_run_mutation() -> TestResult {
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
    let run = RunId::new(405);
    let socket = socket_path("do_denied")?;
    let _cleanup = Cleanup(socket.clone());
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let mut resolver = NoGrantResolver(Resolver::new(workflow.clone()));
    let contracts = resolver.resolve_action_contracts(&workflow);
    assert_eq!(contracts.len(), 1);
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &contracts)?;
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
        0x6005,
        &bytes,
    )?)?;
    client.flush()?;
    server.poll_once_with_resolver(
        &mut open.runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    )?;
    let mut header_bytes = [0; IPC_HEADER_LEN];
    client.read_exact(&mut header_bytes)?;
    let header = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)?;
    let payload_len = usize::try_from(header.payload_len)?;
    let mut response_bytes = vec![0; payload_len];
    client.read_exact(&mut response_bytes)?;
    let response = postcard::from_bytes::<IpcResponse>(&response_bytes)?;
    assert_eq!(header.correlation, 0x6005);
    assert_eq!(
        response,
        IpcResponse::RuntimeError {
            message: String::from("admission rejected: capability denied"),
        }
    );
    assert!(events(&open.journal, run)?.is_empty());
    assert!(open.journal.run_header(run)?.is_none());
    assert!(matches!(
        open.runtime.inspect_run_state(run),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ));
    assert!(
        open.runtime
            .lookup_pending_action_ticket(run, u64::from(do_step.get()))
            .is_none()
    );
    Ok(())
}

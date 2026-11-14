//! Binary IPC ingress matrix over a real Unix socket: SubmitRun,
//! SubmitRunInline, Health, Shutdown, durable journal, and cold replay.

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::path::{Component, PathBuf};
use std::time::Duration;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::value::Taint;
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, WorkflowDigest,
};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_ipc::{
    IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, IpcPayload, MaxPayloadBytes, SubmitRunPayload,
};

use super::support::{
    Durability, TestResult, compile, events, open_runtime, persist_artifact, terminal,
    tick_until_terminal,
};

#[path = "ingress_matrix/ask_route.rs"]
mod ask_route;
#[path = "ingress_matrix/do_route.rs"]
mod do_route;
#[path = "ingress_matrix/primitive_routes.rs"]
mod primitive_routes;
#[path = "ingress_matrix/timer_route.rs"]
mod timer_route;
#[path = "ingress_matrix/wait_recovery.rs"]
mod wait_recovery;

#[path = "ingress_matrix/submit_replay.rs"]
mod submit_replay;
const HEADER: &str = "version: velvet-ballistics/v1\nname: shipping_ipc\nwhen: {manual: {}}\n";

fn source() -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    )
}

struct Resolver {
    workflow: CompiledWorkflow,
    calls: usize,
}

impl Resolver {
    fn new(workflow: CompiledWorkflow) -> Self {
        Self { workflow, calls: 0 }
    }
}

impl WorkflowResolver for Resolver {
    fn resolve_action_contracts(&mut self, workflow: &CompiledWorkflow) -> Box<[ActionContract]> {
        let has_action_zero = workflow.to_parts().nodes.iter().any(|node| {
            matches!(
                node.kind,
                CompiledNodeKind::Do {
                    action,
                    ..
                } if action.get() == 0
            )
        });
        if !has_action_zero {
            return Box::from([]);
        }
        let Ok(name) = ActionName::new("shipping-matrix-action") else {
            return Box::from([]);
        };
        Box::from([ActionContract {
            id: vb_core::ids::ActionId::new(0),
            name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::from([vb_core::capability::Capability::new(
                "velvet.register.write".into(),
                vb_core::ids::ActionId::new(0),
            )]),
        }])
    }

    fn resolve_capability_grants(&mut self, workflow: &CompiledWorkflow) -> vb_core::CapabilitySet {
        let has_action_zero = workflow.to_parts().nodes.iter().any(|node| {
            matches!(
                node.kind,
                CompiledNodeKind::Do {
                    action,
                    ..
                } if action.get() == 0
            )
        });
        if has_action_zero {
            vb_core::CapabilitySet::from_grants(Box::from([vb_core::capability::Capability::new(
                "velvet.register.write".into(),
                vb_core::ids::ActionId::new(0),
            )]))
        } else {
            vb_core::CapabilitySet::empty()
        }
    }

    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        self.calls = self.calls.saturating_add(1);
        if digest == self.workflow.digest() {
            Ok(self.workflow.clone())
        } else {
            Err(WorkflowResolutionError::NotFound)
        }
    }
}

/// Builds a socket path relative to the current directory so the workspace
/// scratch location stays under `sun_path` without using `/tmp`.
fn socket_path(tag: &str) -> TestResult<PathBuf> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.evidence/omp/scratch");
    std::fs::create_dir_all(&target)?;
    let current = std::env::current_dir()?;
    let current_components: Vec<Component<'_>> = current.components().collect();
    let target_components: Vec<Component<'_>> = target.components().collect();
    let common = current_components
        .iter()
        .zip(&target_components)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..current_components.len() {
        relative.push("..");
    }
    for component in target_components.iter().skip(common) {
        relative.push(component.as_os_str());
    }
    relative.push(format!("ipc_{}_{tag}.sock", std::process::id()));
    Ok(relative)
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn encode_request(command: IpcCommand, correlation: u64, payload: &[u8]) -> TestResult<Vec<u8>> {
    Ok(vb_ipc::encode_frame(command, 0, correlation, payload)?)
}

fn read_response(
    server: &mut IpcServer,
    runtime: &mut vb_runtime::runtime::Runtime,
    resolver: &mut Resolver,
    client: &mut std::os::unix::net::UnixStream,
) -> TestResult<(IpcFrameHeader, Option<IpcResponse>)> {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    let mut polls = 0usize;
    loop {
        match client.read_exact(&mut header_bytes) {
            Ok(()) => break,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if polls >= 64 {
                    return Err("no IPC response within the bounded poll budget".into());
                }
                polls = polls.saturating_add(1);
                server.poll_once_with_resolver(
                    runtime,
                    Some(Duration::from_millis(10)),
                    Some(resolver),
                )?;
            }
            Err(error) => return Err(Box::new(error)),
        }
    }
    let header = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)?;
    let payload_len = usize::try_from(header.payload_len)?;
    let mut response = None;
    if payload_len > 0 {
        let mut payload = vec![0u8; payload_len];
        let mut polls = 0usize;
        loop {
            match client.read_exact(&mut payload) {
                Ok(()) => break,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    if polls >= 64 {
                        return Err("no IPC response payload within the bounded poll budget".into());
                    }
                    polls = polls.saturating_add(1);
                    server.poll_once_with_resolver(
                        runtime,
                        Some(Duration::from_millis(10)),
                        Some(resolver),
                    )?;
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
        response = Some(postcard::from_bytes::<IpcResponse>(&payload)?);
    }
    Ok((header, response))
}

#[test]
fn binary_ipc_rejects_unknown_workflow_digest() -> TestResult {
    let workflow = compile(&source())?;
    let dir = tempfile::tempdir()?;
    let socket = socket_path("unknown")?;
    let _cleanup = Cleanup(socket.clone());
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let mut server = IpcServer::bind(&socket)?;
    let mut resolver = Resolver::new(workflow);
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(Duration::from_millis(100)))?;
    server.poll_once_with_resolver(
        &mut open.runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    )?;
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(303),
        workflow: WorkflowDigest::from_bytes([0xAB; 32]),
        input: Vec::new(),
    });
    let bytes = postcard::to_allocvec(&payload)?;
    client.write_all(&encode_request(IpcCommand::SubmitRun, 7, &bytes)?)?;
    client.flush()?;
    let (_, response) = read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(response, Some(IpcResponse::WorkflowResolutionUnsupported));
    assert!(
        events(&open.journal, RunId::new(303))?.is_empty(),
        "rejected IPC submit must not write durable run events"
    );
    Ok(())
}

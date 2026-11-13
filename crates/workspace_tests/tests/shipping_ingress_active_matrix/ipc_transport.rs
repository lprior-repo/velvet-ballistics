use std::io::Read;
use std::path::{Component, PathBuf};
use std::time::Duration;

use vb_core::{CompiledWorkflow, RuntimePolicy};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_ipc::{IPC_HEADER_LEN, IpcFrameHeader, MaxPayloadBytes};

use super::super::support::TestResult;

pub(super) struct Resolver(pub(super) CompiledWorkflow);

impl WorkflowResolver for Resolver {
    fn resolve_action_contracts(
        &mut self,
        _: &CompiledWorkflow,
    ) -> Box<[vb_core::action::ActionContract]> {
        Box::new([])
    }

    fn resolve_capability_grants(&mut self, _: &CompiledWorkflow) -> vb_core::CapabilitySet {
        vb_core::CapabilitySet::empty()
    }

    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        if digest == self.0.digest() {
            Ok(self.0.clone())
        } else {
            Err(WorkflowResolutionError::NotFound)
        }
    }
}

pub(super) fn for_each_has_remaining(cursor: usize, item_count: usize) -> bool {
    cursor < item_count
}

pub(super) fn assert_exhausted_cursor_rejected(
    cursor: &[vb_core::SlotValue],
    source_len: usize,
) -> TestResult {
    let mut completed = cursor.to_vec();
    *completed.get_mut(1).ok_or("cursor has no position")? =
        vb_core::SlotValue::I64(i64::try_from(source_len)?);
    let position = match completed.get(1) {
        Some(vb_core::SlotValue::I64(position)) => usize::try_from(*position)?,
        _ => return Err("completed cursor has no position".into()),
    };
    assert!(
        !for_each_has_remaining(position, source_len),
        "a completed ForEach cursor must not qualify as an active cut"
    );
    Ok(())
}

pub(super) fn config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 1,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

pub(super) fn relative_socket_path(label: &str) -> TestResult<PathBuf> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.evidence/omp/scratch");
    std::fs::create_dir_all(&target)?;
    let current = std::env::current_dir()?;
    let current_parts: Vec<Component<'_>> = current.components().collect();
    let target_parts: Vec<Component<'_>> = target.components().collect();
    let common = current_parts
        .iter()
        .zip(&target_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..current_parts.len() {
        relative.push("..");
    }
    for component in target_parts.iter().skip(common) {
        relative.push(component.as_os_str());
    }
    relative.push(format!("active_ipc_{}_{}.sock", std::process::id(), label));
    Ok(relative)
}

pub(super) struct SocketCleanup(pub(super) PathBuf);

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub(super) fn read_response(
    server: &mut IpcServer,
    runtime: &mut vb_runtime::runtime::Runtime,
    resolver: &mut Resolver,
    client: &mut std::os::unix::net::UnixStream,
) -> TestResult<(IpcFrameHeader, Option<IpcResponse>)> {
    let mut header_bytes = [0; IPC_HEADER_LEN];
    let mut polls = 0;
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
                    return Err("IPC response exceeded bounded poll budget".into());
                }
                polls += 1;
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
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    if !payload.is_empty() {
        client.read_exact(&mut payload)?;
    }
    let response = if payload.is_empty() {
        None
    } else {
        Some(postcard::from_bytes(&payload)?)
    };
    Ok((header, response))
}

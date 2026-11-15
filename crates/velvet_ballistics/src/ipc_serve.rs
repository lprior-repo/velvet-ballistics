//! Module: ipc_serve

use crate::app_impl::prelude::*;
#[path = "ipc_serve/command.rs"]
mod command;
#[path = "ipc_serve/startup.rs"]
mod startup;
pub(crate) use command::cmd_ipc_serve;
#[cfg(test)]
use startup::recover_startup;

struct IpcJournalService {
    journal: Arc<vb_storage::FjallJournal>,
    queue: Arc<vb_storage::JournalWriterQueue>,
}

impl IpcJournalService {
    fn turn(
        &self,
        server: &mut vb_ipc::server::IpcServer,
        runtime: &mut vb_runtime::runtime::Runtime,
        resolver: &mut StorageWorkflowResolver,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, IpcServiceError> {
        self.flush_batch()?;
        if !server
            .poll_once_with_resolver(runtime, timeout, Some(resolver))
            .map_err(IpcServiceError::Ipc)?
        {
            return Ok(false);
        }
        let alive = runtime.tick_all().map_err(IpcServiceError::Runtime)?;
        self.flush_batch()?;
        Ok(alive || server.has_pending_responses())
    }

    fn flush_batch(&self) -> Result<(), IpcServiceError> {
        self.queue
            .flush_batch_durable(&self.journal)
            .map_err(IpcServiceError::Storage)?;
        Ok(())
    }
}

#[derive(Debug)]
enum IpcServiceError {
    Ipc(vb_ipc::server::IpcServerError),
    Runtime(vb_runtime::RuntimeError),
    Storage(vb_storage::JournalError),
}

impl std::fmt::Display for IpcServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ipc(error) => write!(f, "IPC: {error}"),
            Self::Runtime(error) => write!(f, "runtime: {error}"),
            Self::Storage(error) => write!(f, "storage: {error}"),
        }
    }
}

impl std::error::Error for IpcServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ipc(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "ipc_serve_tests.rs"]
mod tests;

struct StorageWorkflowResolver {
    journal: Arc<vb_storage::FjallJournal>,
    available_names: crate::run_compiled_runtime::secret_presence::AvailableNames,
}

impl vb_ipc::server::WorkflowResolver for StorageWorkflowResolver {
    fn prepare_runtime(
        &mut self,
        runtime: &mut vb_runtime::runtime::Runtime,
        workflow: &vb_core::CompiledWorkflow,
    ) {
        self.available_names.install(runtime, workflow);
    }

    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<vb_core::CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
        let record = compiled_record_for_digest(&self.journal, digest)?;
        decode_compiled_record(&record, digest)
    }
}

fn compiled_record_for_digest(
    journal: &vb_storage::FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<vb_storage::CompiledIrRecord, vb_ipc::server::WorkflowResolutionError> {
    match journal.compiled_ir(digest) {
        Ok(Some(record)) => Ok(record),
        Ok(None) => match journal.compiled_ir_for_source_digest(digest) {
            Ok(Some(record)) => Ok(record),
            Ok(None) => Err(vb_ipc::server::WorkflowResolutionError::NotFound),
            Err(_) => Err(vb_ipc::server::WorkflowResolutionError::InvalidArtifact),
        },
        Err(_) => Err(vb_ipc::server::WorkflowResolutionError::InvalidArtifact),
    }
}

fn decode_compiled_record(
    record: &vb_storage::CompiledIrRecord,
    requested: vb_core::WorkflowDigest,
) -> Result<vb_core::CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
    if raw_payload_digest(record) == record.digest {
        return decode_workflow_parts(record.ir.as_slice());
    }

    let artifact = postcard::from_bytes::<vb_storage::AcceptedArtifact>(record.ir.as_slice())
        .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)?;
    let artifact_digest = vb_storage::admission::accepted_artifact_digest(&artifact)
        .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)?;
    let digest_matches = artifact.digest == record.digest
        && artifact.verification.digest == record.digest
        && artifact_digest == record.digest;
    let lookup_matches = record.digest == requested || artifact.source_digest == requested;
    if !(digest_matches && lookup_matches) {
        return Err(vb_ipc::server::WorkflowResolutionError::InvalidArtifact);
    }
    decode_workflow_parts(artifact.ir.as_slice())
}

fn raw_payload_digest(record: &vb_storage::CompiledIrRecord) -> vb_core::WorkflowDigest {
    vb_core::WorkflowDigest::from_bytes(blake3::hash(record.ir.as_slice()).into())
}

fn decode_workflow_parts(
    bytes: &[u8],
) -> Result<vb_core::CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
    let parts = postcard::from_bytes::<vb_core::WorkflowParts>(bytes)
        .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)
}

// Proposed startup work ceilings cover retained headers and journal rows.
const STARTUP_HEADER_SCAN_LIMIT: usize = 65_536;

impl IpcJournalService {
    fn flush_startup_journal(&self) -> Result<(), vb_runtime::RuntimeError> {
        let counts = self.queue.pending_profile_counts()?;
        let pending = counts.journaled.checked_add(counts.strict).ok_or(
            vb_runtime::RuntimeError::UnsupportedOperation {
                operation: "startup_journal_count_overflow",
            },
        )?;
        for _ in 0..pending {
            if self.queue.flush_batch_durable(&self.journal)?.drained == 0 {
                break;
            }
        }
        let counts = self.queue.pending_profile_counts()?;
        if counts.journaled != 0 || counts.strict != 0 {
            return Err(vb_runtime::RuntimeError::UnsupportedOperation {
                operation: "startup_journal_not_drained",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "ipc_startup_recovery_tests.rs"]
mod startup_recovery_tests;

#[cfg(test)]
#[path = "ipc_restart_tests.rs"]
mod ipc_restart_tests;

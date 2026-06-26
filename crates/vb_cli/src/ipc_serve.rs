//! Module: ipc_serve

use crate::app_impl::prelude::*;

pub(crate) fn cmd_ipc_serve(socket: &std::path::Path, db: &std::path::Path) -> ExitCode {
    // Open the storage journal to validate the path
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return CliExitCode::IpcError.into();
        }
    };
    let journal = Arc::new(journal);
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&journal),
    };
    let queue =
        match vb_storage::JournalWriterQueue::new(1024, 64, vb_storage::StorageLimits::DEFAULT) {
            Ok(q) => Arc::new(q),
            Err(e) => {
                errln!("error creating journal queue: {e}");
                return CliExitCode::IpcError.into();
            }
        };
    let runtime_journal = match vb_runtime::journal::RuntimeJournalConfig::new(
        vb_storage::DurabilityProfile::Journaled,
    )
    .shared_journal(journal, queue)
    {
        Ok(j) => j,
        Err(e) => {
            errln!("error creating runtime journal: {e}");
            return CliExitCode::IpcError.into();
        }
    };

    // Create runtime
    let shard_count = match NonZeroUsize::new(1) {
        Some(count) => count,
        None => NonZeroUsize::MIN,
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, runtime_journal);

    // Bind the IPC server
    let mut server = match vb_ipc::server::IpcServer::bind(socket) {
        Ok(s) => s,
        Err(e) => {
            errln!("error binding IPC socket at {}: {e}", socket.display());
            return CliExitCode::IpcError.into();
        }
    };

    outln!("ipc server listening on {}", socket.display());

    // Event loop
    loop {
        match server.poll_once_with_resolver(
            &mut runtime,
            Some(std::time::Duration::from_millis(100)),
            Some(&mut resolver),
        ) {
            Ok(true) => {}
            Ok(false) => {
                outln!("shutdown requested");
                break;
            }
            Err(e) => {
                errln!("ipc server error: {e}");
                return CliExitCode::IpcError.into();
            }
        }

        // Process pending commands
        match runtime.tick_all() {
            Ok(true) => {}
            Ok(false) => {
                outln!("runtime shut down");
                break;
            }
            Err(e) => {
                errln!("runtime tick error: {e}");
                return CliExitCode::IpcError.into();
            }
        }
    }

    ExitCode::SUCCESS
}

struct StorageWorkflowResolver {
    journal: Arc<vb_storage::FjallJournal>,
}

impl vb_ipc::server::WorkflowResolver for StorageWorkflowResolver {
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

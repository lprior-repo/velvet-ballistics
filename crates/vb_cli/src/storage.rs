//! Storage and IPC commands for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::io::{errln, outln};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use vb_core::{CompiledWorkflow, WorkflowDigest, WorkflowParts};
use vb_ipc::server::{IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_runtime::journal::RuntimeJournalConfig;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

mod events;
pub use events::{event_name, print_event};

pub fn cmd_ipc_serve(socket: &Path, db: &Path) -> ExitCode {
    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };
    let journal = Arc::new(journal);
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&journal),
    };
    let queue = match JournalWriterQueue::new(1024, 64, StorageLimits::DEFAULT) {
        Ok(q) => Arc::new(q),
        Err(e) => {
            errln!("error creating journal queue: {e}");
            return ExitCode::FAILURE;
        }
    };
    let runtime_journal = match RuntimeJournalConfig::new(DurabilityProfile::Journaled)
        .shared_journal(journal, queue)
    {
        Ok(j) => j,
        Err(e) => {
            errln!("error creating runtime journal: {e}");
            return ExitCode::FAILURE;
        }
    };

    let shard_count = std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN);
    let config = ShardConfig::default();
    let mut runtime = Runtime::new_with_journal(shard_count, config, runtime_journal);

    let mut server = match IpcServer::bind(socket) {
        Ok(s) => s,
        Err(e) => {
            errln!("error binding IPC socket at {}: {e}", socket.display());
            return ExitCode::FAILURE;
        }
    };

    outln!("ipc server listening on {}", socket.display());

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
                return ExitCode::FAILURE;
            }
        }

        match runtime.tick_all() {
            Ok(true) => {}
            Ok(false) => {
                outln!("runtime shut down");
                break;
            }
            Err(e) => {
                errln!("runtime tick error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

pub struct StorageWorkflowResolver {
    pub journal: Arc<FjallJournal>,
}

impl WorkflowResolver for StorageWorkflowResolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        let record = compiled_record_for_digest(&self.journal, digest)?;
        decode_compiled_record(&record, digest)
    }
}

fn compiled_record_for_digest(
    journal: &FjallJournal,
    digest: WorkflowDigest,
) -> Result<vb_storage::CompiledIrRecord, WorkflowResolutionError> {
    match journal.compiled_ir(digest) {
        Ok(Some(record)) => Ok(record),
        Ok(None) => match journal.compiled_ir_for_source_digest(digest) {
            Ok(Some(record)) => Ok(record),
            Ok(None) => Err(WorkflowResolutionError::NotFound),
            Err(_) => Err(WorkflowResolutionError::InvalidArtifact),
        },
        Err(_) => Err(WorkflowResolutionError::InvalidArtifact),
    }
}

fn decode_compiled_record(
    record: &vb_storage::CompiledIrRecord,
    requested: WorkflowDigest,
) -> Result<CompiledWorkflow, WorkflowResolutionError> {
    if raw_payload_digest(record) == record.digest {
        return decode_workflow_parts(record.ir.as_slice());
    }

    let artifact = postcard::from_bytes::<vb_storage::AcceptedArtifact>(record.ir.as_slice())
        .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
    let artifact_digest = vb_storage::admission::accepted_artifact_digest(&artifact)
        .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
    let digest_matches = artifact.digest == record.digest
        && artifact.verification.digest == record.digest
        && artifact_digest == record.digest;
    let lookup_matches = record.digest == requested || artifact.source_digest == requested;
    if !(digest_matches && lookup_matches) {
        return Err(WorkflowResolutionError::InvalidArtifact);
    }
    decode_workflow_parts(artifact.ir.as_slice())
}

fn raw_payload_digest(record: &vb_storage::CompiledIrRecord) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(record.ir.as_slice()).into())
}

fn decode_workflow_parts(bytes: &[u8]) -> Result<CompiledWorkflow, WorkflowResolutionError> {
    let parts = postcard::from_bytes::<WorkflowParts>(bytes)
        .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| WorkflowResolutionError::InvalidArtifact)
}

pub fn cmd_inspect(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                outln!("run {run_id}: no events found");
            } else {
                let terminal = events.last();
                let status = match terminal {
                    Some(JournalEvent::RunFinished { .. }) => "finished",
                    Some(JournalEvent::RunFailedEvent { .. }) => "failed",
                    Some(JournalEvent::RunCancelled { .. }) => "cancelled",
                    _ => "running",
                };
                outln!("run {run_id}: status={status}, events={}", events.len());
            }
        }
        Err(e) => {
            errln!("error reading run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub fn cmd_events(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                outln!("no events found for run {run_id}");
            } else {
                for event in &events {
                    print_event(event);
                }
                outln!("{} event(s) total", events.len());
            }
        }
        Err(e) => {
            errln!("error reading events for run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            errln!("invalid run_id '{raw}': {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

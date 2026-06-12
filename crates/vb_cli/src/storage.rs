//! Storage and IPC commands for velvet-ballistics.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use vb_core::{WorkflowDigest, WorkflowParts};
use vb_ipc::server::{IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_runtime::journal::RuntimeJournalConfig;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

pub(crate) fn cmd_ipc_serve(socket: &Path, db: &Path) -> ExitCode {
    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::errln!("error opening journal at {}: {e}", db.display());
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
            crate::errln!("error creating journal queue: {e}");
            return ExitCode::FAILURE;
        }
    };
    let runtime_journal =
        RuntimeJournalConfig::new(DurabilityProfile::Journaled).shared_journal(journal, queue);

    let shard_count = std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN);
    let config = ShardConfig::default();
    let mut runtime = Runtime::new_with_journal(shard_count, config, runtime_journal);

    let mut server = match IpcServer::bind(socket) {
        Ok(s) => s,
        Err(e) => {
            crate::errln!("error binding IPC socket at {}: {e}", socket.display());
            return ExitCode::FAILURE;
        }
    };

    crate::outln!("ipc server listening on {}", socket.display());

    loop {
        match server.poll_once_with_resolver(
            &mut runtime,
            Some(std::time::Duration::from_millis(100)),
            Some(&mut resolver),
        ) {
            Ok(true) => {}
            Ok(false) => {
                crate::outln!("shutdown requested");
                break;
            }
            Err(e) => {
                crate::errln!("ipc server error: {e}");
                return ExitCode::FAILURE;
            }
        }

        match runtime.tick_all() {
            Ok(true) => {}
            Ok(false) => {
                crate::outln!("runtime shut down");
                break;
            }
            Err(e) => {
                crate::errln!("runtime tick error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

pub(crate) struct StorageWorkflowResolver {
    pub journal: Arc<FjallJournal>,
}

impl WorkflowResolver for StorageWorkflowResolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<vb_core::CompiledWorkflow, WorkflowResolutionError> {
        let record = match self.journal.compiled_ir(digest) {
            Ok(Some(record)) => record,
            Ok(None) => return Err(WorkflowResolutionError::NotFound),
            Err(_) => return Err(WorkflowResolutionError::InvalidArtifact),
        };
        if record.digest != digest {
            return Err(WorkflowResolutionError::InvalidArtifact);
        }
        let artifact = postcard::from_bytes::<vb_storage::AcceptedArtifact>(&record.ir)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
        if artifact.digest != digest {
            return Err(WorkflowResolutionError::InvalidArtifact);
        }
        let mut parts = postcard::from_bytes::<WorkflowParts>(&artifact.ir)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
        parts.digest = artifact.digest;
        vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)
    }
}

pub(crate) fn cmd_inspect(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                crate::outln!("run {run_id}: no events found");
            } else {
                let terminal = events.last();
                let status = match terminal {
                    Some(JournalEvent::RunFinished { .. }) => "finished",
                    Some(JournalEvent::RunFailedEvent { .. }) => "failed",
                    Some(JournalEvent::RunCancelled { .. }) => "cancelled",
                    _ => "running",
                };
                crate::outln!("run {run_id}: status={status}, events={}", events.len());
            }
        }
        Err(e) => {
            crate::errln!("error reading run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn cmd_events(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                crate::outln!("no events found for run {run_id}");
            } else {
                for event in &events {
                    print_event(event);
                }
                crate::outln!("{} event(s) total", events.len());
            }
        }
        Err(e) => {
            crate::errln!("error reading events for run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

// `print_event` and `event_name` were extracted to `storage_event_format.rs`
// to keep this file under the 300-line cap. The re-exports below
// preserve the existing call sites in `cmd_events` and `cmd_replay`.
pub(crate) use crate::storage_event_format::{event_name, print_event};

pub(crate) fn cmd_replay(run_id: &str, db: &Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => {
            crate::outln!("recovered {} event(s) for run {run_id}", events.len());
            for event in &events {
                print_event(event);
            }
            match vb_storage::recovery::extract_terminal(&events) {
                Some(terminal) => {
                    crate::outln!("terminal: {}", event_name(terminal));
                }
                None => {
                    crate::outln!("terminal: none");
                }
            }
        }
        Err(e) => {
            crate::errln!("error replaying run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}



fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            crate::errln!("invalid run_id '{raw}': {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

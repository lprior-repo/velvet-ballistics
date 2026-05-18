//! Storage and IPC commands for velvet-ballastics.
#![forbid(unsafe_code)]

use crate::io::{errln, outln};
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
    let runtime_journal = RuntimeJournalConfig::new(DurabilityProfile::Journaled)
        .shared_journal(journal, queue);

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
    ) -> Result<vb_core::CompiledWorkflow, WorkflowResolutionError> {
        let record = match self.journal.compiled_ir(digest) {
            Ok(Some(record)) => record,
            Ok(None) => return Err(WorkflowResolutionError::NotFound),
            Err(_) => return Err(WorkflowResolutionError::InvalidArtifact),
        };
        if record.digest != digest {
            return Err(WorkflowResolutionError::InvalidArtifact);
        }
        let parts = postcard::from_bytes::<WorkflowParts>(&record.ir)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
        vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)
    }
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

pub fn print_event(event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            outln!("  seq={}: RunAccepted", seq.get());
        }
        JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        JournalEvent::StepSucceeded { seq, step, output, .. } => {
            outln!("  seq={}: StepSucceeded step={} output={}", seq.get(), step.get(), output.get());
        }
        JournalEvent::ActionScheduled { seq, step, action, .. } => {
            outln!("  seq={}: ActionScheduled step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::ActionCompletedEvent { seq, step, action, .. } => {
            outln!("  seq={}: ActionCompleted step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::ActionFailedEvent { seq, step, action, .. } => {
            outln!("  seq={}: ActionFailed step={} action={}", seq.get(), step.get(), action.get());
        }
        JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        JournalEvent::RunCancelled { seq, .. } => {
            outln!("  seq={}: RunCancelled", seq.get());
        }
        JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  seq={}: RunFailed", seq.get());
        }
    }
}

pub fn cmd_replay(run_id: &str, db: &Path) -> ExitCode {
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

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => {
            outln!("recovered {} event(s) for run {run_id}", events.len());
            for event in &events {
                print_event(event);
            }
            match vb_storage::recovery::extract_terminal(&events) {
                Some(terminal) => {
                    outln!("terminal: {}", event_name(terminal));
                }
                None => {
                    outln!("terminal: none");
                }
            }
        }
        Err(e) => {
            errln!("error replaying run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

pub fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::RunAccepted { .. } => "RunAccepted",
        JournalEvent::StepStarted { .. } => "StepStarted",
        JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        JournalEvent::RunCancelled { .. } => "RunCancelled",
        JournalEvent::RunFinished { .. } => "RunFinished",
        JournalEvent::RunFailedEvent { .. } => "RunFailed",
    }
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

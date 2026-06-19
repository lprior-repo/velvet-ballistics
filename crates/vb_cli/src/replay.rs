#![forbid(unsafe_code)]
//! Run replay command.

mod failure;
mod report;

#[cfg(test)]
mod tests;

pub(crate) use failure::write_locked_read_surface;

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::file_io::{ensure_existing_journal_directory, parse_run_id, report_storage_open_error};
use std::process::ExitCode;

pub(crate) fn cmd_replay(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    if let Err(code) = ensure_existing_journal_directory(db, output) {
        return code;
    }

    let journal = match open_replay_journal(db, run_id, output) {
        Ok(journal) => journal,
        Err(code) => return code,
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => report::write_replay_success(run_id, &events, output),
        Err(error) => {
            let context_events = failure::replay_context_events(&journal, rid);
            failure::write_replay_error(run_id, &error, context_events.as_deref(), output)
        }
    }
}

fn open_replay_journal(
    db: &std::path::Path,
    run_id: &str,
    output: OutputFormat,
) -> Result<vb_storage::FjallJournal, ExitCode> {
    match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => Ok(journal),
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => {
            Err(write_locked_read_surface("replay", run_id, output))
        }
        Err(error) => {
            report_storage_open_error(&error, db, output);
            Err(CliExitCode::StorageError.into())
        }
    }
}

fn write_vb_kyyf_trace(command: &str, run_id: &str, events_len: u64) {
    crate::outln!(
        "BDD-KYYF-002 command={command} run_id={run_id} evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events={events_len}"
    );
}

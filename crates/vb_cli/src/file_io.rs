#![forbid(unsafe_code)]
//! File reading and parsing utilities.

fn read_file(
    path: &std::path::Path,
    output: OutputFormat,
    exit_code: CliExitCode,
) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            let message = format!("error reading {}: {e}", path.display());
            write_failure_message(&message, output, exit_code);
            Err(exit_code.into())
        }
    }
}

fn write_failure_message(message: &str, output: OutputFormat, exit_code: CliExitCode) {
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        write_diagnostic_message_stderr(message, exit_code, output);
    }
}

fn parse_run_id(raw: &str, output: OutputFormat) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => {
            if id == 0 {
                write_failure_message(
                    &format!("invalid run_id '{raw}': run_id must be non-zero"),
                    output,
                    CliExitCode::ValidationFailed,
                );
                return Err(CliExitCode::ValidationFailed.into());
            }
            Ok(vb_core::RunId::new(id))
        }
        Err(e) => {
            write_failure_message(
                &format!("invalid run_id '{raw}': {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

fn report_storage_open_error(
    e: &vb_storage::JournalError,
    db: &std::path::Path,
    output: OutputFormat,
) {
    let message = format!("error opening journal at {}: {e}", db.display());
    if output != OutputFormat::Text {
        write_failure_message(&message, output, CliExitCode::StorageError);
    } else {
        errln!("{message}");
    }
}

fn read_journal_events(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<Vec<vb_storage::JournalEvent>, ExitCode> {
    let rid = parse_run_id(run_id, output)?;
    if !db.exists() {
        let msg = format!("journal directory does not exist: {}", db.display());
        if output != OutputFormat::Text {
            write_failure_message(&msg, output, CliExitCode::StorageError);
        } else {
            errln!("{msg}");
        }
        return Err(CliExitCode::StorageError.into());
    }
    let journal = vb_storage::FjallJournal::open(db, None).map_err(|e| -> ExitCode {
        report_storage_open_error(&e, db, output);
        CliExitCode::StorageError.into()
    })?;
    journal.events_for_run(rid).map_err(|e| {
        if output != OutputFormat::Text {
            write_failure_message(
                &format!("error reading run {run_id}: {e}"),
                output,
                CliExitCode::StorageError,
            );
        } else {
            errln!("error reading run {run_id}: {e}");
        }
        CliExitCode::StorageError.into()
    })
}

// --- Command implementations ---


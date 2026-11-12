use std::sync::Arc;
use vb_core::RunId;
use vb_storage::{FjallJournal, OwnedWriterError};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn open() -> TestResult<(tempfile::TempDir, Arc<FjallJournal>)> {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.probe_health()?;
    Ok((directory, journal))
}

pub(super) fn refused(
    journal: &Arc<FjallJournal>,
    run: u64,
    capacity: usize,
) -> TestResult<OwnedWriterError> {
    journal
        .try_acquire_owned_writer(RunId::new(run), capacity)
        .err()
        .ok_or_else(|| "unexpected registry owner granted".into())
}

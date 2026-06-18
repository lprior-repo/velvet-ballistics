//! Snapshot retrieval from the journal and event trail.

#![forbid(unsafe_code)]

pub(super) fn latest_snapshot_for_run(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    events: &[vb_storage::JournalEvent],
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    latest_snapshot_from_events(events, |seq| journal.snapshot(run, seq))
}

pub(super) fn latest_snapshot_from_events(
    events: &[vb_storage::JournalEvent],
    mut snapshot_at: impl FnMut(
        vb_storage::EventSeq,
    )
        -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError>,
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    events.iter().rev().try_fold(None, |found, event| {
        if found.is_some() {
            Ok(found)
        } else {
            snapshot_at(event.seq())
        }
    })
}

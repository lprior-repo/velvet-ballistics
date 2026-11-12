use super::latest_snapshot_from_events;
use vb_storage::{EventSeq, JournalError, JournalEvent};

#[test]
fn ai_context_latest_snapshot_from_events_propagates_snapshot_lookup_error() -> Result<(), String> {
    let run = vb_core::RunId::new(9);
    let events = [JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([1; 32]),
    }];

    let result = latest_snapshot_from_events(&events, |_| Err(JournalError::WriteLockPoisoned));

    match result {
        Err(JournalError::WriteLockPoisoned) => Ok(()),
        Err(e) => Err(format!("expected WriteLockPoisoned, got {e:?}")),
        Ok(v) => Err(format!("expected Err(WriteLockPoisoned), got Ok({v:?})")),
    }
}

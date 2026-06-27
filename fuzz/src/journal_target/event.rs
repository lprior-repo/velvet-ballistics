//! Journal event codec and replay fuzz target bodies.

use super::MAX_FUZZ_PAYLOAD;
use super::errors::{assert_typed_journal_error, assert_typed_recovery_error};

pub fn fuzz_journal_event(data: &[u8]) {
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        MAX_FUZZ_PAYLOAD,
    );
    match decoded {
        Ok((_envelope, event)) => {
            assert!(event.is_valid(), "Decoded event must be structurally valid");
            let Ok(encoded) = vb_storage::encode_record(
                vb_storage::MAGIC_JOURNAL_EVENT,
                event.record_kind(),
                event.seq().get(),
                &event,
                MAX_FUZZ_PAYLOAD,
            ) else {
                return;
            };
            let reparsed = vb_storage::decode_record::<vb_storage::JournalEvent>(
                &encoded,
                vb_storage::MAGIC_JOURNAL_EVENT,
                MAX_FUZZ_PAYLOAD,
            );
            assert!(
                reparsed.is_ok(),
                "Round-trip encode/decode must succeed for valid event"
            );
        }
        Err(error) => assert_typed_journal_error(error),
    }
}

pub fn fuzz_replay_events(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    if events.is_empty() {
        return;
    }
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    let result = vb_storage::recovery::replay_events(&events, &mut tracker, &[]);
    match result {
        Ok(replayed) => {
            assert!(
                replayed.len() <= events.len(),
                "replayed {} events must not exceed input {} events",
                replayed.len(),
                events.len()
            );
            for event in &replayed {
                if let vb_storage::JournalEvent::ActionCompletedEvent { action, step, .. } = event {
                    assert!(
                        tracker.has_completed(*action, *step),
                        "ActionCompletedEvent must be tracked as completed"
                    );
                }
                if let vb_storage::JournalEvent::ActionFailedEvent { action, step, .. } = event {
                    assert!(
                        tracker.has_failed(*action, *step),
                        "ActionFailedEvent must be tracked as failed"
                    );
                }
            }
        }
        Err(e) => assert_typed_recovery_error(e),
    }
}

pub fn fuzz_extract_terminal(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    let terminal = vb_storage::recovery::extract_terminal(&events);
    if let Some(event) = terminal {
        assert!(
            matches!(
                event,
                vb_storage::JournalEvent::RunFinished { .. }
                    | vb_storage::JournalEvent::RunFailedEvent { .. }
                    | vb_storage::JournalEvent::RunCancelled { .. }
            ),
            "terminal event must be a terminal kind, got {:?}",
            event.record_kind()
        );
    }
}

pub fn fuzz_action_tracker(data: &[u8]) {
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    for chunk in data.chunks_exact(3).take(64) {
        let Some(mode) = chunk.first().copied() else {
            continue;
        };
        let Some(action) = chunk.get(1).copied() else {
            continue;
        };
        let Some(step) = chunk.get(2).copied() else {
            continue;
        };
        let action = vb_core::ActionId::new(u16::from(action));
        let step = vb_core::StepIdx::new(u16::from(step));
        match mode % 3 {
            0 => {
                let was_resolved = tracker.is_resolved(action, step);
                tracker.mark_completed(action, step);
                assert!(tracker.is_resolved(action, step));
                assert!(tracker.has_completed(action, step));
                let _ = was_resolved;
            }
            1 => {
                let was_resolved = tracker.is_resolved(action, step);
                tracker.mark_failed(action, step);
                assert!(tracker.is_resolved(action, step));
                assert!(tracker.has_failed(action, step));
                let _ = was_resolved;
            }
            _ => {
                let first = tracker.is_resolved(action, step);
                let second = tracker.is_resolved(action, step);
                assert_eq!(first, second);
            }
        }
    }
}

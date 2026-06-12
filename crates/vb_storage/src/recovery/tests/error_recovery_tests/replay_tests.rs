use super::sample_event;
use crate::JournalEvent;
use crate::codec::{decode_journal_event, encode_journal_event_record};
use crate::constants::MAGIC_JOURNAL_EVENT;
use vb_core::RunId;

#[test]
fn decode_accepts_duplicate_sequence_but_replay_rejects() {
    let event1 = sample_event();
    let event2 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        step: vb_core::StepIdx::ZERO,
        attempt: 1,
    };
    let bytes1 = encode_journal_event_record(&event1).expect("event1 encodes");
    let bytes2 = encode_journal_event_record(&event2).expect("event2 encodes");
    let (env1, _) =
        decode_journal_event(&bytes1, MAGIC_JOURNAL_EVENT, 65_536).expect("event1 decodes");
    let (env2, _) =
        decode_journal_event(&bytes2, MAGIC_JOURNAL_EVENT, 65_536).expect("event2 decodes");
    assert_eq!(
        env1.sequence, env2.sequence,
        "duplicate sequence must be observable in envelope"
    );

    let mut tracker = crate::recovery::ActionReplayTracker::default();
    let events = vec![event1, event2];
    let err = crate::recovery::replay_events(&events, &mut tracker, &[])
        .expect_err("replay across duplicate seq must fail");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("ReplayDivergence")
            || msg.contains("SequenceGap")
            || msg.contains("StepOrder"),
        "duplicate sequence replay must surface typed replay error, got {msg}"
    );
}

#[test]
fn replay_rejects_gap_in_sequence() {
    let event1 = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
    };
    let event2 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(2),
        step: vb_core::StepIdx::ZERO,
        attempt: 1,
    };
    let mut tracker = crate::recovery::ActionReplayTracker::default();
    let err = crate::recovery::replay_events(&[event1, event2], &mut tracker, &[])
        .expect_err("gap in sequence must fail replay");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("ReplayDivergence")
            || msg.contains("SequenceGap")
            || msg.contains("StepOrder"),
        "gap in sequence must surface typed replay error, got {msg}"
    );
}

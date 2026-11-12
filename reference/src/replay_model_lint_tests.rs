use super::{ReplayModel, ReplayPhase, ReplayState};
use vb_core::{RunId, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

fn event(run: u64, seq: u64, digest: u8) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([digest; 32]),
    }
}

fn events(sequences: &[u64]) -> Vec<JournalEvent> {
    sequences.iter().map(|seq| event(1, *seq, 0)).collect()
}

fn assert_state(state: &ReplayState, phase: &ReplayPhase, cursor: usize, events: &[JournalEvent]) {
    assert_eq!(&state.phase, phase);
    assert_eq!(state.last_applied_event_index, cursor);
    assert_eq!(state.events, events);
}

#[test]
fn replay_next_preserves_events_and_returns_exact_order() {
    let original = events(&[0, 1]);
    let mut state = ReplayState::new(original.clone());
    assert_eq!(state.replay_next(), Some(event(1, 0, 0)));
    assert_state(&state, &ReplayPhase::Replay, 1, &original);
    assert_eq!(state.is_replay_complete(), false);
    assert_eq!(state.replay_next(), Some(event(1, 1, 0)));
    assert_state(&state, &ReplayPhase::Replay, 2, &original);
    assert_eq!(state.replay_next(), None);
    assert_eq!(state.replay_next(), None);
    assert_state(&state, &ReplayPhase::Replay, 2, &original);
    assert_eq!(state.is_replay_complete(), true);
}

#[test]
fn empty_replay_does_not_advance_or_change_phase() {
    let mut state = ReplayState::new(vec![]);
    assert_eq!(state.replay_next(), None);
    assert_state(&state, &ReplayPhase::Replay, 0, &[]);
    assert_eq!(state.is_replay_complete(), true);
}

#[test]
fn public_exhausted_and_oversized_cursors_stay_unchanged() {
    [1, 2, usize::MAX].into_iter().for_each(|cursor| {
        let original = events(&[0]);
        let mut state = ReplayState::new(original.clone());
        state.last_applied_event_index = cursor;
        assert_eq!(state.replay_next(), None);
        assert_eq!(state.replay_next(), None);
        assert_state(&state, &ReplayPhase::Replay, cursor, &original);
        assert_eq!(state.is_replay_complete(), true);
    });
}

#[test]
fn live_phase_keeps_existing_phase_independent_replay_behavior() {
    let original = events(&[u64::MAX]);
    let mut state = ReplayState::new(original.clone());
    state.phase = ReplayPhase::Live;
    assert_eq!(state.replay_next(), Some(event(1, u64::MAX, 0)));
    assert_state(&state, &ReplayPhase::Live, 1, &original);
    assert_eq!(state.replay_next(), None);
    assert_state(&state, &ReplayPhase::Live, 1, &original);
}

fn assert_sequence_case(snapshot: &[u64], tail: &[u64], full: &[u64], expected: bool) {
    assert_eq!(
        ReplayModel::snapshot_plus_tail_equals_full(
            &events(snapshot),
            &events(tail),
            &events(full)
        ),
        expected
    );
}

#[test]
fn snapshot_tail_equality_checks_complete_lengths_order_and_duplicates() {
    let cases: &[(&[u64], &[u64], &[u64], bool)] = &[
        (&[], &[], &[], true),
        (&[0], &[], &[0], true),
        (&[], &[0], &[0], true),
        (&[0, 1], &[2], &[0, 1, 2], true),
        (&[0, 1], &[2], &[0, 1], false),
        (&[0, 1], &[2], &[0, 1, 2, 3], false),
        (&[0], &[2, 1], &[0, 1, 2], false),
        (&[0], &[0], &[0, 0], true),
        (&[0], &[0], &[0], false),
        (&[], &[1], &[], false),
    ];
    cases.iter().for_each(|(snapshot, tail, full, expected)| {
        assert_sequence_case(snapshot, tail, full, *expected);
    });
}

#[test]
fn snapshot_tail_comparison_preserves_full_event_identity() {
    [event(2, 0, 0), event(1, 0, 1)]
        .into_iter()
        .for_each(|different| {
            assert_eq!(
                ReplayModel::snapshot_plus_tail_equals_full(&[event(1, 0, 0)], &[], &[different]),
                false
            );
        });
}

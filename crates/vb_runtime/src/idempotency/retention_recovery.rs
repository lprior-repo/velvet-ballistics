use super::IdempotencyTracker;
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, StepIdx};

fn ticket(key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
        fencing_token: 0,
    }
}

#[test]
fn completion_order_retains_only_the_bounded_live_fifo() {
    for capacity in [0, 1, 2, 1024] {
        let mut tracker = IdempotencyTracker::new(capacity);
        let limit = capacity.max(1);
        for key in 0..4096u128 {
            assert_eq!(tracker.mark_completed(&ticket(key)), Ok(()));
            assert!(
                tracker.order.len() <= limit,
                "capacity={capacity}, key={key}"
            );
            assert_eq!(tracker.order.len(), tracker.len());
            assert!(
                tracker
                    .order
                    .iter()
                    .all(|key| tracker.completed.contains_key(key))
            );
            let mut expected: Vec<_> = (0..=key).rev().take(limit).collect();
            expected.reverse();
            assert_eq!(tracker.order.iter().copied().collect::<Vec<_>>(), expected);
        }
    }
}

#[test]
fn repeated_evicted_keys_and_clone_keep_bounded_independent_order() {
    let mut tracker = IdempotencyTracker::new(2);
    for key in (0..3).cycle().take(4096) {
        assert_eq!(tracker.mark_completed(&ticket(key)), Ok(()));
        assert_eq!(tracker.order.len(), tracker.len());
        assert!(tracker.order.len() <= 2);
    }
    let mut cloned = tracker.clone();
    let previous = tracker.order.iter().copied().collect::<Vec<_>>();
    assert_eq!(cloned.mark_completed(&ticket(99)), Ok(()));
    assert_eq!(tracker.order.iter().copied().collect::<Vec<_>>(), previous);
    assert_eq!(cloned.order.len(), 2);
    assert!(!tracker.is_completed(&ticket(99)));
}

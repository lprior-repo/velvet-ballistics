#[cfg(test)]
mod unit_tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    fn make_ticket(seq: u32) -> ActionTicket {
        ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(u64::from(seq)),
            action: ActionId::new(1),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: seq as u128,
            capacity: 1,
            fencing_token: 0,
        }
    }

    #[test]
    fn bounded_action_queue_new_with_capacity_stores_capacity() {
        let queue = BoundedActionCompletionQueue::new(10).unwrap();
        assert_eq!(queue.capacity(), 10);
    }

    #[test]
    fn bounded_action_queue_new_is_empty() {
        let queue = BoundedActionCompletionQueue::new(5).unwrap();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn bounded_action_queue_new_with_zero_capacity_returns_error() {
        let result = BoundedActionCompletionQueue::new(0);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: 0,
                reason: InvalidActionQueueCapacity::Zero,
            })
        ));
    }

    #[test]
    fn bounded_action_queue_new_with_max_capacity_succeeds() {
        let queue =
            BoundedActionCompletionQueue::new(MAX_ACTION_COMPLETION_QUEUE_CAPACITY).unwrap();
        assert_eq!(queue.capacity(), MAX_ACTION_COMPLETION_QUEUE_CAPACITY);
    }

    #[test]
    fn bounded_action_queue_new_above_max_capacity_returns_error() {
        let requested = MAX_ACTION_COMPLETION_QUEUE_CAPACITY + 1;
        let result = BoundedActionCompletionQueue::new(requested);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: value,
                reason: InvalidActionQueueCapacity::AboveMaximum {
                    maximum,
                },
            }) if value == requested && maximum == MAX_ACTION_COMPLETION_QUEUE_CAPACITY
        ));
    }

    #[test]
    fn bounded_action_queue_with_backpressure_rejects_zero_capacity() {
        let result = BoundedActionCompletionQueue::with_backpressure(0);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: 0,
                reason: InvalidActionQueueCapacity::Zero,
            })
        ));
    }

    #[test]
    fn bounded_action_queue_with_backpressure_rejects_above_max_capacity() {
        let requested = MAX_ACTION_COMPLETION_QUEUE_CAPACITY + 1;
        let result = BoundedActionCompletionQueue::with_backpressure(requested);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: value,
                reason: InvalidActionQueueCapacity::AboveMaximum { maximum },
            }) if value == requested && maximum == MAX_ACTION_COMPLETION_QUEUE_CAPACITY
        ));
    }

    #[test]
    fn bounded_action_queue_preallocates_vecdeque_to_validated_capacity() {
        let queue = BoundedActionCompletionQueue::new(13).unwrap();
        let inner = queue.inner.lock().unwrap();
        assert_eq!(inner.items.capacity(), 13);
    }

    #[test]
    fn bounded_action_queue_enqueue_single_item_succeeds() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        let ticket = make_ticket(0);
        let result = queue.enqueue(ticket);
        assert!(result.is_ok());
    }

    #[test]
    fn bounded_action_queue_enqueue_at_capacity_returns_queue_full_error() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        for i in 0..3 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let result = queue.enqueue(make_ticket(100));
        assert!(matches!(
            result,
            Err(ActionQueueError::QueueFull { capacity }) if capacity.get() == 3
        ));
    }

    #[test]
    fn bounded_action_queue_dequeue_from_empty_returns_none() {
        let queue = BoundedActionCompletionQueue::new(4).unwrap();
        let result = queue.dequeue();
        assert_eq!(result, None);
    }

    #[test]
    fn bounded_action_queue_dequeue_returns_fifo_order() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        queue.enqueue(make_ticket(2)).unwrap();

        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_decrements_after_enqueue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        assert_eq!(queue.remaining_capacity(), 8);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_increments_after_dequeue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        assert_eq!(queue.remaining_capacity(), 6);
        let _ = queue.dequeue();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity() {
        let queue = BoundedActionCompletionQueue::new(7).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        let _ = queue.dequeue();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
    }

    // =============================================================================
    // Group F: Action Queue Backpressure (POST-006, INV-005)
    // Scenario F1: Queue full returns error — covered by
    // bounded_action_queue_enqueue_at_capacity_returns_queue_full_error
    // Scenario F2: 80% capacity triggers backpressure warning — tested below
    // Scenario F3: 79% does not trigger backpressure — tested below
    // Scenario F4: Invariant — len never exceeds capacity — tested below
    // =============================================================================

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity() {
        // Given: capacity=10, backpressure threshold = (10*8)/10 = 8 (80%)
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%) — no warning expected
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(7)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=8, cap=10)"
        );
    }

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity_var_20() {
        // Given: capacity=20, backpressure threshold = (20*8)/10 = 16 (80%)
        let capacity = 20;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (75%) — no warning expected
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 16th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(15)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 16,
                capacity: 20
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=16, cap=20)"
        );
    }

    #[test]
    fn action_queue_no_warning_before_80_percent_capacity() {
        // Given: capacity=10, threshold=8
        // At depth=7 (70%), no warning should fire
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%)
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // No backpressure warning at 70% (7/10 = 70% < 80%)
        assert_eq!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty),
            "no backpressure warning at 70% capacity (7/10)"
        );

        // Queue state is correct
        assert_eq!(queue.len(), 7);
        assert_eq!(queue.remaining_capacity(), 3);
    }

    #[test]
    fn action_queue_no_warning_at_79_percent_capacity() {
        // Given: capacity=19, threshold = ceil(19 * 8 / 10) = ceil(15.2) = 16.
        // At depth=15: 15/19 = 78.9% < 80%, so no warning.
        // At depth=16: 16/19 = 84.2% >= 80%, warning fires.
        // Regression: previously the floor-division bug made threshold=15, which
        // produced a misleading warning at 78.9% (RP-019 / bead vb-lxkqh).
        let capacity = 19;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (78.9%) — below the ceiling-of-80% threshold of 16.
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // Depth=15 < threshold=16 → no warning should be emitted.
        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout),
            "no warning should fire below the ceiling-of-80% threshold (depth=15, threshold=16)"
        );
    }

    #[test]
    fn action_queue_backpressure_warning_fires_once_per_enqueue_at_threshold() {
        // Verify warning fires on EACH enqueue that crosses or meets threshold
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue to 7 (70%) — no warning
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th (80%) — warning fires
        queue.enqueue(make_ticket(7)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );

        // Enqueue 9th (90%) — another warning fires (depth=9 >= threshold=8)
        queue.enqueue(make_ticket(8)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 9,
                capacity: 10
            })
        );

        // Drain and verify no more warnings after drain
        let _ = queue.dequeue();
        let _ = queue.dequeue();
        assert_eq!(queue.len(), 7);

        // Next enqueue to 8 (back at 80%) should also fire warning
        queue.enqueue(make_ticket(100)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );
    }

    #[test]
    fn action_queue_invariant_len_never_exceeds_capacity() {
        // INV-005: Bounded queue capacity invariant
        let capacities = [1, 2, 3, 5, 7, 10, 16, 100];
        for cap in capacities {
            let queue = BoundedActionCompletionQueue::new(cap).unwrap();
            assert_eq!(
                queue.len() <= queue.capacity(),
                true,
                "len() <= capacity() must hold at all times for capacity={}",
                cap
            );
            assert_eq!(
                queue.len() + queue.remaining_capacity() == queue.capacity(),
                true,
                "len() + remaining_capacity() == capacity() must hold for capacity={}",
                cap
            );

            // Exhaust the queue
            for i in 0..cap {
                queue
                    .enqueue(make_ticket(u32::try_from(i).expect("capacity fits in u32")))
                    .unwrap();
                assert_eq!(
                    queue.len() <= queue.capacity(),
                    true,
                    "len() must not exceed capacity after enqueue {} for capacity={}",
                    i + 1,
                    cap
                );
            }

            // One more should fail
            assert_eq!(
                queue.enqueue(make_ticket(255)),
                Err(ActionQueueError::QueueFull {
                    capacity: ActionQueueCapacity(cap)
                }),
                "enqueue at capacity must return QueueFull for capacity={}",
                cap
            );
            assert_eq!(
                queue.len(),
                cap,
                "len must still equal capacity after rejected enqueue for capacity={}",
                cap
            );
        }
    }

    #[test]
    fn action_queue_dequeue_returns_fifo_order() {
        // INV-004: FIFO ordering invariant
        let queue = BoundedActionCompletionQueue::new(5).unwrap();

        // Enqueue with sequential seq numbers
        for i in 0..5 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // Dequeue must return in FIFO order
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(3));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(4));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn action_queue_backpressure_no_warning_without_receiver() {
        // Backpressure sender is optional — warnings can be silently dropped
        let queue = BoundedActionCompletionQueue::new(10).unwrap();

        // Enqueue to 80% — should NOT panic even without a receiver
        for i in 0..8 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        // If we get here without panic, the test passes
        assert_eq!(queue.len(), 8);
    }
    #[test]
    fn backpressure_threshold_meets_documented_80_percent_vb_lxkqh() {
        // Regression test for RP-019 (bead vb-lxkqh).
        // The queue documents warnings at 80% capacity, so the threshold must be
        // the *smallest* integer >= capacity * 8 / 10 (ceiling division), not the
        // largest integer <= capacity * 8 / 10 (floor division). Floor division
        // causes noisy and misleading backpressure signals for capacities where
        // capacity * 8 is not a multiple of 10 (e.g. capacity=7 warns at 5, ~71%).
        //
        // For every c in `capacities`, the threshold must satisfy:
        //   threshold(c) >= c * 4 / 5            (>= 80% in integer arithmetic)
        //   threshold(c) >= ceil(c * 8 / 10)     (ceiling of 80%)
        // and the minimum-1 clamp must hold for c = 1.
        let capacities: &[usize] = &[1, 2, 3, 5, 7, 9, 10, 20, 100, 1_000];
        for &c in capacities {
            let threshold = backpressure_threshold(ActionQueueCapacity(c));
            let floor_80pct = (c * 4) / 5;
            let ceiling_80pct = (c * 8 + 9) / 10;
            assert!(
                threshold >= floor_80pct,
                "threshold for capacity {c} must be >= 80% ({floor_80pct}); got {threshold}"
            );
            assert!(
                threshold >= ceiling_80pct,
                "threshold for capacity {c} must be >= ceil(80%) = {ceiling_80pct}; got {threshold}"
            );
        }

        // Minimum-1 clamp must still apply: with capacity = 1, floor(1 * 8 / 10) = 0
        // but the queue must still warn at depth 1 (the queue itself).
        assert_eq!(backpressure_threshold(ActionQueueCapacity(1)), 1);

        // Documented 80% on a capacity-100 queue must yield threshold == 80 exactly.
        assert_eq!(backpressure_threshold(ActionQueueCapacity(100)), 80);

        // The original floor-division bug returned 5 for capacity = 7 (7 * 8 / 10 = 5).
        // Ceiling division must return 6 (7 * 8 / 10 = 5.6 -> 6).
        let threshold_7 = backpressure_threshold(ActionQueueCapacity(7));
        assert!(
            threshold_7 >= 6,
            "capacity=7 must produce threshold >= 6 (ceiling of 80%); got {threshold_7}"
        );
    }

    #[test]
    fn backpressure_outcome_delivered_when_warning_sent() {
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let (result, outcome) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(7));
        assert!(result.is_ok());
        assert_eq!(outcome, BackpressureWarningOutcome::Delivered);
        let warning = rx.try_recv().unwrap();
        assert_eq!(warning.depth, 8);
        assert_eq!(warning.capacity, 10);
    }

    #[test]
    fn backpressure_outcome_not_sent_below_threshold() {
        let (queue, _rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
        let (result, outcome) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(0));
        assert!(result.is_ok());
        assert_eq!(outcome, BackpressureWarningOutcome::NotSent);
    }

    #[test]
    fn backpressure_outcome_not_sent_without_channel() {
        let queue = BoundedActionCompletionQueue::new(5).unwrap();
        for i in 0..3 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let (result, outcome) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(3));
        assert!(result.is_ok());
        assert_eq!(outcome, BackpressureWarningOutcome::NotSent);
    }

    #[test]
    fn backpressure_outcome_channel_disconnected_when_receiver_dropped() {
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
        drop(rx);
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let (result, outcome) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(7));
        assert!(result.is_ok());
        assert_eq!(outcome, BackpressureWarningOutcome::ChannelDisconnected);
    }

    #[test]
    fn backpressure_outcome_channel_full_when_channel_bound_is_smaller() {
        // The public constructor binds the channel to queue capacity, which
        // warnings alone cannot fill; construct with a bound of 1 to prove
        // the Full branch surfaces instead of being silently dropped.
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let queue = BoundedActionCompletionQueue {
            inner: std::sync::Mutex::new(Inner {
                items: VecDeque::with_capacity(10),
            }),
            capacity: ActionQueueCapacity(10),
            backpressure_tx: Some(tx),
        };
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let (first, first_outcome) =
            queue.enqueue_with_backpressure_warning_outcome(make_ticket(7));
        assert!(first.is_ok());
        assert_eq!(first_outcome, BackpressureWarningOutcome::Delivered);
        let (second, second_outcome) =
            queue.enqueue_with_backpressure_warning_outcome(make_ticket(8));
        assert!(second.is_ok());
        assert_eq!(second_outcome, BackpressureWarningOutcome::ChannelFull);
    }

    #[test]
    fn backpressure_outcome_channel_full_via_public_api_cycling() {
        // The warning channel is bound to queue capacity (10), so filling it
        // through the public API requires cycling at warning depth while the
        // receiver lags: each dequeue/enqueue pair at depth 8 emits one more
        // warning until the channel holds 10 undelivered warnings.
        let (queue, _rx) = BoundedActionCompletionQueue::with_backpressure(10).unwrap();
        let mut seq = 0;
        for _ in 0..8 {
            queue.enqueue(make_ticket(seq)).unwrap();
            seq += 1;
        }
        // One warning already sent at depth 8; nine more fill the channel.
        let mut outcome = BackpressureWarningOutcome::Delivered;
        for _ in 0..10 {
            assert!(queue.dequeue().is_some());
            let (result, next) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(seq));
            assert!(result.is_ok());
            outcome = next;
            seq += 1;
        }
        assert_eq!(outcome, BackpressureWarningOutcome::ChannelFull);
    }

    #[test]
    fn backpressure_outcome_not_sent_on_queue_full() {
        let (queue, _rx) = BoundedActionCompletionQueue::with_backpressure(2).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        let (result, outcome) = queue.enqueue_with_backpressure_warning_outcome(make_ticket(2));
        assert!(result.is_err());
        assert_eq!(outcome, BackpressureWarningOutcome::NotSent);
    }
}

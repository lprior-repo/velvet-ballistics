// vb-y9d3v: ActionTicket fence proptest harnesses
#[cfg(test)]
mod proptest_attempt_fence;

// HVR-PO-RUNTIME-003/HVR-PO-RUNTIME-004/HVR-PO-RUNTIME-005: vb-god2f runtime generated properties.
#[cfg(test)]
mod proptest_vb_god2f_action_completion;

#[cfg(test)]
mod proptest_idempotency {
    use proptest::prelude::*;
    use proptest::strategy::Strategy;

    use vb_core::action::{ActionError, ActionTicket, Idempotency};
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    use crate::idempotency::IdempotencyTracker;

    fn arb_ticket() -> impl Strategy<Value = ActionTicket> {
        (
            1u64..100,
            0u16..32,
            0u64..1000,
            0u16..64,
            1u16..10,
            any::<u128>(),
            1u16..10,
        )
            .prop_map(
                |(run, step, seq, action, attempt, key, capacity)| ActionTicket {
                    run: RunId::new(run),
                    step: StepIdx::new(step),
                    seq: SeqNo::new(seq),
                    action: ActionId::new(action),
                    attempt,
                    idempotency_key: key,
                    capacity,
                    ..ActionTicket::default()
                },
            )
    }

    fn arb_capacity() -> impl Strategy<Value = usize> {
        1usize..16
    }

    proptest! {
        #[test]
        fn prop_completion_idempotent(ticket in arb_ticket()) {
            let mut tracker = IdempotencyTracker::with_default_capacity();

            let first = tracker.mark_completed(&ticket);
            assert_eq!(first, Ok(()), "first completion must succeed with Ok(()), got {:?}", first);

            let second = tracker.mark_completed(&ticket);
            assert_eq!(
                second,
                Err(ActionError::CompletionAlreadyRecorded),
                "duplicate completion must fail"
            );

            let len_after_first = 1;
            assert_eq!(
                tracker.len(),
                len_after_first,
                "tracker length must not change on duplicate"
            );
        }

        #[test]
        fn prop_length_never_exceeds_capacity(
            capacity in arb_capacity(),
            tickets in prop::collection::vec(arb_ticket(), 1..32)
        ) {
            let mut tracker = IdempotencyTracker::new(capacity);

            for ticket in &tickets {
                let _ = tracker.mark_completed(ticket);
                assert!(
                    tracker.len() <= capacity,
                    "tracker length {} must not exceed capacity {}",
                    tracker.len(),
                    capacity
                );
            }
        }

        #[test]
        fn prop_monotonic_until_eviction(
            tickets in prop::collection::vec(arb_ticket(), 3..16)
        ) {
            let capacity = 3;
            let mut tracker = IdempotencyTracker::new(capacity);

            let mut completed_keys: Vec<(usize, u128)> = Vec::new();
            let mut insert_idx: usize = 0;

            for ticket in &tickets {
                let result = tracker.mark_completed(ticket);
                if result.is_ok() {
                    completed_keys.push((insert_idx, ticket.idempotency_key));
                    insert_idx += 1;

                    let evicted_count = insert_idx.saturating_sub(capacity);
                    for &(idx, key) in &completed_keys {
                        let temp_ticket = ActionTicket {
                            run: ticket.run,
                            step: ticket.step,
                            seq: ticket.seq,
                            action: ticket.action,
                            attempt: ticket.attempt,
                            idempotency_key: key,
                            capacity: ticket.capacity,
                            ..ActionTicket::default()
                        };
                        let should_exist = idx >= evicted_count;
                        if should_exist {
                            assert!(
                                tracker.is_completed(&temp_ticket),
                                "key {} (inserted at idx {}) should still be tracked",
                                key, idx
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn prop_eviction_is_fifo(
            tickets in prop::collection::vec(arb_ticket(), 5..16)
        ) {
            let capacity = 3;
            let mut tracker = IdempotencyTracker::new(capacity);

            let mut unique_tickets: Vec<ActionTicket> = Vec::new();
            let mut key_counter: u128 = 1;
            for base in &tickets {
                let ticket = ActionTicket {
                    run: base.run,
                    step: base.step,
                    seq: base.seq,
                    action: base.action,
                    attempt: base.attempt,
                    idempotency_key: key_counter,
                    capacity: base.capacity,
                    ..ActionTicket::default()
                };
                key_counter += 1;
                unique_tickets.push(ticket);
            }

            for ticket in &unique_tickets {
                let _ = tracker.mark_completed(ticket);
            }

            if unique_tickets.len() > capacity {
                let oldest = &unique_tickets[0];
                assert!(
                    !tracker.is_completed(oldest),
                    "oldest ticket should be evicted"
                );

                let recent_start = unique_tickets.len() - capacity;
                for i in recent_start..unique_tickets.len() {
                    assert!(
                        tracker.is_completed(&unique_tickets[i]),
                        "recent ticket {} should be present",
                        i
                    );
                }
            }
        }

        #[test]
        fn prop_evicted_key_reinsertion(
            tickets in prop::collection::vec(arb_ticket(), 5..16)
        ) {
            let capacity = 2;
            let mut tracker = IdempotencyTracker::new(capacity);

            let mut unique_tickets: Vec<ActionTicket> = Vec::new();
            let mut key_counter: u128 = 1;
            for base in &tickets {
                let ticket = ActionTicket {
                    run: base.run,
                    step: base.step,
                    seq: base.seq,
                    action: base.action,
                    attempt: base.attempt,
                    idempotency_key: key_counter,
                    capacity: base.capacity,
                    ..ActionTicket::default()
                };
                key_counter += 1;
                unique_tickets.push(ticket);
            }

            for ticket in unique_tickets.iter().take(capacity + 1) {
                let _ = tracker.mark_completed(ticket);
            }

            let evicted = &unique_tickets[0];
            assert!(!tracker.is_completed(evicted), "first ticket should be evicted");

            let reinsert = tracker.mark_completed(evicted);
            assert_eq!(reinsert, Ok(()), "re-insertion of evicted key must succeed with Ok(()), got {:?}", reinsert);
            assert!(
                tracker.is_completed(evicted),
                "re-inserted key must be queryable"
            );
        }

        #[test]
        fn prop_policy_independence(
            key in any::<u128>(),
            seq in 0usize..20
        ) {
            let mut tracker = IdempotencyTracker::with_default_capacity();

            for _ in 0..seq {
                assert!(tracker.track_for_policy(Idempotency::DeterministicPure, key));
                assert!(!tracker.is_completed_for_policy(Idempotency::DeterministicPure, key));
            }

            let first = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key);
            assert!(first, "first track must return true");

            for _ in 1..seq {
                let dup = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key);
                assert!(!dup, "subsequent tracks must return false");
            }

            assert!(
                tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key),
                "AtLeastOnceExternal must be tracked"
            );
        }

        #[test]
        fn prop_mark_completed_for_policy_monotonic(
            key in any::<u128>()
        ) {
            let mut tracker = IdempotencyTracker::with_default_capacity();

            let first = tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key);
            assert_eq!(first, Ok(()), "first mark must succeed with Ok(()), got {:?}", first);

            let second = tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key);
            assert_eq!(
                second,
                Err(ActionError::CompletionAlreadyRecorded),
                "second mark must fail as duplicate"
            );

            assert!(
                tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key),
                "key must still be tracked"
            );
        }

        #[test]
        fn prop_different_keys_independent(
            key_a in any::<u128>(),
            key_b in any::<u128>()
        ) {
            prop_assume!(key_a != key_b);

            let mut tracker = IdempotencyTracker::with_default_capacity();

            let ticket_a = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(1),
                action: ActionId::new(1),
                attempt: 1,
                idempotency_key: key_a,
                capacity: 1,
                ..ActionTicket::default()
            };
            let ticket_b = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::new(0),
                seq: SeqNo::new(2),
                action: ActionId::new(2),
                attempt: 1,
                idempotency_key: key_b,
                capacity: 1,
                ..ActionTicket::default()
            };

            assert_eq!(tracker.mark_completed(&ticket_a), Ok(()), "ticket_a completion must succeed");
            assert_eq!(tracker.mark_completed(&ticket_b), Ok(()), "ticket_b completion must succeed");

            assert!(tracker.is_completed(&ticket_a));
            assert!(tracker.is_completed(&ticket_b));

            let dup_a = tracker.mark_completed(&ticket_a);
            assert_eq!(dup_a, Err(ActionError::CompletionAlreadyRecorded));
            assert!(tracker.is_completed(&ticket_b));
        }
    }
}

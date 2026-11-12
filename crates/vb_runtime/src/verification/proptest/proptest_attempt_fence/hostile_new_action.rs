use super::*;

// =========================================================================
// PO-0040: Proptest attempt fence — hostile inputs
// =========================================================================

proptest! {
    /// Property: Hostile inputs (zero attempt, zero capacity, extreme values)
    /// never cause panics in production functions.
    #[test]
    fn prop_hostile_inputs_no_panic(
        ticket in arb_hostile_ticket(),
        current in proptest::prelude::any::<u16>(),
    ) {
        let mut state = make_do_run_state(1);
        if let Some(slot) = state.action_attempts.get_mut(0) {
            *slot = current;
        }

        // Test normalize_scheduled_ticket with hostile inputs
        let _result1 = normalize_scheduled_ticket(&state, ticket);

        // Test record_scheduled_attempt with hostile inputs
        let mut state2 = make_do_run_state(1);
        if let Some(slot) = state2.action_attempts.get_mut(0) {
            *slot = current;
        }
        record_scheduled_attempt(&mut state2, ticket);

        // Test validate_action_completion with hostile inputs
        let mut state3 = make_do_run_state(1);
        if let Some(slot) = state3.action_attempts.get_mut(0) {
            *slot = current;
        }
        let _result3 = validate_action_completion(&state3, ticket);

        // Must not have panicked
    }
}

// =========================================================================
// Additional comprehensive fence properties
// =========================================================================

proptest! {
    /// Property: new_action_attempts creates correctly-sized trackers.
    #[test]
    fn prop_new_action_attempts_correct_size(
        step_count in 0u16..256,
    ) {
        let tracker = new_action_attempts(step_count);
        prop_assert_eq!(tracker.len(), step_count as usize,
            "tracker must have exactly step_count entries");
        // All entries must be zero
        for entry in tracker.iter() {
            prop_assert_eq!(*entry, 0, "all tracker entries must be initialized to 0");
        }
    }

    /// Property: Attempt counter is never mutated by failed validations.
    #[test]
    fn prop_validation_failure_does_not_mutate(
        current in proptest::prelude::any::<u16>(),
        (ticket_attempt, capacity) in (proptest::prelude::any::<u16>(),
                                        proptest::prelude::any::<u16>()),
    ) {
        let mut state = make_do_run_state(1);
        if let Some(slot) = state.action_attempts.get_mut(0) {
            *slot = current;
        }

        let before = state.action_attempts.get(0).copied().unwrap_or(0);

        let ticket = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(0),
            action: ActionId::new(0),
            run_attempt: 1,
            attempt: ticket_attempt,
            idempotency_key: 0,
            capacity,
         fencing_token: 0, };

        let _ = validate_action_completion(&state, ticket);
        let after = state.action_attempts.get(0).copied().unwrap_or(0);

        prop_assert_eq!(before, after,
            "validation must not mutate state (it takes &self)");
    }
}

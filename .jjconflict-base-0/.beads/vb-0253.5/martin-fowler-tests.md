# Martin Fowler Test Plan - vb-0253.5

## Happy Path Tests
- test_valid_transitions_accepted
- test_terminal_states_returns_correct_set
- test_non_terminal_states_returns_correct_set

## Error Path Tests
- test_invalid_transitions_rejected
- test_validate_transition_returns_err_for_invalid

## Edge Case Tests
- test_running_state_can_transition_to_waiting
- test_running_state_can_transition_to_asking
- test_terminal_states_are_final (no outgoing transitions)

## Contract Verification Tests
- test_precondition_pending_valid_initial_state
- test_postcondition_valid_transition_returns_true
- test_invariant_terminal_states_have_no_outgoing_transitions

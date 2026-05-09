# Martin Fowler Test Plan: vb-jggy

## Happy Path Tests

- `test_admitting_a_run_persists_attempt_one_before_ack` — a newly submitted run has its per-step `action_attempts` counter zeroed at admission; the first action ticket dispatched carries `attempt = 1`.
- `test_retrying_a_run_increments_and_persists_the_latest_attempt_before_new_tickets_are_issued` — when a retry is granted, `record_scheduled_attempt` advances the counter to `max(current, ticket.attempt)` and the next ticket carries the incremented attempt.

## Error Path Tests

- `test_completion_for_attempt_one_is_rejected_after_attempt_two_is_admitted` — given a run where `action_attempts[step] = 2`, an `ActionCompleted` for `attempt = 1` is rejected with `RuntimeError::StaleAttempt { incoming: 1, current: 2 }` before any journal write.
- `test_stale_failure_event_cannot_overwrite_the_latest_attempt_terminal_state` — given a run where `action_attempts[step] = 2`, an `ActionFailed` for `attempt = 1` is rejected with `RuntimeError::StaleAttempt` before any state mutation or journal write.

## Edge Case Tests

- `test_zero_attempt_ticket_is_rejected` — `validate_ticket_attempt` rejects `attempt = 0` immediately.
- `test_attempt_beyond_capacity_is_rejected` — a ticket with `attempt > capacity` returns `Err(AttemptBeyondMax)` before journal.
- `test_action_attempts_array_length_matches_step_count` — `new_action_attempts(step_count)` produces exactly `step_count` entries, all initialized to `0`.
- `test_record_scheduled_attempt_ignores_lower_attempt` — if current counter is `3`, a ticket with `attempt = 2` does NOT update the counter.
- `test_record_scheduled_attempt_accepts_higher_attempt` — if current counter is `1`, a ticket with `attempt = 3` updates the counter to `3`.

## Contract Verification Tests

- `test_precondition_stale_attempt_error_variant_exists` — verifies `RuntimeError::StaleAttempt` has `incoming` and `current` fields.
- `test_precondition_validate_ticket_attempt_exists_in_helpers` — verifies the function is accessible and has correct signature.
- `test_postcondition_first_ticket_attempt_is_one` — issues a ticket on a fresh run and asserts `ticket.attempt == 1`.
- `test_postcondition_journal_step_succeeded_carries_attempt` — after a successful completion, the journal event recorded carries the correct attempt number.
- `test_postcondition_journal_step_failed_carries_attempt` — after a failure, the journal event recorded carries the correct attempt number.
- `test_invariant_one_latest_attempt` — after multiple retries, `action_attempts[step]` holds exactly the maximum observed attempt.
- `test_invariant_monotonic_counter` — repeated calls to `record_scheduled_attempt` never produce a decrease.

## Given-When-Then Scenarios

### Scenario 1: First run admission initializes attempt counter to zero
**Given**: A `CompiledWorkflow` with 3 steps and a `RunId`
**When**: `handle_submit_with_inputs` is called
**Then**:
- `RunState::action_attempts` is a `Box<[u16]>` of length 3 with all values `0`
- No `RuntimeJournalEvent::StepSucceeded` or `StepFailed` has been written yet
- The run is inserted into `self.runs` map

### Scenario 2: First action ticket carries attempt = 1
**Given**: A submitted run with `action_attempts[step] = 0`
**When**: The engine issues the first `ActionTicket` for step 0
**Then**:
- `ticket.attempt == 1`
- `ticket.capacity == retry_policy.max_attempts`
- `record_scheduled_attempt(state, ticket)` updates `action_attempts[0]` to `1`

### Scenario 3: Retry increments the attempt counter before new ticket
**Given**: A run where step 0 has `action_attempts[0] = 1` and the previous ticket was `attempt = 1`
**When**: A retry is granted and `record_scheduled_attempt` is called with `ticket.attempt = 2`
**Then**:
- `action_attempts[0]` is updated to `2`
- The new ticket carries `attempt = 2`

### Scenario 4: Stale completion is rejected before journal mutation
**Given**: A run where `action_attempts[step] = 2` (attempt 2 already admitted)
**When**: `handle_action_completion` receives a ticket with `ticket.attempt = 1`
**Then**:
- `validate_ticket_attempt` returns `Err(RuntimeError::StaleAttempt { incoming: 1, current: 2 })`
- No `journal.append` call is made
- No frame state is mutated
- The run state is unchanged

### Scenario 5: Stale failure is rejected before journal mutation
**Given**: A run where `action_attempts[step] = 3`
**When**: `handle_action_failure` receives a ticket with `ticket.attempt = 2`
**Then**:
- Returns `Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })`
- No journal event is written
- No frame state is mutated

### Scenario 6: Valid completion proceeds to journal
**Given**: A run where `action_attempts[step] = 2` and the incoming ticket has `ticket.attempt = 2`
**When**: `handle_action_completion` is called with a valid ticket
**Then**:
- `validate_ticket_attempt` returns `Ok(())`
- The output slot is written
- The step is marked succeeded
- `RuntimeJournalEvent::StepSucceeded { run, step, attempt: 2 }` is appended to journal
- PC is advanced

### Scenario 7: Attempt counter never decreases
**Given**: A run where `action_attempts[0] = 5`
**When**: `record_scheduled_attempt` is called with a ticket for step 0 with `attempt = 3`
**Then**:
- `action_attempts[0]` remains `5` (not updated)
- No error is returned (noop for stale-wins-current)

### Scenario 8: Attempt counter overflow is rejected
**Given**: A retry policy with `max_attempts = 65535` and a run where `action_attempts[step] = 65535`
**When**: `record_scheduled_attempt` is called with `ticket.attempt = 65536`
**Then**:
- Returns `Err(RuntimeError::AttemptBeyondMax { attempt: 65536, max: 65535 })`

### Scenario 9: Zero attempt ticket is always rejected
**Given**: Any run state and a ticket with `ticket.attempt = 0`
**When**: `validate_ticket_attempt` is called
**Then**:
- Returns `Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: ticket.capacity })`

### Scenario 10: Full pipeline — admit, dispatch, retry, stale race
**Given**: A complete run from submit through action and retry
**When**: The following sequence occurs:
1. Submit run → `action_attempts[*] = 0`
2. First dispatch → ticket.attempt = 1, `action_attempts[step] = 1`
3. Action fails → retry granted
4. Retry dispatch → ticket.attempt = 2, `action_attempts[step] = 2`
5. Stale completion for attempt = 1 arrives late
**Then**:
- Steps 1–4 succeed normally
- Step 5 is rejected with `RuntimeError::StaleAttempt { incoming: 1, current: 2 }`
- Journal contains events only for attempts 1 and 2 in order; no stale event written

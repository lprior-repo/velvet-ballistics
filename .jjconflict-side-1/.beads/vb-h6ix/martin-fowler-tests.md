# Martin Fowler Test Plan: vb-h6ix — Replay Latest Execution Attempt Only

## Happy Path Tests

### test_replay_of_attempt_one_only_hydrates_attempt_one_state

**Given**: A journal with events from a single attempt (attempt 1) for a run.

**When**: Replay is executed on the journal events.

**Then**:
- The recovered `RecoveryFrameSeed` reflects only attempt 1 state.
- The `ActionReplayTracker` records only attempt 1 action completions.
- All input events are returned in the replay output list.

---

### test_replay_of_attempt_one_then_attempt_two_hydrates_attempt_two_state

**Given**: A journal with interleaved events from attempt 1 and attempt 2 for the same run (e.g., attempt 1 starts, fails, attempt 2 starts).

**When**: Replay is executed on the mixed-attempt journal.

**Then**:
- The recovered `RecoveryFrameSeed` reflects only attempt 2 state.
- Events from attempt 1 are NOT used to populate live slot values or pending actions.
- The `ActionReplayTracker` records only attempt 2 action completions.
- All events (both attempt 1 and attempt 2) are returned in the replay output list for diagnostics.

---

## Error Path Tests

### test_stale_success_from_attempt_one_after_attempt_two_does_not_complete_the_run

**Given**: A journal where attempt 1 has a `RunFinished` event, but attempt 2 has `StepStarted` and later events indicating the run is still in progress.

**When**: Replay is executed.

**Then**:
- The recovered terminal state is NOT `RunFinished` from attempt 1.
- If attempt 2 ends with `RunFailedEvent`, the recovered terminal state is `Failed`.
- If attempt 2 has no terminal event yet, the recovered state shows the run as in-progress.

---

### test_stale_timer_from_attempt_one_is_not_rearmed_after_attempt_two_starts

**Given**: A journal where attempt 1 schedules a `WaitScheduledEvent`, but attempt 2 starts before any wait completion.

**When**: Replay is executed.

**Then**:
- The `WaitScheduledEvent` from attempt 1 does NOT appear in the recovered pending actions.
- No timer/ticket is allocated for the stale wait from attempt 1.
- The recovered pending actions list is empty or reflects only attempt 2's scheduling.

---

## Edge Case Tests

### test_all_events_returned_including_stale

**Given**: A journal with events from multiple attempts.

**When**: Replay is executed.

**Then**:
- The returned replay event list has the same length as the input event list.
- All events from all attempts are preserved in the output (for diagnostics).

---

### test_replay_divergence_on_out_of_order_steps

**Given**: A journal where a step event from an older attempt has a higher step index than a step from a newer attempt (step ordering violation).

**When**: Replay is executed.

**Then**:
- `RecoveryError::ReplayDivergence` is returned with diagnostic details.

---

### test_stale_action_duplicate_is_blocked

**Given**: A journal where the same action is scheduled and completed in both attempt 1 and attempt 2 at the same step.

**When**: Replay is executed.

**Then**:
- The second occurrence of the action (from the stale attempt) returns `RecoveryError::NonIdempotentActionBlocked`.

---

### test_max_attempt_number_wins

**Given**: A journal where events have attempt numbers 1, 2, and 3 interleaved.

**When**: Replay determines the latest attempt.

**Then**:
- Attempt 3 is selected as the latest.
- Only attempt 3 events influence the recovered live state.

---

## Contract Verification Tests

### test_precondition_attempt_numbers_present

**Given**: Events with attempt numbers on action scheduling and completion events.

**When**: Attempt number is extracted.

**Then**:
- The maximum attempt number is correctly identified across all events.

---

### test_postcondition_latest_attempt_state_only

**Given**: A mixed-attempt journal.

**When**: Replay completes.

**Then**:
- `RecoveryFrameSeed` contains only state derived from the latest attempt.

---

### test_invariant_no_live_allocation_from_stale

**Given**: A mixed-attempt journal.

**When**: Replay completes.

**Then**:
- No slot values, pending action tickets, or timers from stale attempts appear in live state.

---

### test_invariant_tracker_latest_only

**Given**: A mixed-attempt journal.

**When**: Replay completes.

**Then**:
- `ActionReplayTracker` contains only completed/failed actions from the latest attempt.

---

### test_invariant_stale_terminal_blocked

**Given**: A journal with a stale `RunFinished` event from an older attempt followed by `RunFailedEvent` from a newer attempt.

**When**: `extract_terminal` is called on the replay output.

**Then**:
- The returned terminal event is from the newer attempt (failed), not the stale one (finished).

---

## Given-When-Then Scenarios

### Scenario 1: Single attempt replay

**Given**: A run with one attempt and no retries.

**When**: The system recovers the run from journal.

**Then**:
- All events are replayed.
- The recovered state reflects the single attempt.
- No stale events exist.

---

### Scenario 2: Retry produces latest attempt

**Given**: A run where attempt 1 failed and attempt 2 succeeded.

**When**: The system replays the journal.

**Then**:
- Only attempt 2 events are used for live hydration.
- Attempt 1 events are preserved in the output for diagnostics.
- The recovered terminal state is `Finished` from attempt 2.

---

### Scenario 3: Stale success does not resurrect run

**Given**: A run where attempt 1 succeeded, then attempt 2 started but did not complete.

**When**: The system replays the journal.

**Then**:
- The recovered terminal state is NOT `Finished` from attempt 1.
- The recovered state reflects attempt 2 as in-progress or failed.
- Attempt 1's `RunFinished` is preserved as a stale diagnostic event.

---

### Scenario 4: Multiple interleaved attempts

**Given**: A run with 3 attempts, each with interleaved step and action events.

**When**: The system replays the journal.

**Then**:
- Attempt 3 is selected as the latest.
- Only attempt 3 step and action events affect live state.
- The `ActionReplayTracker` reflects only attempt 3 completions.
- All 3 attempts' events are returned in the replay output.

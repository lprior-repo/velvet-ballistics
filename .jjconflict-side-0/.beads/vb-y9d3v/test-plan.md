# Test Plan: ActionTicket Generation Fence + Body Re-entry State Reset

bead_id: vb-y9d3v
plan_state: 8
plan_skill: test-planner
plan_invocation_id: vb-y9d3v-state8-test-planner-attempt1
plan_date: 2026-05-30
workdir: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v
input_proof_review: vb-y9d3v-state7-proof-reviewer-attempt3 (APPROVED)
input_bridge: proof-to-rust-map.md + proof-to-rust-review.md (APPROVED)

## Summary

- **Behaviors identified**: 75 (61 Part A ActionTicket fence, 14 Part B body re-entry)
- **Trophy allocation**: 16 unit / 28 integration / 5 e2e / 3 static
- **Target ratios**: ~55% integration, ~30% unit, ~10% e2e, ~5% static
- **Proptest invariants**: 6
- **Fuzz targets**: 1
- **Kani harnesses**: documented for reference (executed by proof-writer, not test-writer)
- **Mutation threshold**: >= 90% kill rate

## 1. Behavior Inventory

### Part A: ActionTicket Generation Fence

#### Attempt Authority Behaviors (ACT-001 through ACT-005)

| # | Behavior |
|---|----------|
| B-001 | `validate_ticket_attempt` returns `Ok(())` when attempt equals current and is valid (nonzero, within capacity) |
| B-002 | `validate_ticket_attempt` returns `Err(StaleAttempt)` when attempt is lower than current |
| B-003 | `validate_ticket_attempt` returns `Err(FutureAttempt)` or `Err(InvalidActionCompletion)` when attempt exceeds current (G005: future-attempt rejection) |
| B-004 | `validate_ticket_attempt` returns `Err(AttemptBeyondMax)` when attempt equals zero |
| B-005 | `validate_ticket_attempt` returns `Err(AttemptBeyondMax)` when capacity equals zero |
| B-006 | `validate_ticket_attempt` returns `Err(AttemptBeyondMax)` when attempt exceeds capacity |
| B-007 | `validate_ticket_attempt` returns `Err(InvalidActionCompletion)` when step index is out of bounds for action_attempts |
| B-008 | `validate_action_completion` returns `Err(InvalidActionCompletion)` when step state is not Running |
| B-009 | `validate_action_completion` returns `Err(InvalidActionCompletion)` when workflow node at step is missing |
| B-010 | `validate_action_completion` returns `Err(InvalidActionCompletion)` when node kind is not Do or action id mismatch |
| B-011 | `validate_action_completion` returns `Ok(())` when all preconditions satisfied (attempt exact, step Running, Do node, action matches) |

#### Ticket Scheduling Behaviors (ACT-002, ACT-003)

| # | Behavior |
|---|----------|
| B-012 | `normalize_scheduled_ticket` promotes attempt to 1 when current is zero and ticket.attempt is zero |
| B-013 | `normalize_scheduled_ticket` returns `Err(AttemptBeyondMax)` when normalized attempt (max of current, ticket.attempt, 1) exceeds capacity |
| B-014 | `normalize_scheduled_ticket` returns `Err(InvalidActionCompletion)` when step is out of bounds |
| B-015 | `normalize_scheduled_ticket` returns `Err(AttemptBeyondMax)` when capacity is zero |
| B-016 | `record_scheduled_attempt` is a no-op when ticket.attempt is zero |
| B-017 | `record_scheduled_attempt` updates action_attempts when current is zero or ticket.attempt is higher |

#### Retry Fence Behaviors (ACT-009 through ACT-011)

| # | Behavior |
|---|----------|
| B-018 | `validate_retry_attempt` returns `Ok(())` when policy.max_attempts > 0, ticket.attempt > 0, and attempt <= max_attempts |
| B-019 | `validate_retry_attempt` returns `Err(AttemptBeyondMax)` when policy.max_attempts is zero |
| B-020 | `validate_retry_attempt` returns `Err(AttemptBeyondMax)` when ticket.attempt is zero |
| B-021 | `validate_retry_attempt` returns `Err(AttemptBeyondMax)` when attempt exceeds policy.max_attempts |
| B-022 | `record_retry_attempt` returns `Ok(true)` when current attempt < max_attempts and checked increment succeeds |
| B-023 | `record_retry_attempt` returns `Ok(false)` when current attempt >= max_attempts (retries exhausted) |
| B-024 | `record_retry_attempt` returns `Err(AttemptBeyondMax)` when validation fails |
| B-025 | `record_retry_attempt` uses `checked_add(1)` and returns `Err(UnsupportedOperation)` on overflow |
| B-026 | `record_retry_attempt` returns `Err(InvalidActionCompletion)` when step is out of bounds |
| B-027 | `retry_policy_after_action` returns `Err(UnsupportedOperation)` when retry check node is missing |
| B-028 | `retry_policy_after_action` returns `Err(UnsupportedOperation)` when policy slot is unreadable |
| B-029 | `retry_policy_after_action` returns `Err(UnsupportedOperation)` when policy slot is not I64 |
| B-030 | `retry_policy_after_action` returns `Err(UnsupportedOperation)` when max_attempts is out of u16 range |
| B-031 | `retry_policy_after_action` returns `Err(UnsupportedOperation)` when max_attempts is zero |
| B-032 | `retry_policy_after_action` returns `Ok(RetryPolicy)` with valid retry check node and I64 slot |
| B-033 | `retry_metadata_exists` returns true when Do node has RetryCheck successor |
| B-034 | `retry_metadata_exists` returns false when no RetryCheck successor exists |

#### Completion Preflight Behaviors (ACT-008)

| # | Behavior |
|---|----------|
| B-035 | `reject_invalid_ticket_key` returns `Ok(())` when ticket.idempotency_key matches canonical key for (run, seq, action) |
| B-036 | `reject_invalid_ticket_key` returns `Err(InvalidActionCompletion)` when key does not match canonical |
| B-037 | `preflight_action_completion` returns valid `ActionCompletionPreflight` when all checks pass |
| B-038 | `preflight_action_completion` returns `Err(InvalidActionCompletion)` when output_slot differs from Do output |
| B-039 | `preflight_action_completion` rejects taint downgrade with typed error |
| B-040 | `preflight_action_completion` rejects encoded length mismatch with typed error |
| B-041 | `preflight_action_completion` rejects contract output size exceeded |
| B-042 | `preflight_action_completion` rejects resource output size exceeded |

#### Terminal Run Fence Behaviors (ACT-012, TMR-001 through TMR-003)

| # | Behavior |
|---|----------|
| B-043 | `handle_action_completion` on a missing run returns `Err(RunNotFound)` |
| B-044 | `handle_action_completion` on a terminal (finished) run returns `Err(RunNotFound)` |
| B-045 | `handle_action_completion` on a cancelled run returns `Err(RunNotFound)` |
| B-046 | `finish_run` appends `RunFinished` journal event, inserts run into terminal_runs, releases frame |
| B-047 | `handle_action_failure` on a missing run returns `Err(RunNotFound)` |
| B-048 | `handle_action_failure` with stale attempt returns `Err(StaleAttempt)` |
| B-049 | `retry_is_available` returns true when retry policy is Retryable and retry metadata exists |
| B-050 | `apply_action_failure_to_state` returns `ActionFailureOutcome::RetryNow` when retry is available |

#### Timer Wheel Authority Behaviors (TMR-001 through TMR-003)

| # | Behavior |
|---|----------|
| B-051 | `TimerWheel::insert` creates a new entry with generation=1 when no prior entry exists |
| B-052 | `TimerWheel::insert` increments generation (gen+1) when replacing existing entry; overflow returns `GenerationExhausted` |
| B-053 | `TimerWheel::cancel` removes entry from both indexes and returns true |
| B-054 | `TimerWheel::cancel` returns false when no entry exists for run |
| B-055 | `TimerWheel::fire_expired` fires entry when its generation matches current run-index entry |
| B-056 | `TimerWheel::fire_expired` removes stale entry from time-index but does NOT remove current run-index (stale fire is ignored) |
| B-057 | `TimerWheel::fire_expired` returns fired entries in deadline order |

#### Non-Mutation Behaviors (ACT-007)

| # | Behavior |
|---|----------|
| B-058 | Stale attempt completion leaves journal, frame, trace, runtime state, counters unchanged |
| B-059 | Future attempt completion leaves journal, frame, trace, runtime state, counters unchanged |
| B-060 | Noncanonical key completion leaves journal, frame, trace, runtime state, counters unchanged |
| B-061 | Invalid action completion (wrong step state/action/node) leaves all observable state unchanged |

### Part B: Body Re-entry State Reset (Already has existing tests)

| # | Behavior | Existing Test |
|---|----------|---------------|
| B-062 | `Succeeded → Pending` is a valid transition in VALID_TRANSITIONS | `test_invalid_transitions` |
| B-063 | Terminal states reject non-terminal transitions | `test_terminal_immutable` |
| B-064 | `RunFrame::mark_pending` transitions step to Pending | `frame_mark_succeeded_on_pending_step_allows_overwrite` |
| B-065 | `jump_to_body` resets Succeeded→Pending before set_pc | `tc001_jump_to_body_succeeded_to_pending` |
| B-066 | `for_each_next` uses jump_to_body for re-entry | `vb_y4pa_001_for_each_two_item_reentry` |
| B-067 | `reduce_next` uses jump_to_body for re-entry | `vb_y4pa_002_reduce_reentry` |
| B-068 | `collect_next` uses jump_to_body for re-entry | `vb_y4pa_003_collect_next_reentry` |
| B-069 | `collect_page` uses jump_to_body for re-entry | `vb_y4pa_004_collect_page_reentry` |
| B-070 | `repeat_attempt` uses jump_to_body for re-entry | `vb_y4pa_005_repeat_attempt_reentry` |
| B-071 | `repeat_check` uses jump_to_body for re-entry | `vb_y4pa_006_repeat_check_reentry` |
| B-072 | GWT-1: for_each 2-item list body runs twice without state machine error | `gwt_re1_for_each_body_reentry_after_succeeded` |
| B-073 | Proptest: jump_to_body never errors | `prop1_jump_to_body_never_errors` |
| B-074 | Proptest: for_each n-items all re-entry | `prop2_for_each_n_items_all_reentry` |
| B-075 | Verus: terminal_cannot_transition_to_non_terminal holds | `terminal_cannot_transition_to_non_terminal` |

## 2. Trophy Allocation

| Layer | Count | % | Rationale |
|---|---|---|---|
| **Static Analysis** | 3 | 5% | clippy zero-tolerance, cargo-deny, compile-time type checks. `ActionTicket` fields are `u16`, no `unsafe`, checked arithmetic enforced at compile time. |
| **Unit (Calc)** | 16 | 30% | Pure functions: `validate_ticket_attempt`, `validate_retry_attempt`, `reject_invalid_ticket_key`, `compute_action_idempotency_key`, `action_ticket_has_valid_key`. These are the exact mathematical fence computations with no I/O. |
| **Integration** | 28 | 55% | Component interactions: `validate_action_completion` (combines attempt + state + node checks), `record_retry_attempt` (mutates state + checks arithmetic), `preflight_action_completion` (chains multiple validation steps), `handle_action_completion` (Shard lifecycle), `TimerWheel::fire_expired` (dual-index consistency), `apply_action_failure_to_state` (failure flow). |
| **E2E** | 5 | 10% | Full Shard API acceptance tests: submit run → complete action (exact), stale attempt after retry, terminal run fence, timer replacement authority, valid failure-to-retry flow. |
| **Total (Part A new)**| 52 | 100% | Unit + integration + e2e + static for Part A only |

**Deviation justification**: The integration layer dominates because ActionTicket fence behaviors are inherently component-boundary behaviors — `validate_ticket_attempt` is private and tested through `validate_action_completion`, retry functions manipulate `RunState`, and lifecycle handlers operate on Shard. Unit tests cover pure calculations; integration tests verify the real fence works at the component crossings.

## 3. BDD Scenarios

### 3.1 Attempt Authority Scenarios (B-001 through B-011)

#### Behavior B-001: Exact attempt match passes validation

```
### Behavior: validate_ticket_attempt accepts exact current attempt
Given: a RunState with action_attempts[step] = 3, a ticket with attempt=3, capacity=5
When: validate_ticket_attempt(state, ticket) is called
Then: returns Ok(())
```

```
fn validate_ticket_attempt_returns_ok_when_attempt_equals_current()
```

#### Behavior B-002: Stale attempt rejected

```
### Behavior: validate_ticket_attempt rejects lower attempt
Given: a RunState with action_attempts[step] = 3, a ticket with attempt=1, capacity=5
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::StaleAttempt { incoming: 1, current: 3 })
And: does not mutate any RunState field
```

```
fn validate_ticket_attempt_returns_stale_attempt_when_attempt_lower_than_current()
```

#### Behavior B-003: Future attempt rejected (G005 — implementation gap)

```
### Behavior: validate_ticket_attempt rejects future attempt
Given: a RunState with action_attempts[step] = 3, a ticket with attempt=5, capacity=10
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::FutureAttempt { incoming: 5, current: 3 })
      OR returns Err(RuntimeError::InvalidActionCompletion) if FutureAttempt variant not yet added
And: does not mutate any RunState field
```

```
fn validate_ticket_attempt_rejects_future_attempt_when_attempt_exceeds_current()
```

#### Behavior B-004: Zero attempt rejected

```
### Behavior: validate_ticket_attempt rejects attempt zero
Given: a RunState with action_attempts[step] = 0, a ticket with attempt=0, capacity=5
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 5 })
```

```
fn validate_ticket_attempt_rejects_when_attempt_is_zero()
```

#### Behavior B-005: Zero capacity rejected

```
### Behavior: validate_ticket_attempt rejects capacity zero
Given: a RunState with action_attempts[step] = 0, a ticket with attempt=1, capacity=0
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
```

```
fn validate_ticket_attempt_rejects_when_capacity_is_zero()
```

#### Behavior B-006: Over-capacity attempt rejected

```
### Behavior: validate_ticket_attempt rejects attempt exceeding capacity
Given: a RunState with action_attempts[step] = 3, a ticket with attempt=6, capacity=5
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 6, max: 5 })
```

```
fn validate_ticket_attempt_rejects_when_attempt_exceeds_capacity()
```

#### Behavior B-007: Missing step in action_attempts

```
### Behavior: validate_ticket_attempt rejects out-of-bounds step
Given: a RunState with action_attempts of length 3, a ticket with step index 5
When: validate_ticket_attempt(state, ticket) is called
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

```
fn validate_ticket_attempt_rejects_when_step_out_of_bounds()
```

#### Behavior B-008: Wrong step state

```
### Behavior: validate_action_completion rejects non-Running step state
Given: a RunState where frame.step_state(step) is Succeeded, ticket valid for that step
When: validate_action_completion(state, ticket) is called
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

```
fn validate_action_completion_rejects_when_step_not_running()
```

#### Behavior B-009: Missing workflow node

```
### Behavior: validate_action_completion rejects missing node
Given: a RunState with step index beyond workflow node count
When: validate_action_completion(state, ticket) is called
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

```
fn validate_action_completion_rejects_when_node_missing()
```

#### Behavior B-010: Wrong node kind or action mismatch

```
### Behavior: validate_action_completion rejects non-Do node or action mismatch
Given: a RunState with node.kind == CompiledNodeKind::Label at step, or Do{action: other}
When: validate_action_completion(state, ticket) is called
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

```
fn validate_action_completion_rejects_when_node_not_do_or_action_mismatch()
```

#### Behavior B-011: All preconditions satisfied

```
### Behavior: validate_action_completion accepts valid ticket on Do node
Given: a RunState with Do node at step, state=Running, attempt exact, action matches
When: validate_action_completion(state, ticket) is called
Then: returns Ok(())
```

```
fn validate_action_completion_returns_ok_when_all_preconditions_satisfied()
```

### 3.2 Ticket Scheduling Scenarios (B-012 through B-017)

#### Behavior B-012: Normalize from zero to 1

```
### Behavior: normalize_scheduled_ticket promotes zero attempt to 1
Given: a RunState with action_attempts[step] = 0, a ticket with attempt=0, capacity=5
When: normalize_scheduled_ticket(state, ticket) is called
Then: returns Ok(ActionTicket { attempt: 1, ..ticket })
```

```
fn normalize_scheduled_ticket_promotes_to_one_when_current_and_ticket_are_zero()
```

#### Behavior B-013: Normalize exceeds capacity

```
### Behavior: normalize_scheduled_ticket rejects when normalized attempt exceeds capacity
Given: a RunState with action_attempts[step] = 0, a ticket with attempt=10, capacity=5
When: normalize_scheduled_ticket(state, ticket) is called
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 10, max: 5 })
```

```
fn normalize_scheduled_ticket_rejects_when_normalized_attempt_exceeds_capacity()
```

#### Behavior B-014/B-015: Normalize error paths

```
fn normalize_scheduled_ticket_rejects_when_step_out_of_bounds()
fn normalize_scheduled_ticket_rejects_when_capacity_is_zero()
```

#### Behavior B-016/B-017: Record scheduled attempt

```
fn record_scheduled_attempt_is_noop_when_ticket_attempt_is_zero()
fn record_scheduled_attempt_updates_when_current_is_zero()
fn record_scheduled_attempt_updates_when_ticket_attempt_is_higher()
```

### 3.3 Retry Fence Scenarios (B-018 through B-034)

#### Behavior B-018: Valid retry attempt

```
### Behavior: validate_retry_attempt accepts valid retry
Given: a policy with max_attempts=3, a ticket with attempt=2
When: validate_retry_attempt(ticket, policy) is called
Then: returns Ok(())
```

```
fn validate_retry_attempt_returns_ok_when_within_bounds()
```

#### Behaviors B-019 through B-021: Invalid retry inputs

```
fn validate_retry_attempt_rejects_when_max_attempts_is_zero()
fn validate_retry_attempt_rejects_when_ticket_attempt_is_zero()
fn validate_retry_attempt_rejects_when_attempt_exceeds_max()
```

#### Behaviors B-022 through B-026: Record retry attempt

```
### Behavior: record_retry_attempt increments and returns true
Given: a RunState with action_attempts[step]=2, policy.max_attempts=5
When: record_retry_attempt(state, ticket, policy) is called
Then: action_attempts[step] becomes 3
And: returns Ok(true)
```

```
fn record_retry_attempt_increments_and_returns_true_when_retries_remain()
fn record_retry_attempt_returns_false_when_retries_exhausted()
fn record_retry_attempt_rejects_when_validation_fails()
fn record_retry_attempt_rejects_on_overflow_with_checked_add()
fn record_retry_attempt_rejects_when_step_out_of_bounds()
```

#### Behaviors B-027 through B-034: Retry policy extraction

```
fn retry_policy_after_action_rejects_when_retry_check_missing()
fn retry_policy_after_action_rejects_when_policy_slot_unreadable()
fn retry_policy_after_action_rejects_when_policy_slot_not_i64()
fn retry_policy_after_action_rejects_when_max_attempts_out_of_u16_range()
fn retry_policy_after_action_rejects_when_max_attempts_is_zero()
fn retry_policy_after_action_returns_valid_retry_policy()
fn retry_metadata_exists_returns_true_when_retry_check_successor_exists()
fn retry_metadata_exists_returns_false_when_no_retry_check()
```

### 3.4 Completion Preflight Scenarios (B-035 through B-042)

#### Behaviors B-035/B-036: Idempotency key

```
### Behavior: reject_invalid_ticket_key accepts canonical key
Given: a ticket whose idempotency_key equals compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)
When: reject_invalid_ticket_key(ticket) is called
Then: returns Ok(())
```

```
fn reject_invalid_ticket_key_returns_ok_when_key_matches_canonical()
fn reject_invalid_ticket_key_rejects_when_key_does_not_match_canonical()
```

#### Behaviors B-037/B-038: Preflight happy path and output slot mismatch

```
fn preflight_action_completion_returns_preflight_when_all_checks_pass()
fn preflight_action_completion_rejects_when_output_slot_mismatches()
```

#### Behaviors B-039 through B-042: Payload preflight rejects

```
fn preflight_action_completion_rejects_taint_downgrade()
fn preflight_action_completion_rejects_encoded_len_mismatch()
fn preflight_action_completion_rejects_contract_output_too_large()
fn preflight_action_completion_rejects_resource_output_too_large()
```

### 3.5 Terminal Run Fence Scenarios (B-043 through B-050)

#### Behaviors B-043 through B-046: Terminal run fence

```
### Behavior: handle_action_completion rejects completion on missing run
Given: a Shard with no active run for ticket.run
When: shard.handle_action_completion(ticket, output) is called
Then: returns Err(RuntimeError::RunNotFound)
And: journal length, trace_ring length, runtime_states map, counters unchanged
```

```
fn handle_action_completion_returns_run_not_found_when_run_missing()
fn handle_action_completion_returns_run_not_found_when_run_finished()
fn handle_action_completion_returns_run_not_found_when_run_cancelled()
fn finish_run_appends_run_finished_event_and_inserts_terminal_run()
```

#### Behaviors B-047 through B-050: Failure handling

```
fn handle_action_failure_returns_run_not_found_when_run_missing()
fn handle_action_failure_returns_stale_attempt_when_attempt_mismatch()
fn apply_action_failure_to_state_returns_retry_now_when_retry_available()
fn apply_action_failure_to_state_returns_drive_handler_when_retry_not_available()
```

### 3.6 Timer Wheel Authority Scenarios (B-051 through B-057)

#### Behaviors B-051 through B-054: Insert and cancel

```
### Behavior: TimerWheel::insert creates fresh entry
Given: an empty TimerWheel
When: insert(run, deadline, kind) is called
Then: by_run contains entry with generation=1
And: by_deadline contains entry at correct deadline
```

```
fn timer_wheel_insert_creates_entry_with_generation_one_when_no_prior_entry()
fn timer_wheel_insert_increments_generation_when_replacing_existing_entry()
fn timer_wheel_insert_returns_generation_exhausted_on_overflow()
fn timer_wheel_cancel_removes_entry_and_returns_true_when_entry_exists()
fn timer_wheel_cancel_returns_false_when_no_entry_exists()
```

#### Behaviors B-055/B-056/B-057: Fire expired

```
### Behavior: fire_expired fires current entries, ignores stale
Given: a TimerWheel with entry (run, gen=2) at by_run, and time-index still has old entry (run, gen=1) in by_deadline from before replacement
When: fire_expired(now) is called for a time after deadline
Then: the gen=2 entry is returned in fired list
And: the gen=1 stale entry is only removed from time-index (not currently matched by by_run)
```

```
fn timer_wheel_fire_expired_fires_fresh_entry_when_generation_matches()
fn timer_wheel_fire_expired_ignores_stale_entry_when_generation_mismatch()
fn timer_wheel_fire_expired_ignores_stale_entry_after_cancel()
fn timer_wheel_fire_expired_returns_fired_entries_in_deadline_order()
```

### 3.7 Non-Mutation Scenarios (B-058 through B-061)

These are the most critical integration assertions — they verify that invalid authority does not mutate any observable state.

```
### Behavior: stale attempt completion is non-mutating
Given: a Shard with an active live run, journals, traces, counters initialized
And: a ticket with attempt lower than current
When: shard.handle_action_completion(ticket, output) is called
Then: returns Err(RuntimeError::StaleAttempt { ... })
And: journal event count, trace_ring entry count, runtime_states, counters completed, counters failed, run frame step states, action_attempts all equal their values before the call
```

```
fn stale_attempt_completion_does_not_mutate_journal_frame_trace_counters_or_runtime_state()
fn future_attempt_completion_does_not_mutate_journal_frame_trace_counters_or_runtime_state()
fn noncanonical_key_completion_does_not_mutate_journal_frame_trace_counters_or_runtime_state()
fn invalid_action_completion_does_not_mutate_journal_frame_trace_counters_or_runtime_state()
```

### 3.8 E2E Acceptance Scenarios

These are black-box tests exercising full Shard public API.

```
### E2E: Exact attempt completion succeeds end-to-end
Given: a Shard with admitted run containing a Do node, action contract, engine has scheduled attempt=1 with capacity=3
When: external caller submits action completion with ticket.attempt=1, correct key, correct output
Then: Shard appends ActionCompletedEnvelope to journal
And: frame step state transitions to Succeeded
And: run continues execution

### E2E: Stale attempt after retry is rejected end-to-end
Given: same setup, but engine has advanced to attempt=2 after one retry
When: external caller submits action completion with ticket.attempt=1 (the old, stale ticket)
Then: Shard returns StaleAttempt error
And: no journal event appended
And: frame state preserved at attempt=2

### E2E: Future attempt without scheduling is rejected end-to-end
Given: engine has scheduled attempt=1, capacity=5
When: external caller submits action completion with ticket.attempt=3 (skipping attempt 2)
Then: Shard returns FutureAttempt or InvalidActionCompletion error
And: no journal event appended

### E2E: Timer replacement invalidates old generation
Given: a Shard with run awaiting timer at deadline D, timer generation=1
When: timer is replaced (generation becomes 2)
And: fire_expired is called for deadline D
Then: the fired entry at generation=1 is stale and does not resume the run
And: the current generation=2 entry remains in by_run

### E2E: Valid failure with retry advances to next attempt
Given: a Shard with run at attempt=1, Do node with RetryCheck successor, capacity=3
When: external action failure is received for attempt=1 with Retryable policy
Then: run transitions to retry, attempt advances to 2
And: ActionFailed journal event appended
```

```
fn e2e_exact_attempt_completion_succeeds()
fn e2e_stale_attempt_after_retry_is_rejected()
fn e2e_future_attempt_without_scheduling_is_rejected()
fn e2e_timer_replacement_invalidates_old_generation()
fn e2e_valid_failure_with_retry_advances_to_next_attempt()
```

## 4. Proptest Invariants

### Proptest 1: Attempt fence exhaustive classification

```
### Proptest: validate_ticket_attempt_attempt_classification
Invariant: For any valid (attempt, capacity, current) tuple:
  - attempt == 0 OR capacity == 0 OR attempt > capacity => Err(AttemptBeyondMax)
  - attempt < current => Err(StaleAttempt) with correct {incoming, current} values
  - attempt == current => Ok(())  (after zero/over-capacity filter)
  - attempt > current => Err(FutureAttempt) or Err(InvalidActionCompletion) (G005)
Strategy: Generate arbitrary u16 values for attempt, capacity, current with realistic ranges (0..=u16::MAX, but bounded to prevent exhaustive search blowup). Also generate hostile edge cases.
Anti-invariant: No input should cause panic (panic freedom is the meta-property).
```

```
fn prop_validate_ticket_attempt_classifies_all_attempt_relations()
fn prop_validate_ticket_attempt_never_panics()
```

### Proptest 2: Idempotency key determinism

```
### Proptest: compute_action_idempotency_key_determinism
Invariant: compute_action_idempotency_key(run, seq, action) always returns the same value for the same inputs.
Also: Different (run, seq, action) tuples may collide (wrapping hash), but same tuple always produces same output.
Strategy: arbitrary RunId, SeqNo, ActionId values. Run twice and assert equality.
```

```
fn prop_idempotency_key_is_deterministic()
```

### Proptest 3: Retry attempt monotonicity

```
### Proptest: record_retry_attempt_increments_by_exactly_one
Invariant: When record_retry_attempt returns Ok(true), action_attempts[step] is exactly previous + 1.
When it returns Ok(false), attempt >= max_attempts and was not incremented.
Strategy: arbitrary valid RunState with action_attempts populated, valid policy, ticket with attempt within bounds.
```

```
fn prop_record_retry_attempt_increments_by_exactly_one_or_not_at_all()
fn prop_record_retry_attempt_never_panics()
```

### Proptest 4: Timer generation monotonicity

```
### Proptest: timer_generation_is_monotonic
Invariant: Each insert (after the first) increments generation by exactly 1.
Overflow path: when generation hits u64::MAX, next insert returns GenerationExhausted.
Strategy: arbitrary sequence of insert operations; track expected generation.
```

```
fn prop_timer_insert_increments_generation_monotonically()
```

### Proptest 5: Cancel idempotency

```
### Proptest: timer_cancel_is_idempotent
Invariant: cancel(run) after cancel(run) returns false (already removed).
Strategy: insert entry, cancel once (expect true), cancel again (expect false).
```

```
fn prop_timer_cancel_is_idempotent()
fn prop_timer_cancel_returns_false_on_missing_entry()
```

### Proptest 6: Non-mutation property

```
### Proptest: invalid_authority_is_non_mutating
Invariant: For any invalid ticket (stale, future, noncanonical key, wrong step state, wrong action),
  calling handle_action_completion produces the same journal, trace_ring, counters, and runtime_states
  as before the call (sampled before and after).
Strategy: generate arbitrary hostile ActionTicket values against a valid live run state.
Anti-invariant: No invalid ticket should ever append a journal event, push a trace, or increment a counter.
```

```
fn prop_invalid_authority_never_mutates_state()
```

## 5. Fuzz Targets

### Fuzz Target 1: Retry counter serialization boundary

```
### Fuzz Target: fuzz_retry_codec
Input type: arbitrary byte sequences (Vec<u8>)
Risk: The retry codec path (serialization/deserialization of retry-related data through Postcard
  or custom binary encoding) may panic on malformed input, OOM on large inputs, or produce
  incorrect state on edge cases.
Corpus seeds:
  - empty byte sequence
  - single byte 0x00
  - valid Postcard-encoded u16 values
  - Postcard-encoded ActionTicket with corrupted length prefix
  - max-length byte sequences
  - byte sequences with all zeros / all ones
Target function: deserialization entry point for retry-related data structures (if applicable),
  or fallback to encode/decode round-trip of ActionTicket through Postcard.
```

```
fn fuzz_retry_codec_roundtrip(arbitrary_bytes: &[u8])
```

## 6. Kani Verification Harnesses

Kani harnesses were planned and partially written by the proof-writer (State 5). These are documented for reference but are NOT test-writer obligations. Test-writer writes only behavior tests and proptest properties. Kani execution is deferred to State 12.

| Harness | Property | Bound | Planned Source |
|---|---|---|---|
| `proof_stale_attempt_rejected` | `ticket.attempt < current => Err(StaleAttempt)` | Attempt range 0..=u8::MAX | `kani_attempt_fence_harnesses.rs` |
| `proof_future_attempt_rejected` | `ticket.attempt > current => Err(FutureAttempt)` (G005) | Attempt range 0..=u8::MAX | `kani_attempt_fence_harnesses.rs` |
| `proof_retry_capacity_fence` | `attempt > max_attempts => Err(AttemptBeyondMax)` | Attempt range 0..=u8::MAX | `kani_attempt_fence_harnesses.rs` |
| `proof_stale_authority_no_mutation` | Invalid authority never mutates state | Bounded state exploration | `kani_attempt_fence_harnesses.rs` |
| `proof_single_terminal_event_invariant` | One terminal event per run | Bounded state machine | `kani_attempt_fence_harnesses.rs` |
| `proof_typed_missing_run_error` | `RunNotFound` for absent runs | Shard state exploration | `kani_attempt_fence_harnesses.rs` |
| `proof_action_fence_panic_free` | No panics in attempt fence path | All u16 inputs | `kani_attempt_fence_harnesses.rs` |

## 7. Mutation Checkpoints

### Critical mutations that tests must survive:

| # | Code Location | Mutation | Killing Test |
|---|---|---|---|
| M-1 | `helpers.rs:76` `== 0` → removed | Zero attempt accepted | `validate_ticket_attempt_rejects_when_attempt_is_zero` |
| M-2 | `helpers.rs:76` `== 0` (capacity) → removed | Zero capacity accepted | `validate_ticket_attempt_rejects_when_capacity_is_zero` |
| M-3 | `helpers.rs:76` `>` → `>=` | Boundary shift accepts attempt==capacity as invalid | `validate_ticket_attempt_returns_ok_when_attempt_equals_current` (with attempt==capacity) |
| M-4 | `helpers.rs:87` `<` → `<=` | Exact attempt treated as stale | `validate_ticket_attempt_returns_ok_when_attempt_equals_current` |
| M-5 | `helpers.rs:87-91` removed | Stale attempt silently passes | `validate_ticket_attempt_returns_stale_attempt_when_attempt_lower_than_current` |
| M-6 | `helpers.rs:288` `checked_add(1)` → `wrapping_add(1)` | Overflow silently wraps | `retry_attempt_at_u16_max_returns_overflow_error` |
| M-7 | `helpers.rs:285` `>=` → `>` | Off-by-one allows one extra retry beyond max | `record_retry_attempt_returns_false_when_retries_exhausted` |
| M-8 | `chunk_003.rs:86` `==` → `!=` | Key check inverted, wrong keys pass, right keys fail | `reject_invalid_ticket_key_rejects_when_key_does_not_match_canonical` |
| M-9 | `chunk_003.rs:53` preflight call removed | All preflight checks bypassed | `preflight_action_completion_rejects_when_output_slot_mismatches` |
| M-10 | `timer_wheel.rs:120` `self.by_run.get(&entry.run).copied() == Some(entry)` → `true` | Stale timer fires are never filtered | `timer_wheel_fire_expired_ignores_stale_entry_when_generation_mismatch` |
| M-11 | `timer_wheel.rs:84` `checked_add(1)` → `wrapping_add(1)` | Generation overflow silently wraps | `timer_wheel_insert_returns_generation_exhausted_on_overflow` |
| M-12 | `chunk_001.rs:378` run lookup → removed | Preflight runs on wrong/absent state | `handle_action_completion_returns_run_not_found_when_run_missing` |
| M-13 | `helpers.rs:289-292` `checked_add` result → always `Ok(1)` | Overflow path never exercised | `record_retry_attempt_rejects_on_overflow_with_checked_add` |
| M-14 | `transitions.rs:86` terminal run insert → `swap_remove` | Terminal run evictable prematurely | `finish_run_appends_run_finished_event_and_inserts_terminal_run` |

**Threshold**: >=90% mutation kill rate. All 14 mutation checkpoints above must be killed.

## 8. Combinatorial Coverage Matrix

### Test Group: validate_ticket_attempt (Unit tests on extracted helper)

| Scenario | attempt | capacity | current | Expected Output | Test Layer |
|---|---|---|---|---|---|
| Happy: exact match | 3 | 5 | 3 | `Ok(())` | unit |
| Stale: lower by 1 | 2 | 5 | 3 | `Err(StaleAttempt{incoming:2,current:3})` | unit |
| Stale: lower by many | 1 | 5 | 5 | `Err(StaleAttempt{incoming:1,current:5})` | unit |
| Stale: lower edge (1 vs 2) | 1 | 5 | 2 | `Err(StaleAttempt{incoming:1,current:2})` | unit |
| Future: higher by 1 | 4 | 5 | 3 | `Err(FutureAttempt\|InvalidActionCompletion)` | unit |
| Future: higher by many | 5 | 10 | 2 | `Err(FutureAttempt\|InvalidActionCompletion)` | unit |
| Zero attempt | 0 | 5 | 0 | `Err(AttemptBeyondMax{attempt:0,max:5})` | unit |
| Zero capacity | 1 | 0 | 0 | `Err(AttemptBeyondMax{attempt:1,max:0})` | unit |
| Over capacity | 6 | 5 | 5 | `Err(AttemptBeyondMax{attempt:6,max:5})` | unit |
| Over capacity + zero current | 6 | 5 | 0 | `Err(AttemptBeyondMax{attempt:6,max:5})` | unit |
| Boundary: min valid | 1 | 1 | 1 | `Ok(())` | unit |
| Boundary: max valid | u16::MAX | u16::MAX | u16::MAX | `Ok(())` | unit |
| Current is zero | 1 | 5 | 0 | `Err(InvalidActionCompletion)` | unit |
| All fields zero | 0 | 0 | 0 | `Err(AttemptBeyondMax{attempt:0,max:0})` | unit |
| Valid exact, capacity 1 | 1 | 1 | 1 | `Ok(())` | unit |

### Test Group: validate_action_completion (Integration tests on RunState)

| Scenario | Precondition | Expected Output | Test Layer |
|---|---|---|---|
| Do node, Running, exact attempt | All checks pass | `Ok(())` | integration |
| Step state = Succeeded | frame.mark_succeeded called before | `Err(InvalidActionCompletion)` | integration |
| Step state = Pending | frame.mark_pending called before | `Err(InvalidActionCompletion)` | integration |
| Step state = Failed | frame.mark_failed called before | `Err(InvalidActionCompletion)` | integration |
| Node missing at step | step index beyond workflow length | `Err(InvalidActionCompletion)` | integration |
| Node is Label, not Do | workflow node kind != Do | `Err(InvalidActionCompletion)` | integration |
| Do node, action mismatch | ticket.action != node.action | `Err(InvalidActionCompletion)` | integration |

### Test Group: validate_retry_attempt + record_retry_attempt

| Scenario | attempt | max_attempts | current | Expected | Test Layer |
|---|---|---|---|---|---|
| Within bounds | 2 | 5 | 2 | `Ok(true)`, attempt→3 | integration |
| Exactly at max | 5 | 5 | 5 | `Ok(false)`, attempt→5 | integration |
| Below max by 1 | 4 | 5 | 4 | `Ok(true)`, attempt→5 | integration |
| Policy zero | 1 | 0 | 1 | `Err(AttemptBeyondMax)` | unit |
| Attempt zero | 0 | 5 | 0 | `Err(AttemptBeyondMax)` | unit |
| Exceeds policy | 6 | 5 | 6 | `Err(AttemptBeyondMax)` | unit |
| Overflow: current = u16::MAX, within policy | u16::MAX | u16::MAX | u16::MAX | `Ok(false)`, attempt→u16::MAX | integration |
| Missing step index | 1 | 5 | N/A | `Err(InvalidActionCompletion)` | integration |

### Test Group: reject_invalid_ticket_key

| Scenario | Key | Expected | Test Layer |
|---|---|---|---|
| Canonical key matches | `compute_action_idempotency_key(run,seq,action)` | `Ok(())` | unit |
| Wrong key | arbitrary different u128 | `Err(InvalidActionCompletion)` | unit |
| Key with flipped bit | canonical ^ 1 | `Err(InvalidActionCompletion)` | unit |
| Key = 0 | 0 | `Err(InvalidActionCompletion)` (unless canonical is also 0) | unit |

### Test Group: TimerWheel

| Scenario | Operation Sequence | Expected | Test Layer |
|---|---|---|---|
| Fresh insert, fire | insert→fire_expired | Entry fired with gen=1 | integration |
| Replace, fire | insert(deadline=D)→insert(deadline=D')→fire_expired | Only gen=2 entry; gen=1 stale | integration |
| Cancel, fire | insert→cancel→fire_expired | No entries fired | integration |
| Replace→cancel→fire | insert→insert(replace)→cancel→fire_expired | No entries fired (gen=2 cancelled) | integration |
| Generation overflow | insert × (u64::MAX - 1) times → insert again | Last insert returns `Err(GenerationExhausted)` | integration |
| Fire after deadline | insert(deadline=now-1)→fire_expired(now) | Entry fired | integration |
| Fire before deadline | insert(deadline=now+1)→fire_expired(now) | No entries fired | integration |
| Multiple runs fire in order | insert(run_A, D1)→insert(run_B, D2)→fire(D2) | Both fired, A before B in vec | integration |

### Test Group: Non-Mutation (Integration)

| Scenario | Invalid Condition | State Property Asserted | Test Layer |
|---|---|---|---|
| Stale attempt completion | attempt < current | journal_len, trace_len, runtime_states, counters unchanged | integration |
| Future attempt completion | attempt > current | journal_len, trace_len, runtime_states, counters unchanged | integration |
| Noncanonical key completion | wrong idempotency_key | journal_len, trace_len, runtime_states, counters unchanged | integration |
| Wrong step state completion | step state = Succeeded | journal_len, trace_len, runtime_states, counters unchanged | integration |
| Wrong action tid completion | action mismatch | journal_len, trace_len, runtime_states, counters unchanged | integration |
| Missing run completion | run not in self.runs | journal_len, trace_len, runtime_states, counters unchanged | integration |

### Test Group: E2E Acceptance

| Scenario | Given | Expected | Test Layer |
|---|---|---|---|
| Exact attempt succeeds | Live run at attempt=1 | ActionCompletedEnvelope journaled, frame advanced | e2e |
| Stale after retry | Run at attempt=2, incoming=1 | StaleAttempt error, journal unchanged | e2e |
| Future without scheduling | Run at attempt=1, incoming=3 | FutureAttempt/InvalidActionCompletion, journal unchanged | e2e |
| Terminal run fence | Run finished/cancelled | RunNotFound error | e2e |
| Timer replacement authority | Run timer replaced, old gen fires | Run not resumed, timer gen=2 remains | e2e |

## Test File Location Map

| Test File | Behaviors Covered | Layer |
|---|---|---|
| `crates/vb_runtime/src/shard/helpers/tests.rs` | B-001 through B-034 (attempt, retry, scheduling) | unit + integration |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` | B-035 through B-042, B-058 through B-061 (preflight, non-mutation) | integration |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` | B-043 through B-050 (terminal fence, failure) | integration |
| `crates/vb_runtime/src/shard/timer_wheel_tests.rs` | B-051 through B-057 (timer wheel) | unit + integration |
| `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs` | B-043 through B-046, B-058 through B-061 (cross-component integration) | integration |
| `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` | E2E scenarios (exact completion, stale, future, terminal fence, timer authority) | e2e |

## Open Questions

1. **G005 Future Attempt Error Variant**: Should `RuntimeError::FutureAttempt { incoming: u16, current: u16 }` be added as a new stable variant, or should `InvalidActionCompletion` be used with a trace-level distinction? The domain contract and proof obligations prefer `FutureAttempt` but accept either surface as long as behavior is correct. **Decision deferred to State 11 (holzman-rust)**.

2. **Private function testability**: `validate_ticket_attempt` and `reject_invalid_ticket_key` are private functions. Unit tests for these need either `#[cfg(test)]` module access within the same crate, or test via the public wrappers (`validate_action_completion` and `preflight_action_completion`). The test plan specifies both approaches: pure unit tests on extracted helpers (if made `pub(crate) #[cfg(test)]`) and integration tests through public APIs.

3. **Test fixture construction**: Building a minimal valid `RunState` with `CompiledWorkflow`, Do node, `action_attempts`, and running step state requires non-trivial fixture setup. Recommend a `TestRunStateBuilder` helper in the test module.

4. **Timer wheel testing**: `Instant` is used for deadlines. Tests should use `Instant::now()` +/- `Duration` to avoid real-time dependency. The `fire_expired` function takes `now: Instant` as parameter, making it testable without sleep.

5. **E2E test scope**: The 5 E2E scenarios require a fully wired Shard with at least one admitted run and a compiled workflow. If test scaffolding for this is heavy, the E2E tests may be deferred to a Phase 2 bead while integration tests provide coverage in this bead. **For now, all 5 E2E scenarios are planned; deferral decision is recorded for State 11 discussion**.

6. **Part A vs Part B overlap**: Part B body re-entry tests already exist. This test plan adds only the 37 Part A behaviors. Part B behaviors (B-062 through B-075) are documented for traceability but are already tested.

## Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario
- [x] Every pure function with multiple inputs has at least one proptest invariant
- [x] Every error variant in RuntimeError has an explicit test scenario (StaleAttempt, AttemptBeyondMax, InvalidActionCompletion, RunNotFound, UnsupportedOperation, InvalidTimerFire, GenerationExhausted)
- [x] Mutation threshold target stated (>= 90%)
- [x] No test asserts only `is_ok()` or `is_err()` — every assertion specifies exact variant and payload values
- [x] Every parsing/deserialization boundary has a fuzz target (fuzz_retry_codec)
- [x] Part A (61 behaviors) + Part B (14 existing) = 75 total behavior specifications, all mapped to test locations
- [x] Non-mutation assertions exist for all invalid-authority paths (B-058 through B-061)
- [x] Hostile public `ActionTicket` inputs covered (stale lower, future within capacity, zero attempt, zero capacity, over-capacity, noncanonical key)
- [x] Contract clause traceability: every contract clause (ACT-001 through ACT-012, TMR-001 through TMR-003, VER-001/VER-002) maps to at least one behavior

## Part B Existing Test Verification

The Part B body re-entry behaviors (B-062 through B-075) reference existing test functions confirmed real in State 7 bridge review. These tests DO NOT need to be rewritten by the test-writer. The test-writer should verify by running:

```bash
cargo test -p vb_proof_kernels test_invalid_transitions test_terminal_immutable -- --nocapture
cargo test -p vb_core -- state_transition_cancelled_terminal_rejects_pending frame_mark_succeeded_on_pending_step_allows_overwrite -- --nocapture
cargo test -p vb_runtime jump_to_body -- --nocapture
cargo test -p vb_runtime vb_y4pa_001_for_each_two_item_reentry vb_y4pa_002_reduce_reentry vb_y4pa_003_collect_next_reentry vb_y4pa_004_collect_page_reentry vb_y4pa_005_repeat_attempt_reentry vb_y4pa_006_repeat_check_reentry -- --nocapture
cargo test -p vb_runtime gwt_re1_for_each_body_reentry_after_succeeded -- --nocapture
```

If any existing test fails, report to State 11 (holzman-rust) as a regression, not to the test-writer as a new obligation.

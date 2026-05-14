# Test Plan: vb-qi37.16.3 — Durable Retry Transition

## Summary

- **Bead**: vb-qi37.16.3
- **Feature**: Durable retry transition for CLI/runtime
- **Behaviors identified**: 24 (16 contract clauses + 8 derived error paths)
- **Trophy allocation**: 14 unit / 10 integration / 2 E2E / 3 static
- **Proptest invariants**: 6
- **Fuzz targets**: 2 (ActionTicket deserialization, ActionFailure deserialization)
- **Kani harnesses**: 1

---

## 1. Behavior Inventory

### PRE-001 — Run existence validation
- `handle_action_failure_validates_run_exists`: Shard rejects action failure for unknown run with `RuntimeError::RunNotFound`

### PRE-002 — Ticket attempt bounds
- `validate_ticket_attempt_rejects_zero_attempt`: ticket with attempt=0 returns `Err(AttemptBeyondMax { attempt: 0, max: _ })`
- `validate_ticket_attempt_rejects_zero_capacity`: ticket with capacity=0 returns `Err(AttemptBeyondMax { attempt: _, max: 0 })`
- `validate_ticket_attempt_rejects_attempt_beyond_capacity`: ticket with attempt > capacity returns `Err(AttemptBeyondMax { attempt, max })`
- `validate_ticket_attempt_accepts_valid_attempt`: ticket with 1 <= attempt <= capacity returns `Ok(())`

### PRE-003 — Unknown run
- `handle_action_failure_unknown_run_returns_run_not_found`: calling handle_action_failure with non-existent run returns `Err(RuntimeError::RunNotFound)`

### PRE-004 — Retry availability preconditions
- `retry_is_available_requires_retryable_policy`: retry_is_available returns false when policy is NonRetryable
- `retry_metadata_exists_when_retry_check_follows`: retry_is_available returns false when step has no retry metadata

### POST-001 — PC reset on retry
- `apply_action_failure_to_state_sets_pc_to_failed_step_on_retry`: when retry_is_available is true, frame PC is set to ticket.step and returns `ActionFailureOutcome::RetryNow`

### POST-002 — Error handler drives to handler step
- `apply_error_handler_writes_error_slot_and_sets_pc_to_handler`: when retry unavailable and handler exists, error slot is written and PC set to handler, returning `DriveHandler`

### POST-003 — No handler fails run
- `apply_error_handler_returns_fail_run_when_no_handler`: when retry unavailable and no handler, returns `FailRun`
- `action_failure_without_handler_fails_run`: handle_action_failure for non-retryable without handler fails the run

### POST-004 — Journal event emission
- `action_failure_emits_action_failed_journal_event`: handle_action_failure emits exactly one `RuntimeJournalEvent::ActionFailed` to journal before returning
- `action_failure_without_handler_emits_action_failed_before_run_failed`: ActionFailed event emitted before run enters Failed state

### POST-005 — Retry capacity expansion
- `ticket_with_retry_capacity_returns_unchanged_when_no_retry_metadata`: ticket unchanged when retry_metadata_exists is false
- `ticket_with_retry_capacity_increases_capacity_to_max_attempts`: ticket.capacity = max(ticket.capacity, policy.max_attempts) when retryable and metadata exists

### POST-006 — Retry attempt recording
- `record_retry_attempt_increments_and_allows_retry`: after record_retry_attempt, action_attempts[step] >= ticket.attempt; returns `Ok(true)` when attempt < max
- `record_retry_attempt_blocks_when_max_reached`: when attempt >= max_attempts, returns `Ok(false)` and does not increment further

### POST-007 — Stale attempt rejection
- `stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged`: stale attempt (incoming < current) returns `StaleAttempt` error and leaves all run state unchanged
- `future_attempt_completion_rejected_when_current_attempt_exists`: attempt > current+1 returns `InvalidActionCompletion`

### INV-001 — Monotonic counter
- `record_scheduled_attempt_records_first_attempt`: action_attempts[step] set to 1 on first schedule
- `record_scheduled_attempt_updates_higher_attempt`: action_attempts[step] updated to max(current, new_attempt)
- `record_retry_attempt_increments_and_allows_retry`: record_retry_attempt increments counter (idempotent via max)

### INV-002 — Retry exhaustion
- `retry_exhaustion_journal`: after max_attempts failures with Retryable policy, next ActionFailed results in RunFailed; exactly max_attempts ActionFailed events in journal
- `action_failure_without_handler_fails_run`: exhaustion triggers FailRun outcome
- `retry_attempt_blocks_when_max_reached`: no further increments after max

### INV-003 — Journal idempotency
- `journal_replay_idempotent_action_failed`: replaying same ActionFailed event twice produces identical observable state (frame, counters) except duplicate in journal
- `action_failure_without_handler_emits_action_failed_before_run_failed`: ActionFailed appears in journal before run fails

### INV-004 — Slot preservation
- `stale_attempt_completion_leaves_frame_unchanged`: prior ActionCompleted slot writes are not overwritten by handle_action_failure

### INV-005 — PC reset semantics
- `apply_action_failure_to_state_sets_pc_to_failed_step_on_retry`: PC is reset to failed step (not advanced), proven by Verus

### Error taxonomy — UnsupportedOperation variants
- `retry_metadata_missing_error`: retry_metadata_exists returns false when no RetryCheck node follows
- `retry_policy_attempts_zero_error`: retry_policy_after_action returns `UnsupportedOperation("retry_policy_attempts_zero")` when max_attempts == 0
- `retry_policy_slot_unreadable_error`: unreadable policy slot returns `UnsupportedOperation("retry_policy_slot_unreadable")`
- `retry_attempt_overflow_error`: overflow of attempt counter returns `UnsupportedOperation("retry_attempt_overflow")`

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 14 | Pure functions: validate_ticket_attempt, record_retry_attempt, retry_is_available, retry_metadata_exists, retry_policy_after_action, ticket_with_retry_capacity, apply_error_handler, normalize_scheduled_ticket, advance_after_action_completion, find_error_handler_for_failure |
| Integration | 10 | handle_action_failure with real journal + shard state; journal replay with in-memory journal; retry exhaustion with real RunState |
| E2E | 2 | CLI `velvet-ballastics retry` command; full run lifecycle with retry |
| Static | 3 | clippy::forbidden-unsafe, no-unwrap on error paths, cargo-deny checks |

**Rationale**: The Calc layer (unit) dominates because the core retry logic is pure functions operating on ActionTicket, RunState, and RetryPolicy with no I/O. Integration tests cover the stateful interactions (journal append, PC reset, frame mutations). E2E is minimal — two scenarios: CLI retry command and full run lifecycle. Static analysis catches unsafe code, panics, and dbg! usage at compile time.

---

## 3. BDD Scenarios

### PRE-001: Run existence validation

**Scenario: handle_action_failure rejects unknown run**
```
Given: a Shard with no active runs
When: handle_action_failure is called with a ticket referencing a non-existent run
Then: returns Err(RuntimeError::RunNotFound)
And: no journal event is emitted
And: no run state is modified
```

**Scenario: handle_action_failure accepts valid run**
```
Given: a Shard with an active run in state Running at step S
When: handle_action_failure is called with a valid ticket for run, step S
Then: returns Ok(())
And: RuntimeJournalEvent::ActionFailed is appended to journal
```

---

### PRE-002: Ticket attempt bounds

**Scenario: validate_ticket_attempt rejects zero attempt**
```
Given: a RunState with action_attempts[step] = 0
When: validate_ticket_attempt is called with ticket.attempt = 0
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: _ })
```

**Scenario: validate_ticket_attempt rejects zero capacity**
```
Given: a RunState with action_attempts[step] = 0
When: validate_ticket_attempt is called with ticket.capacity = 0
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: _, max: 0 })
```

**Scenario: validate_ticket_attempt rejects attempt beyond capacity**
```
Given: a RunState with action_attempts[step] = 2
When: validate_ticket_attempt is called with ticket.attempt = 5, ticket.capacity = 3
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 5, max: 3 })
```

**Scenario: validate_ticket_attempt accepts valid bounds**
```
Given: a RunState with action_attempts[step] = 2
When: validate_ticket_attempt is called with ticket.attempt = 3, ticket.capacity = 5
Then: returns Ok(())
```

**Scenario: validate_ticket_attempt rejects stale attempt**
```
Given: a RunState with action_attempts[step] = 3
When: validate_ticket_attempt is called with ticket.attempt = 2
Then: returns Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })
```

**Scenario: validate_ticket_attempt rejects gap attempt**
```
Given: a RunState with action_attempts[step] = 2
When: validate_ticket_attempt is called with ticket.attempt = 4
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

---

### PRE-003: Unknown run

**Scenario: handle_action_failure with unknown run returns RunNotFound**
```
Given: a Shard with no runs
When: handle_action_failure(ticket=ActionTicket { run: unknown }, failure)
Then: returns Err(RuntimeError::RunNotFound)
And: no journal entry is appended
```

---

### PRE-004: Retry availability preconditions

**Scenario: retry_is_available returns false for NonRetryable policy**
```
Given: a RunState with retry metadata for step S
When: retry_is_available(state, ticket, VbCoreRetryPolicy::NonRetryable) is called
Then: returns Ok(false)
And: action_attempts is not modified
```

**Scenario: retry_is_available returns false when no retry metadata**
```
Given: a RunState without retry metadata for step S
When: retry_is_available(state, ticket, VbCoreRetryPolicy::Retryable) is called
Then: returns Ok(false)
And: action_attempts is not modified
```

**Scenario: retry_is_available returns true when retryable and metadata exists**
```
Given: a RunState with retry metadata for step S, action_attempts[S] = 1, max_attempts = 3
When: retry_is_available(state, ticket, VbCoreRetryPolicy::Retryable) is called
Then: returns Ok(true)
And: action_attempts[S] is not modified (increment belongs to record_retry_attempt)
```

---

### POST-001: PC reset on retry

**Scenario: apply_action_failure_to_state resets PC to failed step**
```
Given: a RunState at framePC = X (some step after S), step S is Running
And: action_attempts[S] = 1, max_attempts[S] = 3
When: apply_action_failure_to_state(ticket, failure=Retryable) is called
Then: returns ActionFailureOutcome::RetryNow
And: framePC is set to S (not advanced past S)
```

---

### POST-002: Error handler drives to handler step

**Scenario: apply_error_handler writes error slot and drives to handler**
```
Given: a RunState at step S with error handler at step H (error_slot = E)
When: apply_error_handler(state, ticket) is called
Then: returns ActionFailureOutcome::DriveHandler
And: error_slot[E] contains I64(S)
And: framePC is set to H
```

---

### POST-003: No handler fails run

**Scenario: apply_error_handler returns FailRun when no handler**
```
Given: a RunState at step S with no error handler in workflow
When: apply_error_handler(state, ticket) is called
Then: returns ActionFailureOutcome::FailRun
And: framePC is not modified
And: no slot is written
```

**Scenario: handle_action_failure fails run without handler**
```
Given: a Shard running step S with NonRetryable failure and no handler
When: handle_action_failure(ticket, failure=NonRetryable) is called
Then: run enters Failed state
And: exactly one RuntimeJournalEvent::ActionFailed is in journal
```

---

### POST-004: Journal event emission

**Scenario: handle_action_failure emits exactly one ActionFailed event**
```
Given: a Shard with an active run at step S
When: handle_action_failure(ticket, failure) is called
Then: exactly one RuntimeJournalEvent::ActionFailed { run, step: S, action } is appended
And: no other RuntimeJournalEvent is appended in the same call
```

**Scenario: ActionFailed event appears before run fails**
```
Given: a Shard with an active run at step S with no retry and no handler
When: handle_action_failure(ticket, failure) is called
Then: journal.last() == RuntimeJournalEvent::ActionFailed
And: then run enters Failed state
```

---

### POST-005: Retry capacity expansion

**Scenario: ticket_with_retry_capacity returns unchanged for non-retryable**
```
Given: a ticket with capacity=2
When: ticket_with_retry_capacity(ticket, NonRetryable) is called
Then: returns ticket with capacity unchanged (=2)
```

**Scenario: ticket_with_retry_capacity returns unchanged when no retry metadata**
```
Given: a ticket with capacity=2
And: step S has no retry metadata
When: ticket_with_retry_capacity(ticket, Retryable) is called
Then: returns ticket with capacity unchanged (=2)
```

**Scenario: ticket_with_retry_capacity expands capacity to max_attempts**
```
Given: a ticket with capacity=2
And: step S has retry metadata with max_attempts=5
When: ticket_with_retry_capacity(ticket, Retryable) is called
Then: returns ticket with capacity=5
```

**Scenario: ticket_with_retry_capacity keeps larger capacity**
```
Given: a ticket with capacity=7
And: step S has retry metadata with max_attempts=5
When: ticket_with_retry_capacity(ticket, Retryable) is called
Then: returns ticket with capacity=7
```

---

### POST-006: Retry attempt recording

**Scenario: record_retry_attempt increments counter below max**
```
Given: a RunState with action_attempts[S] = 1, policy.max_attempts = 3
When: record_retry_attempt(state, ticket{step=S, attempt=1}, policy) is called
Then: returns Ok(true)
And: action_attempts[S] == 2
```

**Scenario: record_retry_attempt returns false at max_attempts**
```
Given: a RunState with action_attempts[S] = 2, policy.max_attempts = 3
When: record_retry_attempt(state, ticket{step=S, attempt=2}, policy) is called
Then: returns Ok(false)
And: action_attempts[S] == 3
```

**Scenario: record_retry_attempt does not overflow past max_attempts+1**
```
Given: a RunState with action_attempts[S] = 3, policy.max_attempts = 3
When: record_retry_attempt(state, ticket{step=S, attempt=3}, policy) is called
Then: returns Ok(false)
And: action_attempts[S] == 3 (not 4)
```

---

### POST-007: Stale attempt rejection

**Scenario: stale attempt leaves all state unchanged**
```
Given: a RunState with action_attempts[S] = 3, framePC = X, frame at S is Running
When: validate_ticket_attempt is called with ticket.attempt = 2
Then: returns Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })
And: action_attempts[S] == 3 (unchanged)
And: framePC == X (unchanged)
And: frame slot values are unchanged
```

**Scenario: future attempt with gap is rejected**
```
Given: a RunState with action_attempts[S] = 2
When: validate_ticket_attempt is called with ticket.attempt = 4
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

---

### INV-001: Monotonic counter

**Scenario: action_attempts never decreases**
```
Given: a RunState with action_attempts[S] = N
When: record_retry_attempt or normalize_scheduled_ticket is called with attempt M >= N
Then: action_attempts[S] >= N after call
```

---

### INV-002: Retry exhaustion

**Scenario: after max_attempts failures, next failure fails the run**
```
Given: a run at step S with Retryable policy, max_attempts = 3
And: action_attempts[S] = 3 (exhausted)
When: handle_action_failure(ticket, Retryable) is called
Then: run enters Failed state
And: journal contains exactly 3 ActionFailed events for (run, S)
```

---

### INV-003: Journal idempotency

**Scenario: duplicate ActionFailed does not corrupt state**
```
Given: a journal with ActionFailed(run, S) already appended
When: journal replay appends ActionFailed(run, S) again
Then: observable state (frame, action_attempts, stepState) is identical to single-append
And: journal length is 2 (both events present)
```

---

### INV-004: Slot preservation

**Scenario: ActionCompleted slot values are not overwritten by ActionFailed**
```
Given: a run where step S has written slot 5 with value V via ActionCompleted
When: handle_action_failure is called for step S
Then: slot 5 still contains value V
And: no slot written by ActionCompleted is modified
```

---

### Error taxonomy scenarios

**Scenario: retry_policy_attempts_zero returns specific error**
```
Given: a step with RetryCheck node reading slot P containing I64(0)
When: retry_policy_after_action(state, step) is called
Then: returns Err(RuntimeError::UnsupportedOperation { operation: "retry_policy_attempts_zero" })
```

**Scenario: retry_metadata_missing returns specific error**
```
Given: a step S where node.next is None (no RetryCheck node)
When: retry_policy_after_action(state, S) is called
Then: returns Err(RuntimeError::UnsupportedOperation { operation: "retry_metadata_missing" })
```

**Scenario: retry_attempt_overflow returns specific error**
```
Given: action_attempts[S] = u16::MAX
When: record_retry_attempt is called
Then: returns Err(RuntimeError::UnsupportedOperation { operation: "retry_attempt_overflow" })
```

---

## 4. Proptest Invariants

### Proptest: `validate_ticket_attempt`
**Invariant**: For any valid RunState and ActionTicket where 1 <= ticket.attempt <= ticket.capacity, validate_ticket_attempt returns Ok(())
**Strategy**: (state_with_valid_attempts, attempt in 1..=capacity)
**Anti-invariant**: attempt == 0 -> AttemptBeyondMax; attempt > capacity -> AttemptBeyondMax; attempt < current -> StaleAttempt

### Proptest: `record_retry_attempt`
**Invariant**: After record_retry_attempt returns Ok(true), action_attempts[step] == old(action_attempts[step]) + 1
**Strategy**: (state, ticket with attempt >= current, policy.max_attempts > current)
**Anti-invariant**: attempt >= max_attempts -> Ok(false); attempt == 0 -> AttemptBeyondMax

### Proptest: `ticket_with_retry_capacity`
**Invariant**: After ticket_with_retry_capacity with Retryable + metadata, returned ticket.capacity == max(original.capacity, policy.max_attempts)
**Strategy**: (ticket.capacity in 0..=100, policy.max_attempts in 1..=100)
**Anti-invariant**: NonRetryable -> capacity unchanged; no metadata -> capacity unchanged

### Proptest: `normalize_scheduled_ticket`
**Invariant**: normalize_scheduled_ticket returns attempt >= 1 and attempt >= original.attempt
**Strategy**: (state with action_attempts[step] in 0..=10, ticket.attempt in 0..=10)
**Anti-invariant**: capacity == 0 -> AttemptBeyondMax; attempt > capacity -> AttemptBeyondMax

### Proptest: Journal replay idempotency
**Invariant**: Appending ActionFailed(run, step) twice produces same final state as appending once
**Strategy**: (run_id, step_idx, action, then replay twice)
**Anti-invariant**: Non-deterministic replay order -> state divergence

### Proptest: `retry_is_available` guards
**Invariant**: retry_is_available returns false iff (policy == NonRetryable OR !retry_metadata_exists)
**Strategy**: (state, policy in {Retryable, NonRetryable}, with/without retry metadata)

---

## 5. Fuzz Targets

### Fuzz Target: `ActionTicket` deserialization at `handle_action_failure` boundary
**Input type**: Arbitrary bytes -> ActionTicket struct reconstruction
**Risk**: Panic from out-of-bounds StepIdx, attempt overflow, capacity overflow, invalid RunId lookup
**Corpus seeds**: ticket with attempt=0, capacity=0, attempt>capacity, step beyond workflow node count, unknown run
**Target function**: `handle_action_failure(ticket: ActionTicket, failure: ActionFailure)`

### Fuzz Target: `ActionFailure` deserialization at `handle_action_failure` boundary
**Input type**: Arbitrary bytes -> ActionFailure struct
**Risk**: Invalid retry_policy discriminant, invalid taint value, panic from malformed detail string
**Corpus seeds**: failure with Retryable/NonRetryable, taint values, empty detail, malformed code
**Target function**: `handle_action_failure(ticket, failure: ActionFailure)`

---

## 6. Kani Harnesses

### Kani Harness: `validate_ticket_attempt` — PRE-002 arithmetic bounds
**Property**: For any (attempt: u16, capacity: u16, current: u16), validate_ticket_attempt returns Err(AttemptBeyondMax) when attempt == 0 OR capacity == 0 OR attempt > capacity; returns Err(StaleAttempt) when attempt < current; otherwise returns Ok(())
**Bound**: attempt in 0..=65535, capacity in 0..=65535, current in 0..=65535
**Rationale**: Arithmetic bounds must never overflow or panic. This is the primary entry gate for all retry logic — a panic here would corrupt run state. Proptest covers the happy paths; Kani covers the full u16 boundary.
**Evidence**: `cargo kani --package vb_runtime --harness validate_ticket_attempt --no-unwind`

---

## 7. Mutation Checkpoints

### Critical mutations to survive (cargo-mutants):

| Function | Mutation | Must be caught by test |
|----------|----------|----------------------|
| `validate_ticket_attempt` | Change `ticket.attempt == 0` to `ticket.attempt < 0` | `validate_ticket_attempt_rejects_zero_attempt` |
| `validate_ticket_attempt` | Change `ticket.attempt > ticket.capacity` to `>=` | `validate_ticket_attempt_rejects_attempt_beyond_capacity` |
| `validate_ticket_attempt` | Remove `ticket.attempt < current` check | `stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged` |
| `record_retry_attempt` | Change `*attempt >= policy.max_attempts` to `>` | `record_retry_attempt_blocks_when_max_reached` |
| `record_retry_attempt` | Remove overflow check | `retry_attempt_overflow_error` |
| `ticket_with_retry_capacity` | Change `max()` to min() | `ticket_with_retry_capacity_increases_capacity_to_max_attempts` |
| `retry_is_available` | Invert NonRetryable check | `retry_is_available_requires_retryable_policy` |
| `apply_action_failure_to_state` | Set PC to wrong step | `apply_action_failure_to_state_sets_pc_to_failed_step_on_retry` |
| `handle_action_failure` | Swap ActionFailed journal order before/after outcome | `action_failure_emits_action_failed_journal_event` |
| `find_error_handler_for_failure` | Return None when handler exists | `apply_error_handler_writes_error_slot_and_sets_pc_to_handler` |

**Threshold**: ≥ 90% mutation kill rate

---

## 8. Combinatorial Coverage Matrix

### Unit: `helpers::validate_ticket_attempt`

| Scenario | Input attempt | Input capacity | Current counter | Expected output | Layer |
|----------|-------------|---------------|----------------|-----------------|-------|
| zero attempt | 0 | 5 | 0 | Err(AttemptBeyondMax) | unit |
| zero capacity | 3 | 0 | 0 | Err(AttemptBeyondMax) | unit |
| attempt > capacity | 5 | 3 | 0 | Err(AttemptBeyondMax) | unit |
| stale attempt | 2 | 5 | 3 | Err(StaleAttempt) | unit |
| gap attempt | 4 | 5 | 2 | Err(InvalidActionCompletion) | unit |
| valid equal | 1 | 1 | 0 | Ok | unit |
| valid within | 3 | 5 | 2 | Ok | unit |
| valid at capacity | 5 | 5 | 0 | Ok | unit |
| valid resume | 3 | 5 | 3 | Ok | unit |
| overflow attempt u16::MAX | u16::MAX | u16::MAX | 0 | Err(AttemptBeyondMax) | unit |

### Unit: `helpers::record_retry_attempt`

| Scenario | Current | Attempt | max_attempts | Expected | Layer |
|----------|---------|---------|--------------|----------|-------|
| below max | 1 | 1 | 3 | Ok(true), counter=2 | unit |
| at max-1 | 2 | 2 | 3 | Ok(true), counter=3 | unit |
| at max | 3 | 3 | 3 | Ok(false), counter=3 | unit |
| above max | 4 | 4 | 3 | Ok(false), counter=4 | unit |
| overflow | u16::MAX | u16::MAX | 3 | Err(overflow) | unit |
| zero max_attempts | 1 | 1 | 0 | Err(UnsupportedOperation) | unit |

### Unit: `lifecycle::retry_is_available`

| Scenario | Policy | metadata_exists | current | max | Expected | Layer |
|----------|--------|-----------------|---------|-----|----------|-------|
| retryable + metadata | Retryable | true | 1 | 3 | Ok(true) | unit |
| NonRetryable | NonRetryable | true | 1 | 3 | Ok(false) | unit |
| no metadata | Retryable | false | 1 | 3 | Ok(false) | unit |
| exhausted | Retryable | true | 3 | 3 | Ok(false) | unit |

### Unit: `lifecycle::ticket_with_retry_capacity`

| Scenario | Policy | has_metadata | ticket.capacity | policy.max | Expected capacity | Layer |
|----------|--------|--------------|-----------------|------------|-------------------|-------|
| NonRetryable | NonRetryable | true | 2 | 5 | 2 | unit |
| no metadata | Retryable | false | 2 | 5 | 2 | unit |
| expand | Retryable | true | 2 | 5 | 5 | unit |
| keep larger | Retryable | true | 7 | 5 | 7 | unit |
| equal | Retryable | true | 5 | 5 | 5 | unit |

### Unit: `lifecycle::apply_error_handler`

| Scenario | handler_exists | error_slot | Expected outcome | PC after | Layer |
|----------|----------------|------------|-----------------|----------|-------|
| has handler | true | Some(E) | DriveHandler | handler step | unit |
| has handler, no slot | true | None | DriveHandler | handler step | unit |
| no handler | false | - | FailRun | unchanged | unit |

### Integration: `handle_action_failure` full flow

| Scenario | Policy | has_handler | attempts | max | Expected journal | Expected state | Layer |
|----------|--------|-------------|----------|-----|------------------|----------------|-------|
| retry now | Retryable | any | 1 | 3 | 1 ActionFailed | Running, PC=S | integration |
| drive handler | NonRetryable | true | any | any | 1 ActionFailed | Running, PC=H | integration |
| fail run | NonRetryable | false | any | any | 1 ActionFailed | Failed | integration |
| exhaustion | Retryable | false | 3 | 3 | 3 ActionFailed | Failed | integration |
| stale rejection | any | any | stale | - | none | unchanged | integration |
| double emit | any | any | any | any | exactly 1 ActionFailed per call | - | integration |

### Integration: Journal replay idempotency

| Scenario | Events in journal | Replay events | Final frame | Final attempts | Layer |
|----------|-------------------|---------------|-------------|---------------|-------|
| single append | [ActionFailed] | [ActionFailed] | Running | N | integration |
| duplicate append | [ActionFailed] | [ActionFailed, ActionFailed] | Running | N | integration |
| triple append | [ActionFailed] | [ActionFailed x3] | Running | N | integration |

---

## 9. Manual QA / BDD Expectations for Durable Retry

### CLI E2E: `velvet-ballastics retry` command

**Given** a run in `Failed` state with retryable step S  
**When** user runs `velvet-ballastics retry --run-id <id> --db <path>`  
**Then** the command returns exit code 0  
**And** the run transitions from `Failed` back to `Running` at step S  
**And** `velvet-ballastics status --run-id <id>` shows `Running`  

**Given** a run in `Failed` state with non-retryable step S and no handler  
**When** user runs `velvet-ballastics retry --run-id <id> --db <path>`  
**Then** the command returns exit code 1  
**And** `velvet-ballastics status --run-id <id>` shows `Failed`  

**Given** a run in `Failed` state with exhausted retries (max_attempts reached)  
**When** user runs `velvet-ballastics retry --run-id <id> --db <path>`  
**Then** the command returns exit code 1  
**And** `velvet-ballastics status --run-id <id>` shows `Failed`  

### CLI E2E: Journal replay after crash

**Given** a run that failed with ActionFailed event persisted in journal  
**When** runtime restarts and replays journal  
**Then** the run enters `Failed` state (not `Running`)  
**And** the ActionFailed event is replayed but does not increment attempt counter again  

### CLI E2E: Retry with error handler

**Given** a step S with error handler at step H  
**When** action at step S fails with NonRetryable policy  
**Then** run PC is set to H  
**And** error slot contains step index S  
**And** run continues from H  

---

## 10. Open Questions

1. **Journal replay determinism**: Does the in-memory journal used in integration tests have the same replay semantics as the Fjall-backed journal? If not, are there behavioral differences that need separate integration test coverage?
   - **Resolution needed**: Confirm journal abstraction boundary. If Fjall journal has fsync ordering semantics, add separate integration test with real Fjall.

2. **ActionTicket construction path**: The contract says ticket must be valid, but who constructs the ticket? If the engine constructs it, what prevents a malicious or corrupted ticket from reaching `handle_action_failure`? Should there be a test for a ticket with a step index beyond the workflow node count?
   - **Resolution needed**: Confirm ticket is always engine-validated before reaching shard. If external callers exist, add fuzz test for out-of-bounds step.

3. **Frame slot type coercion**: `write_failure_slot` writes `I64(step.get())`. Is there a test that slot E already contains a value of a different type? What happens if slot E is the same slot written by ActionCompleted?
   - **Resolution needed**: Confirm INV-004 holds even when error_slot == slot written by prior action. Add integration test for this collision case.

4. **CLI integration test isolation**: Do `velvet-ballastics retry` E2E tests require a real database, or can they use an in-memory substitute? If real DB is required, how do we ensure CI stability?
   - **Resolution needed**: Confirm test DB strategy for E2E tests. If real Fjall instance needed, add cleanup procedure.

---

## 11. Verification Gate Commands

| Gate ID | Command | Expected evidence | Layer |
|---------|---------|-------------------|-------|
| GATE-PROOF-001 | `moon run :verify-proof` | exits 0, all proof obligations PASS/WAIVED | gauntlet |
| GATE-STANDARD-001 | `moon run :verify-standard` | exits 0, all standard tests pass | gauntlet |
| UNIT-LIFECYCLE-001 | `cargo test -p vb_runtime apply_error_handler --lib` | test action_failure_without_handler_fails_run passes | unit |
| INTEGRATION-RETRY-001 | `cargo test -p vb_runtime retry --test '*'` | retry_exhaustion_journal passes with 2 ActionFailed events | integration |
| INTEGRATION-JOURNAL-001 | `cargo test -p vb_runtime journal_replay --test '*'` | journal replay idempotency tests pass | integration |
| INTEGRATION-STALE-001 | `cargo test -p vb_runtime stale_attempt --lib` | stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged passes | integration |
| TLA-RETRY-001 | `tlc -config specs/RetryFSM.cfg specs/RetryFSM.tla` | TLC reports no invariant violations | tla-plus |
| TLA-RETRY-002 | `tlc -config specs/RetryJournal.cfg specs/RetryJournal.tla` | TLC reports no invariant violations for JournalIdempotency | tla-plus |
| TLA-RETRY-003 | `tlc -config specs/RetryJournal.cfg specs/RetryJournal.tla` | TLC reports EventuallyJournalAppended satisfied | tla-plus |
| VERUS-PRE-002 | `verus crates/vb_runtime/src/shard/helpers.rs` | Verus verified validate_ticket_attempt with 0 errors | verus |
| VERUS-INV-001 | `verus crates/vb_runtime/src/shard/helpers.rs` | Verus verified record_retry_attempt loop invariant with 0 errors | verus |
| VERUS-POST-006 | `verus crates/vb_runtime/src/shard/helpers.rs` | Verus verified record_retry_attempt postconditions with 0 errors | verus |
| VERUS-POST-001 | `verus crates/vb_runtime/src/shard/lifecycle.rs` | Verus verified apply_action_failure_to_state PC reset with 0 errors | verus |
| VERUS-PRE-004 | `verus crates/vb_runtime/src/shard/lifecycle.rs` | Verus verified retry_is_available precondition with 0 errors | verus |
| KANI-PRE-002 | `cargo kani --package vb_runtime --harness validate_ticket_attempt --no-unwind` | Kani reports no overflow or panic | kani |

---

## 12. Test File Locations

| Test group | File location | Test type |
|-----------|---------------|-----------|
| validate_ticket_attempt bounds | `crates/vb_runtime/src/shard/helpers.rs` (existing tests at line ~1049) | unit |
| record_retry_attempt | `crates/vb_runtime/src/shard/helpers.rs` (existing tests at line ~1049) | unit |
| retry_is_available | `crates/vb_runtime/src/shard/lifecycle.rs` | unit |
| ticket_with_retry_capacity | `crates/vb_runtime/src/shard/lifecycle.rs` | unit |
| apply_error_handler | `crates/vb_runtime/src/shard/lifecycle.rs` | unit |
| handle_action_failure | `crates/vb_runtime/src/shard/lifecycle.rs` | integration |
| journal replay idempotency | `crates/vb_runtime/src/journal/` | integration |
| stale attempt rejection | `crates/vb_runtime/src/shard/lifecycle.rs` | integration |
| CLI retry command | `crates/velvet_ballastics/tests/cli_integration.rs` | E2E |
| full run lifecycle | `crates/vb_runtime/tests/` | E2E |

---

*Plan produced by test-planner for vb-qi37.16.3. All 16 contract clauses have traceability entries. No test asserts only `is_ok()` or `is_err()` without specifying the value. Mutation threshold: ≥ 90%.*

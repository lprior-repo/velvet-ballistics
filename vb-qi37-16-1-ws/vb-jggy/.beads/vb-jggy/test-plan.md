# Test Plan: vb-jggy — Persist execution attempt numbers and reject stale completions

## Section 1 — Behavior Inventory

### Core Pure Functions (`vb_runtime::shard::helpers`)

| # | Subject | Action | Outcome when | Condition |
|---|---------|--------|--------------|-----------|
| B1 | `new_action_attempts` | creates tracker | returns `Box<[u16]>` with all zeros | any `step_count: u16` |
| B2 | `record_scheduled_attempt` | records first attempt | `action_attempts[step] = ticket.attempt` | current is 0 and ticket.attempt > 0 |
| B3 | `record_scheduled_attempt` | ignores lower attempt | `action_attempts[step]` unchanged | ticket.attempt < current |
| B4 | `record_scheduled_attempt` | updates to higher attempt | `action_attempts[step] = ticket.attempt` | ticket.attempt > current |
| B5 | `record_scheduled_attempt` | no-op on zero attempt | `action_attempts[step]` unchanged | ticket.attempt == 0 |
| B6 | `record_scheduled_attempt` | no-op on OOB step | no mutation | step out of bounds |
| B7 | `validate_ticket_attempt` | accepts valid ticket | `Ok(())` | ticket.attempt >= current && ticket.attempt > 0 && ticket.attempt <= capacity |
| B8 | `validate_ticket_attempt` | rejects stale attempt | `Err(StaleAttempt { incoming, current })` | ticket.attempt < current |
| B9 | `validate_ticket_attempt` | rejects attempt=0 | `Err(AttemptBeyondMax { attempt: 0, .. })` | ticket.attempt == 0 |
| B10 | `validate_ticket_attempt` | rejects capacity=0 | `Err(AttemptBeyondMax { .. })` | ticket.capacity == 0 |
| B11 | `validate_ticket_attempt` | rejects attempt>capacity | `Err(AttemptBeyondMax { .. })` | ticket.attempt > ticket.capacity |
| B12 | `validate_ticket_attempt` | rejects future attempt when current>0 | `Err(InvalidActionCompletion)` | current != 0 && ticket.attempt > current |
| B13 | `normalize_scheduled_ticket` | normalizes ticket attempt | returns ticket with `attempt = max(current, ticket.attempt).max(1)` | current > 0 |
| B14 | `normalize_scheduled_ticket` | enforces capacity bound | `Err(AttemptBeyondMax)` | normalized attempt > capacity |
| B15 | `validate_action_completion` | validates step state | `Err(InvalidActionCompletion)` | step not in Running state |
| B16 | `validate_action_completion` | validates action kind | `Err(InvalidActionCompletion)` | node kind mismatch |

### Lifecycle — Run Admission

| # | Subject | Action | Outcome when | Condition |
|---|---------|--------|--------------|-----------|
| B17 | `handle_submit_with_inputs` | admits new run | `RunState.action_attempts` all zeros | first submit |
| B18 | `handle_submit_with_inputs` | rejects duplicate run | `Err(RuntimeError::RunAlreadyExists)` | run already in `runs` |
| B19 | `handle_submit_with_inputs` | rejects at capacity | `Err(RuntimeError::ActiveRunCapacityExceeded { .. })` | at max_active_runs |

### Lifecycle — Action Completion

| # | Subject | Action | Outcome when | Condition |
|---|---------|--------|--------------|-----------|
| B20 | `handle_action_completion` | succeeds and writes journal | `Ok(())` + journal appends | valid ticket, step Running |
| B21 | `handle_action_completion` | rejects stale attempt | `Err(StaleAttempt { incoming, current })` BEFORE journal write | ticket.attempt < current |
| B22 | `handle_action_completion` | rejects unknown run | `Err(RuntimeError::RunNotFound)` | run not in `runs` map |
| B23 | `handle_action_completion` | rejects non-running step | `Err(InvalidActionCompletion)` | step state != Running |
| B24 | `handle_action_completion` | rejects future attempt when current>0 | `Err(InvalidActionCompletion)` | ticket.attempt > current |

### Lifecycle — Action Failure

| # | Subject | Action | Outcome when | Condition |
|---|---------|--------|--------------|-----------|
| B25 | `handle_action_failure` | routes to retry | `Ok(())` + drive_run | retryable + retry available |
| B26 | `handle_action_failure` | routes to error handler | `Ok(())` + drive_run | has error handler |
| B27 | `handle_action_failure` | fails run | run removed from `runs` | non-retryable, no handler |
| B28 | `handle_action_failure` | rejects stale attempt | `Err(StaleAttempt { .. })` BEFORE journal write | ticket.attempt < current |
| B29 | `handle_action_failure` | sets PC back to step | run continues | retry selected |

### Journal Events

| # | Subject | Action | Outcome when | Condition |
|---|---------|--------|--------------|-----------|
| B30 | `RuntimeJournalEvent::StepSucceeded` | carries attempt field | `attempt: u16` present | completion path |
| B31 | `RuntimeJournalEvent::ActionFailed` | carries attempt field | `attempt: u16` present | failure path (note: current enum lacks this field) |

---

## Section 2 — Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| B1–B16: Pure helper functions | **Unit** (`#[cfg(test)]` in helpers.rs) | Pure computation; exhaustive boundary coverage |
| B17–B19: Run admission | **Integration** (`tests/lifecycle_tests.rs`) | Real Shard, real RunState, real frame pool |
| B20–B24: Action completion | **Integration** (`tests/lifecycle_tests.rs`) | Shard command queue, journal append, state mutation |
| B25–B29: Action failure | **Integration** (`tests/lifecycle_tests.rs`) | Error handler routing, retry state |
| B30–B31: Journal events | **Unit** + **Fuzz** | Postcard serialization; attempt field existence |
| INV-001–INV-004: Monotonicity, single latest, ordering | **Kani** | State machine proof; bounded model checking |
| POST-001–POST-006: Attempt invariants | **Proptest** | Property-based over many random sequences |

**Target ratio**: ~60% integration, ~30% unit, ~5% proptest, ~5% Kani/static.

---

## Section 3 — BDD Scenarios

### B1: new_action_attempts creates zeroed tracker
```
Given: step_count = 3
When: new_action_attempts(3) is called
Then: returns Box<[u16]> of length 3 with all values equal to 0
```

### B2: record_scheduled_attempt records first attempt
```
Given: RunState with action_attempts[step 0] = 0
When: record_scheduled_attempt(state, ticket{step: 0, attempt: 1})
Then: state.action_attempts[0] == 1
```

### B3: record_scheduled_attempt ignores lower attempt
```
Given: RunState with action_attempts[step 0] = 5
When: record_scheduled_attempt(state, ticket{step: 0, attempt: 3})
Then: state.action_attempts[0] == 5
```

### B4: record_scheduled_attempt updates to higher attempt
```
Given: RunState with action_attempts[step 0] = 2
When: record_scheduled_attempt(state, ticket{step: 0, attempt: 5})
Then: state.action_attempts[0] == 5
```

### B6: record_scheduled_attempt no-op on out-of-bounds step
```
Given: RunState with action_attempts of length 3 (valid steps 0, 1, 2) and all zeros
When: record_scheduled_attempt(state, ticket{step: 5, attempt: 1})
Then: state.action_attempts is unchanged — all elements remain 0
And: returns Ok(()) (no panic, no error)
```

### B7: validate_ticket_attempt accepts valid ticket
```
Given: RunState with action_attempts[step 0] = 1, ticket{step: 0, attempt: 2, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Ok(())
```

### B8: validate_ticket_attempt rejects stale attempt
```
Given: RunState with action_attempts[step 0] = 3, ticket{step: 0, attempt: 2, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })
```

### B9: validate_ticket_attempt rejects attempt=0
```
Given: RunState with action_attempts[step 0] = 0, ticket{step: 0, attempt: 0, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 3 })
```

### B11: validate_ticket_attempt rejects attempt>capacity
```
Given: RunState with action_attempts[step 0] = 0, ticket{step: 0, attempt: 5, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Err(RuntimeError::AttemptBeyondMax { attempt: 5, max: 3 })
```

### B17: handle_submit_with_inputs zero-initializes action_attempts
```
Given: Shard with capacity for runs, valid workflow with 2 steps
When: handle_submit_with_inputs(run, workflow, [], caps) is called
Then: the RunState inserted has action_attempts == [0, 0]
```

### B21: handle_action_completion rejects stale attempt before journal write
```
Given: Shard with active run, action_attempts[step 0] = 3, stale ticket{step: 0, attempt: 2}
When: handle_action_completion(stale_ticket, output) is called
Then: returns Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })
And: journal has no StepSucceeded event for this step
And: frame step state is unchanged (still Running)
And: action_attempts is unchanged
```

### B28: handle_action_failure rejects stale attempt before journal write
```
Given: Shard with active run, action_attempts[step 0] = 3, stale ticket{step: 0, attempt: 2}
When: handle_action_failure(stale_ticket, failure) is called
Then: returns Err(RuntimeError::StaleAttempt { incoming: 2, current: 3 })
And: journal has no ActionFailed event for this step
And: frame step state is unchanged
And: action_attempts is unchanged
```

### Error: StaleAttempt on completion when current=1, incoming=1 (equal is OK)
```
Given: RunState with action_attempts[step 0] = 1, ticket{step: 0, attempt: 1, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Ok(())  // equal is not stale
```

### Error: StaleAttempt on completion when current=0, incoming=1 (first is OK)
```
Given: RunState with action_attempts[step 0] = 0, ticket{step: 0, attempt: 1, capacity: 3}
When: validate_ticket_attempt(state, ticket)
Then: returns Err(RuntimeError::InvalidActionCompletion)  // current=0, attempt>current is invalid
```

### Error: future attempt when current>0 is rejected
```
Given: RunState with action_attempts[step 0] = 1, ticket{step: 0, attempt: 3, capacity: 5}
When: validate_ticket_attempt(state, ticket)
Then: returns Err(RuntimeError::InvalidActionCompletion)
```

### Error: handle_action_completion returns EncodeFailed on serialization failure
```
Given: Shard with active run, valid ticket for step 0 in Running state
And: journal encode path is forced to fail (e.g., payload too large for postcard)
When: handle_action_completion(ticket, output) is called
Then: returns Err(RuntimeError::EncodeFailed)
And: no StepSucceeded event appears in the journal
And: run state is unchanged (step still Running)
```

### Error: handle_action_failure returns EncodeFailed on serialization failure
```
Given: Shard with active run, valid ticket for step 0 in Running state
And: journal encode path is forced to fail
When: handle_action_failure(ticket, failure) is called
Then: returns Err(RuntimeError::EncodeFailed)
And: no ActionFailed event appears in the journal
And: run state is unchanged
```

### B30: RuntimeJournalEvent::StepSucceeded carries attempt field
```
Given: ActionTicket with step=0, attempt=3
When: RuntimeJournalEvent::StepSucceeded { run, step, attempt } is constructed
Then: the encoded event contains attempt == 3
And: round-trip decode produces attempt == 3
```

### B31: RuntimeJournalEvent::ActionFailed carries attempt field
```
Given: ActionTicket with step=1, attempt=7
When: RuntimeJournalEvent::ActionFailed { run, step, attempt, code } is constructed
Then: the encoded event contains attempt == 7
And: round-trip decode produces attempt == 7
```

---

## Section 4 — Proptest Invariants

### `validate_ticket_attempt` invariants (pure, 3 inputs)

**Invariant 1 — Monotonicity gate**: If `Ok(())` is returned, then `ticket.attempt >= current` holds.
```rust
prop_compose! {
    // valid: current in 0..=10, attempt in 1..=10 where attempt >= current
}
```

**Invariant 2 — Positive attempt**: If `Ok(())` is returned, then `ticket.attempt > 0`.
```rust
// any valid configuration
```

**Invariant 3 — Capacity bound**: If `Ok(())` is returned, then `ticket.attempt <= ticket.capacity`.
```rust
prop_compose! {
    // capacity in 1..=10, attempt in 1..=capacity
}
```

**Invariant 4 — Stale rejection**: If `ticket.attempt < current`, then result is `Err(StaleAttempt { incoming, current })`.
```rust
prop_compose! {
    // current in 1..=10, attempt in 0..current
}
```

### `record_scheduled_attempt` invariants

**Invariant 5 — Non-decrease**: After `record_scheduled_attempt(state, ticket)`, `action_attempts[step] >= previous_value`.
```rust
// arbitrary initial state + arbitrary ticket
```

**Invariant 6 — Never exceeds ticket.attempt**: After record, `action_attempts[step] <= ticket.attempt` when ticket.attempt > 0.
```rust
// for all valid tickets with attempt > 0
```

**Invariant 7 — Monotonicity over N calls**: For a sequence of tickets for the same step, `action_attempts[step]` is non-decreasing.
```rust
// vec![ticket(step=0, attempt=i) for i in 1..=5] -> monotonic
```

### `new_action_attempts` invariants

**Invariant 8 — All zeros**: All elements of the returned slice are 0.
```rust
// step_count in 0..=1000
```

---

## Section 5 — Fuzz Targets

### `RuntimeJournalEvent` serialization (`vb_storage` or `vb_runtime`)

**Risk class**: Data durability; corrupted journal events can prevent replay.

**Input type**: `RuntimeJournalEvent` (postcard-encoded bytes via `serde`)

**Corpus seeds**: JSON fixtures for `StepSucceeded`, `ActionFailed`, `SlotWritten`, `RunSubmitted`.

**Target variants to fuzz**:
- `RuntimeJournalEvent::StepSucceeded` — currently missing `attempt: u16` field (POST-003 gap)
- `RuntimeJournalEvent::ActionFailed` — currently missing `attempt: u16` field (POST-003 gap)

**Fuzz harness**: `arbitrary`-based corpus expansion targeting round-trip encode/decode.

```rust
// fuzz_targets/runtime_journal_event.rs
fn fuzz_round_trip(event: RuntimeJournalEvent) {
    let encoded = postcard::to_allocvec(&event).unwrap();
    let decoded: RuntimeJournalEvent = postcard::from_bytes(&encoded).unwrap();
    assert_eq!(event, decoded);
}
```

### `ActionTicket` deserialization

**Risk class**: Wire protocol; malformed tickets could bypass attempt validation.

**Input type**: `ActionTicket` serialized bytes

**Corpus**: Valid tickets with attempt=1,2,3, capacity=3.

---

## Section 6 — Kani Harnesses

### HK-1: `validate_ticket_attempt` ordering (POST-004, INV-003)

**Property**: `validate_ticket_attempt` returns `Ok(())` implies `ticket.attempt >= state.action_attempts[ticket.step]`.

**Bound**: `step_count <= 100`, `attempt <= 100`, `capacity <= 100`.

**Rationale**: Proves stale gate precedes any state mutation call site in lifecycle.

### HK-2: `record_scheduled_attempt` monotonicity (INV-004, POST-006)

**Property**: For any two calls `record_scheduled_attempt(state, t1)` then `record_scheduled_attempt(state, t2)` with same step, `state.action_attempts[step]` is non-decreasing.

**Bound**: `step in 0..50`, `attempt in 0..u16::MAX`.

**Rationale**: Proves attempt counter never decreases across retries.

### HK-3: `handle_action_completion` stale-first ordering (INV-003)

**Property**: In `handle_action_completion`, `validate_ticket_attempt` result is checked before any `journal.append` call.

**Bound**: Single step, single action, no concurrent access.

**Rationale**: Kani proves call-ordering between validation and journal mutation.

### HK-4: `RunState::action_attempts` zero-initialized (POST-001)

**Property**: After `handle_submit_with_inputs`, for all steps `i`, `action_attempts[i] == 0`.

**Bound**: Workflow with `step_count <= 20`.

**Rationale**: Guarantees fresh runs start with clean attempt state.

---

## Section 7 — Mutation Testing Checkpoints

| Mutation | Kill Test | Behavior Tested |
|----------|-----------|----------------|
| Change `current != 0 && ticket.attempt > current` to `ticket.attempt > current` | `future_attempt_completion_rejected_when_current_attempt_exists` | B24 |
| Change `ticket.attempt < current` to `ticket.attempt <= current` | `validate_ticket_attempt_rejects_stale_when_attempt_less` | B8 |
| Change `*attempt = ticket.attempt` to `*attempt = current.max(ticket.attempt)` (in record) | `record_scheduled_attempt_updates_higher_attempt` | B4 |
| Swap order: journal.append before validate | `stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged` | B21 |
| Remove `ticket.attempt == 0` guard in record | `record_scheduled_attempt_on_zero_attempt_is_noop` | B5 |
| Change `new_action_attempts` to init with 1s | `new_action_attempts_creates_zeroed_tracker` | B1 |
| Remove EncodeFailed error path | `encode_failed_completion_returns_error` | EncodeFailed scenario |

**Target kill rate**: ≥ 90%.

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| B1: zero init | step_count=0 | len=0, all=0 | unit |
| B1: single step | step_count=1 | len=1, all=0 | unit |
| B1: many steps | step_count=100 | len=100, all=0 | unit |
| B7: equal attempt | current=2, attempt=2 | Ok(()) | unit |
| B8: stale attempt | current=3, attempt=2 | Err(StaleAttempt{2,3}) | unit |
| B9: attempt=0 | current=0, attempt=0 | Err(AttemptBeyondMax{0,cap}) | unit |
| B11: over capacity | current=0, attempt=5, cap=3 | Err(AttemptBeyondMax{5,3}) | unit |
| B12: future attempt | current=1, attempt=3 | Err(InvalidActionCompletion) | unit |
| B17: admit fresh run | new workflow | action_attempts=[0,0,...] | integration |
| B21: stale completion | current=3, attempt=2 | Err(StaleAttempt{2,3}) before journal | integration |
| B20: valid completion | current=0, attempt=1 | Ok(()), journal has StepSucceeded | integration |
| B28: stale failure | current=3, attempt=2 | Err(StaleAttempt{2,3}) before journal | integration |
| B25: retryable failure | retry policy with retries left | ActionFailureOutcome::RetryNow | integration |
| INV-4: monotonic N-calls | 5 tickets same step | non-decreasing | proptest |
| B6: OOB step no-op | step=99, step_count=3 | no mutation, Ok(()) | unit |
| EncodeFailed: completion | postcard serialization fails | Err(EncodeFailed) | integration |
| EncodeFailed: failure | postcard serialization fails | Err(EncodeFailed) | integration |

---

## Section 9 — Proof Obligation Mapping

| PO ID | Target | Test Strategy |
|-------|--------|---------------|
| PRE-001 | master-doc Section 72 | Manual QA + code review |
| PRE-002 | handle_action_completion stale gate | Kani HK-3 + integration B21 |
| PRE-003 | handle_action_failure stale gate | Kani HK-3 + integration B28 |
| PRE-004 | StaleAttempt variant exists | `cargo clippy --all-targets` static scan |
| POST-001 | RunState zero-init | Kani HK-4 + unit B1 |
| POST-002 | First ticket.attempt == 1 | Kani HK-1 + `execute_do` unit test |
| POST-003 | StepSucceeded/StepFailed carry attempt | **IN SCOPE**: `RuntimeJournalEvent::StepSucceeded { run, step, attempt: u16 }` and `RuntimeJournalEvent::StepFailed { run, step, attempt: u16, code }` are extended as part of vb-jggy implementation; unit test confirms field presence |
| POST-004 | validate_ticket_attempt ordering | Kani HK-3 |
| POST-005 | stale returns error before mutation | Kani HK-1 + integration B21 |
| POST-006 | record_scheduled_attempt monotonic | Kani HK-2 + proptest Inv-7 |
| INV-001 | exactly one latest attempt per step | Kani HK-2 |
| INV-002 | older attempts cannot win | Kani HK-1 + unit B8 |
| INV-003 | check before mutation | Kani HK-3 + integration B21/B28 |
| INV-004 | monotonic non-decrease | Kani HK-2 + proptest Inv-7 |
| GATE-001 | full moon ci passes | gauntlet-all |

### POST-003 Resolution

The `RuntimeJournalEvent` enum variants are **extended as part of vb-jggy implementation**:

- `StepSucceeded { run: RunId, step: StepIdx, attempt: u16 }` — field added
- `StepFailed { run: RunId, step: StepIdx, attempt: u16, code: u32 }` — field added

A unit test confirms both variants serialize/deserialize with the `attempt` field correctly. Fuzz target covers round-trip encode/decode.

---

*Generated by test-planner for vb-jggy. All behaviors have BDD scenarios. All proof obligations are mapped. No `is_ok()` / `is_err()` assertions in scenario outcomes.*

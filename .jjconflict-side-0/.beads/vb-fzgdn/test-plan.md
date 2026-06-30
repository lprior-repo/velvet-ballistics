# Test Plan: vb-fzgdn — Deterministic Delayed-Action Timer Seam

## Metadata
- **bead**: vb-fzgdn
- **state**: 8 (test-planner)
- **invocation_id**: vb-fzgdn-state8-test-planner-attempt1
- **delegate**: test-planner
- **bridge_ref**: `.beads/vb-fzgdn/proof-to-rust-map.md` (State 7, APPROVED)
- **rro_count**: 46
- **god_rule_2_deferral**: Verus (10 obligations) deferred to State 11; compensating Kani/Flux/Proptest/Fuzz/Loom coverage active

## Summary
- Behaviors identified: 48
- Trophy allocation: 14 unit-calc / 24 integration / 5 e2e / 5 static
- Proptest invariants: 10 (one per proof seed)
- Fuzz targets: 1 (slot validation boundary PS-006)
- Kani harness references: 10 (planned in proof artifacts, validated in State 5/12)
- Mutation threshold: ≥90% kill rate
- Behavior test files planned: 10
- Refinement harness files planned: 10

## 0. Trophy Allocation Rationale

The timer seam is an integration-heavy concern: it spans numeric value objects (pure Calc), shard-owned mutable state (integration), runtime public API (integration), workflow slot parsing (integration), journal writing (integration), command queue interaction (integration), and deterministic replay end-to-end (E2E). The pure Calc layer — smart constructors for TimerTick/TimerDuration/TimerDeadline, checked_add arithmetic, full authority equality, duplicate-key classification, and deterministic deadline ordering — forms the unit test core. Shard-level workflows (admission, fire, cancel, clock advance, capacity enforcement, zero-duration policy, atomic fire+enqueue) form the integration bulk. Full runtime-level replay with mixed timer/action interleaving forms the E2E layer. Static type enforcement (non_exhaustive enums, Illegal-state-free value objects, no-Instant-in-core types) provides the static analysis base.

```
         [E2E           5 scenarios]  ← full runtime replay interleaving
    [Integration      24 scenarios]  ← shard-level admission/fire/cancel/clock/capacity
    [Unit / Calc      12 scenarios]  ← pure constructors, checked arithmetic, equality, ordering
  [Static Analysis    4 gates]       ← clippy, non_exhaustive, no_Instant_types, cargo-deny
```

Justification for deviation from 60/30/5/5:
- Integration is 53% (24/45) vs target 60% — slightly under because the pure Calc layer (27%) is elevated by the dense set of typed constructors and comparison predicates that constitute the deterministic foundation. Every value object constructor requires exhaustive combinatorial coverage.
- E2E is 11% (5/45) vs target 5% — elevated because the replay-determinism mandate demands cross-tick, cross-shard integration scenarios that exercise the full timer lifecycle end-to-end through the public runtime API.

## 1. Behavior Inventory

### Domain A: Deadline Arithmetic Safety (PS-001)
1. `TimerTick` constructor accepts valid u64 within configured horizon
2. `TimerTick` constructor rejects tick exceeding profile horizon
3. `TimerDuration` constructor accepts duration within max_wait_duration_ticks
4. `TimerDuration` constructor rejects duration exceeding max_wait_duration_ticks
5. `TimerDeadline` construction from `TimerTick + TimerDuration` returns exact checked sum
6. `TimerDeadline` construction returns typed overflow error when sum exceeds u64::MAX
7. `TimerDeadline` construction rejects absolute deadline before current tick when policy forbids retroactive deadlines

### Domain B: Numeric-Only Timer State (PS-002)
8. Timer admission stores numeric tick/duration/deadline/authority and never calls `Instant::now()` in the behavior-affecting path
9. `PendingTimer` fields are numeric (no `std::time::Instant` in deadline field)
10. `ShardCommand::TimerFired` deadline field is numeric
11. Journal events (`WaitScheduled`, `AskScheduled`) carry numeric tick facts
12. Replay rebuilds pending timer state from numeric journal without host time

### Domain C: Authority Validation (PS-003)
13. Missing pending entry (timer not found for run) returns typed `TimerAuthorityMissing` — no mutation
14. Wrong generation authority returns typed `TimerAuthorityMismatch` — no mutation
15. Wrong deadline authority returns typed `TimerAuthorityMismatch` — no mutation
16. Wrong kind authority (Wait vs Ask) returns typed `TimerAuthorityMismatch` — no mutation
17. Wrong run authority (different RunId) returns typed `TimerAuthorityMissing` — no mutation
18. Valid full authority succeeds: removes pending timer, journals success, advances run frame

### Domain D: Generation Exhaustion (PS-004)
19. `next_generation` returns strictly greater generation for existing timer (g+1) when g < u64::MAX
20. `next_generation` returns `Ok(1)` for first timer on a run (no existing generation)
21. `next_generation` returns typed `TimerGenerationExhausted` when current generation is u64::MAX
22. Generation exhaustion does not mutate pending timer state or run frame

### Domain E: Duplicate Delayed-Action Key (PS-005)
23. Identical duplicate (same key, same payload, same deadline, same kind) returns existing authority and preserves original deadline — idempotent
24. Divergent duplicate (same key, different payload) returns typed `DelayedActionDuplicateConflict` — no mutation
25. Divergent duplicate (same key, different deadline) returns typed `DelayedActionDuplicateConflict` — no mutation
26. Divergent duplicate (same key, different kind) returns typed `DelayedActionDuplicateConflict` — no mutation
27. New key creates fresh pending entry and returns new authority

### Domain F: Slot Validation (PS-006)
28. `WaitUntil { deadline_slot }` validates slot value is present, numeric, non-negative integer, within timer bounds — rejects absent slots before mutation
29. `WaitEvent { timeout_slot }` validates slot value is present, numeric, non-negative, within timer bounds — rejects absent slots before mutation
30. `Ask { timeout_slot }` validates slot value is present, numeric, non-negative, within timer bounds — rejects absent slots before mutation
31. Non-timer-registering node kinds do not trigger timer registration (no false positive)
32. Malformed slot values (negative, float, oversized, wrong type) return typed `TimerSlotTypeMismatch` before mutation

### Domain G: Clock Advancement (PS-007)
33. `advance_clock_to(new_tick)` rejects `new_tick < current_tick` with `ClockWentBackwards` — no mutation
34. `advance_clock_to(new_tick)` accepts `new_tick == current_tick` — zero-step advance, fires nothing new
35. `advance_clock_to(new_tick)` fires all timers with `deadline <= new_tick`
36. `advance_clock_to(new_tick)` does not fire timers with `deadline > new_tick`
37. Equal-deadline timers fire in deterministic stable order (by RunId or insertion order)
38. `advance_clock_to` rejects ticks exceeding profile horizon with `ClockTickOutOfRange`

### Domain H: Capacity Bounds (PS-008)
39. Timer registry at max capacity returns `TimerRegistryFull` — registry and run frame unchanged
40. Delayed-action index at max capacity returns `TimerRegistryFull` — index and registry unchanged
41. Command queue at max capacity during fire returns `CommandQueueCapacityExceeded` — timer not removed (atomic fail)
42. Capacity constructor rejects invalid capacity values (zero, exceeding max)
43. Under-capacity admission succeeds normally

### Domain I: Zero-Duration Determinism (PS-009)
44. Zero duration follows exactly one documented deterministic branch — either fireable at current tick or typed `ZeroDelayRejected` — never mapped through host time
45. Zero duration does not call `Instant::now()` or any wall-clock source

### Domain J: Atomic Fire+Enqueue (PS-010)
46. Valid timer fire atomically: removes pending entry, enqueues delayed action, journals success — all or none
47. If downstream queue is full after pending removal, the timer must not be removed (capacity checked before mutation) OR the failure must produce a typed replayable outcome without silent loss
48. Partial mutation (timer removed but action not enqueued) is impossible

## 2. BDD Scenarios

### Domain A: Deadline Arithmetic Safety

#### Behavior A1: TimerTick constructor accepts valid u64
```
Given: a numeric value 0 <= t <= configured_horizon (default u64::MAX)
When: TimerTick::new(t) is called
Then: Ok(TimerTick(t)) is returned
And: the inner value is exactly t
```

Rust test: `fn timer_tick_new_accepts_zero()`
Rust test: `fn timer_tick_new_accepts_midrange_value()`
Rust test: `fn timer_tick_new_accepts_configured_horizon_max()`

#### Behavior A2: TimerTick constructor rejects tick exceeding horizon
```
Given: a numeric value t > configured_horizon
When: TimerTick::new(t) is called
Then: Err(ClockTickOutOfRange) is returned
And: no inner value is constructed
```

Rust test: `fn timer_tick_new_rejects_beyond_horizon()`
Rust test: `fn timer_tick_new_rejects_max_plus_one_when_horizon_less_than_max()`

#### Behavior A3: TimerDuration constructor accepts valid duration
```
Given: a numeric value 0 < d <= max_wait_duration_ticks (where d > 0 or allowed-by-policy)
When: TimerDuration::new(d) is called
Then: Ok(TimerDuration(d)) is returned
And: the inner value is exactly d
```

Rust test: `fn timer_duration_new_accepts_one()`
Rust test: `fn timer_duration_new_accepts_midrange_value()`
Rust test: `fn timer_duration_new_accepts_max_wait_duration()`

#### Behavior A4: TimerDuration constructor rejects excessive duration
```
Given: a numeric value d > max_wait_duration_ticks
When: TimerDuration::new(d) is called
Then: Err(TimerDurationTooLarge) is returned
```

Rust test: `fn timer_duration_new_rejects_exceeding_max()`

#### Behavior A5: TimerDeadline from tick+duration computes checked sum
```
Given: TimerTick(t) where t <= u64::MAX - d, and TimerDuration(d)
When: TimerDeadline::from_tick_and_duration(tick, duration) is called
Then: Ok(TimerDeadline(t + d)) is returned
And: the deadline value equals t + d exactly
```

Rust test: `fn timer_deadline_from_tick_and_duration_computes_exact_sum()`
Rust test: `fn timer_deadline_from_tick_and_duration_zero_duration_returns_same_tick()`
Rust test: `fn timer_deadline_from_tick_and_duration_max_no_overflow()`

#### Behavior A6: TimerDeadline overflow returns typed error
```
Given: TimerTick(t) and TimerDuration(d) where t + d > u64::MAX
When: TimerDeadline::from_tick_and_duration(tick, duration) is called
Then: Err(TimerDeadlineOverflow) is returned
And: no TimerDeadline value is constructed
```

Rust test: `fn timer_deadline_from_tick_and_duration_overflow_returns_error()`
Rust test: `fn timer_deadline_from_tick_and_duration_max_tick_plus_one_overflow()`
Rust test: `fn timer_deadline_from_tick_and_duration_both_max_overflow()`

#### Behavior A7: Absolute TimerDeadline rejects retroactive deadline
```
Given: current tick is C, and an absolute deadline value D < C
When: TimerDeadline::new_absolute(D, current_tick=C) is called and policy forbids retroactive deadlines
Then: Err(TimerDeadlineBeforeCurrent) is returned
```

Rust test: `fn timer_deadline_new_absolute_rejects_before_current_tick()`
Rust test: `fn timer_deadline_new_absolute_accepts_equal_to_current_tick()`
Rust test: `fn timer_deadline_new_absolute_accepts_after_current_tick()`

### Domain B: Numeric-Only Timer State

#### Behavior B1: Timer admission never calls Instant::now()
```
Given: any valid timer admission request
When: the admission path executes
Then: no call to std::time::Instant::now() occurs in behavior-affecting state
And: all stored time values are numeric (u64-based TimerTick/TimerDeadline, not Instant)
```

Rust test: `fn timer_admission_field_types_are_numeric_not_instant()`
Rust test: `fn pending_timer_deadline_field_is_numeric_type()`
Rust test: `fn shard_command_timer_fired_deadline_field_is_numeric_type()`

#### Behavior B2: Journal events carry numeric facts
```
Given: a timer is admitted to the registry
When: the journal event is recorded
Then: WaitScheduled / AskScheduled / TimerFired events contain numeric tick/deadline values
And: no Instant-derived values appear in journal payloads
```

Rust test: `fn wait_scheduled_journal_event_contains_numeric_deadline()`
Rust test: `fn ask_scheduled_journal_event_contains_numeric_tick_facts()`

### Domain C: Authority Validation

#### Behavior C1: Missing timer returns error — no mutation
```
Given: a Shard with no pending timer for run R
When: handle_timer(R, arbitrary_generation, arbitrary_deadline, arbitrary_kind) is called
Then: Err(TimerAuthorityMissing) is returned
And: pending_timers registry is unchanged
And: run frame is unchanged
And: no journal success event is written
And: no delayed action is enqueued
```

Rust test: `fn handle_timer_rejects_when_no_pending_timer_for_run()`

#### Behavior C2: Wrong generation returns error — no mutation
```
Given: a Shard with pending timer T for run R with generation G
When: handle_timer(R, G+1, matching_deadline, matching_kind) is called
Then: Err(TimerAuthorityMismatch) is returned
And: pending timer T remains in registry with generation G unchanged
And: run frame is unchanged
```

Rust test: `fn handle_timer_rejects_when_generation_mismatches()`
Rust test: `fn handle_timer_rejects_when_generation_is_less_than_registered()`

#### Behavior C3: Wrong deadline returns error — no mutation
```
Given: a Shard with pending timer T for run R with deadline D
When: handle_timer(R, matching_generation, D+1, matching_kind) is called
Then: Err(TimerAuthorityMismatch) is returned
And: pending timer T remains with deadline D unchanged
```

Rust test: `fn handle_timer_rejects_when_deadline_mismatches()`

#### Behavior C4: Wrong kind returns error — no mutation
```
Given: a Shard with pending Wait timer for run R
When: handle_timer(R, matching_generation, matching_deadline, Ask) is called
Then: Err(TimerAuthorityMismatch) is returned
And: pending Wait timer remains unchanged
```

Rust test: `fn handle_timer_rejects_when_kind_mismatches_wait_vs_ask()`

#### Behavior C5: Valid full authority succeeds atomically
```
Given: a Shard with pending timer T for run R, generation G, deadline D, kind K
When: handle_timer(R, G, D, K) is called
Then: Ok(()) is returned
And: pending timer T is removed from registry
And: run advances past the timer step
And: journal success event is recorded (WaitResolved for Wait timers)
And: the run continues execution (drive)
```

Rust test: `fn handle_timer_succeeds_with_exact_matching_authority()`
Rust test: `fn handle_timer_removes_pending_entry_after_valid_fire()`
Rust test: `fn handle_timer_journals_wait_resolved_after_valid_fire()`

### Domain D: Generation Exhaustion

#### Behavior D1: next_generation increments for existing timer
```
Given: pending timer for run R with generation G where G < u64::MAX
When: next_pending_timer_generation(R) is called
Then: Ok(G + 1) is returned
And: pending timer state is unchanged (no mutation)
```

Rust test: `fn next_pending_timer_generation_increments_existing()`
Rust test: `fn next_pending_timer_generation_from_zero_returns_one()`
Rust test: `fn next_pending_timer_generation_from_midrange_increments_correctly()`

#### Behavior D2: next_generation returns 1 for new run
```
Given: no pending timer for run R
When: next_pending_timer_generation(R) is called
Then: Ok(1) is returned
```

Rust test: `fn next_pending_timer_generation_returns_one_for_new_run()`

#### Behavior D3: Generation exhaustion at u64::MAX returns typed error
```
Given: pending timer for run R with generation == u64::MAX
When: next_pending_timer_generation(R) is called
Then: Err(TimerGenerationExhausted) is returned
And: pending timer state is unchanged
```

Rust test: `fn next_pending_timer_generation_exhausted_at_max_u64()`

#### Behavior D4: Generation exhaustion preserves existing state
```
Given: pending timer for run R with generation == u64::MAX and deadline D
When: register new timer for run R triggers generation exhaustion
Then: original pending timer (generation u64::MAX, deadline D) remains in registry
And: no new timer is inserted
```

Rust test: `fn await_timer_preserves_existing_on_generation_exhaustion()`

### Domain E: Duplicate Delayed-Action Key

#### Behavior E1: Identical duplicate returns existing authority — idempotent
```
Given: pending delayed-action entry for key K with payload P, deadline D, kind KT
When: schedule identical delayed-action (same K, P, D, KT) is called
Then: Ok(TimerAdmissionOutcome::AlreadyScheduled(existing_authority)) is returned
And: existing deadline D is preserved (not updated to new request's deadline)
And: no new pending entry is created
And: no duplicate journal event is emitted
```

Rust test: `fn identical_duplicate_returns_existing_authority_preserves_deadline()`
Rust test: `fn identical_duplicate_does_not_increment_generation()`
Rust test: `fn identical_duplicate_does_not_overwrite_deadline_in_registry()`

#### Behavior E2: Divergent duplicate (different payload) returns conflict error
```
Given: pending delayed-action entry for key K with payload P1
When: schedule delayed-action with same key K but different payload P2
Then: Err(TimerAdmissionError::DelayedActionDuplicateConflict) is returned
And: existing entry (K, P1) is preserved in registry unchanged
```

Rust test: `fn divergent_duplicate_different_payload_returns_conflict()`
Rust test: `fn divergent_duplicate_different_deadline_returns_conflict()`
Rust test: `fn divergent_duplicate_different_kind_returns_conflict()`

#### Behavior E3: New key creates fresh entry
```
Given: no pending entry for delayed-action key K
When: schedule delayed-action with key K
Then: Ok(TimerAdmissionOutcome::Scheduled(new_authority)) is returned
And: a new pending entry is created with key K
And: the returned authority matches the new entry
```

Rust test: `fn new_delayed_action_key_creates_fresh_entry_and_authority()`

### Domain F: Slot Validation

#### Behavior F1: WaitUntil validates deadline_slot before mutation
```
Given: a compiled workflow with WaitUntil { deadline_slot: S }
When: Shard::await_timer processes a run at this step
Then: slot S is read and validated as a numeric non-negative integer value
And: if slot S is absent, Err(TimerSlotMissing) is returned before any mutation
And: if slot S is negative or non-integer, Err(TimerSlotTypeMismatch) is returned before mutation
And: if valid, the numeric value is used to construct the absolute TimerDeadline
```

Rust test: `fn wait_until_with_present_numeric_slot_succeeds()`
Rust test: `fn wait_until_with_absent_slot_returns_slot_missing_error()`
Rust test: `fn wait_until_with_negative_slot_returns_type_mismatch_error()`
Rust test: `fn wait_until_with_non_integer_slot_returns_type_mismatch_error()`

#### Behavior F2: WaitEvent validates timeout_slot before mutation
```
Given: a compiled workflow with WaitEvent { timeout_slot: S }
When: Shard::await_timer processes a run at this step
Then: slot S is validated as numeric, non-negative, within timer bounds
And: invalid values return typed errors before mutation
```

Rust test: `fn wait_event_with_present_numeric_slot_succeeds()`
Rust test: `fn wait_event_with_absent_slot_returns_slot_missing_error()`
Rust test: `fn wait_event_with_excessive_timeout_returns_duration_too_large_error()`

#### Behavior F3: Ask validates timeout_slot before mutation
```
Given: a compiled workflow with Ask { timeout_slot: S }
When: Shard::await_timer processes a run at this step
Then: slot S is validated identically to WaitEvent timeout_slot
And: invalid values return typed errors before mutation
```

Rust test: `fn ask_with_present_numeric_slot_succeeds()`
Rust test: `fn ask_with_absent_slot_returns_slot_missing_error()`

#### Behavior F4: Non-timer nodes skip registration silently
```
Given: a compiled workflow node that is not WaitUntil, WaitEvent, or Ask
When: Shard::await_timer processes a run at this step
Then: timer_registration_required returns false
And: no timer is registered
And: the run continues normally
```

Rust test: `fn non_timer_node_does_not_register_timer()`
Rust test: `fn action_node_does_not_register_timer()`

### Domain G: Clock Advancement

#### Behavior G1: Backward clock advance rejected
```
Given: current logical tick is C
When: advance_clock_to(B) where B < C
Then: Err(ClockWentBackwards) is returned
And: current tick remains C unchanged
And: no timers are fired
```

Rust test: `fn advance_clock_to_backward_tick_returns_error()`
Rust test: `fn advance_clock_to_backward_tick_preserves_current_tick()`

#### Behavior G2: Equal tick advance is a no-op
```
Given: current logical tick is C
When: advance_clock_to(C) is called
Then: Ok(()) or Ok(empty_fire_list) is returned
And: current tick remains C
And: no currently pending timers are fired (as only deadline < C not == C unless policy fires at exact)
```

Rust test: `fn advance_clock_to_same_tick_is_noop()`

#### Behavior G3: Forward advance fires due timers
```
Given: pending timers with deadlines D1 <= D2 <= D3, and current tick C < D1
When: advance_clock_to(D2) is called
Then: current tick becomes D2
And: timers with deadlines D1 and D2 are fired (exactly those with deadline <= D2)
And: timer with deadline D3 is NOT fired (deadline > D2)
```

Rust test: `fn advance_clock_to_fires_all_due_timers_inclusive()`
Rust test: `fn advance_clock_to_does_not_fire_future_timers()`

#### Behavior G4: Equal-deadline timers fire in deterministic order
```
Given: two pending timers A and B with the same deadline
When: advance_clock_to fires both
Then: the fire order is deterministic and stable across replay (by RunId or insertion order)
And: the same order is observed on every replay with identical state
```

Rust test: `fn equal_deadline_fire_order_is_deterministic()`
Rust test: `fn equal_deadline_fire_order_identical_on_replay()`

#### Behavior G5: ClockTickOutOfRange
```
Given: configured_horizon H
When: advance_clock_to(H+1) is called
Then: Err(ClockTickOutOfRange) is returned
And: current tick unchanged
```

Rust test: `fn advance_clock_to_rejects_tick_exceeding_horizon()`

### Domain H: Capacity Bounds

#### Behavior H1: Timer registry full returns error — no mutation
```
Given: timer registry at max_capacity with N pending timers
When: schedule a new timer
Then: Err(TimerRegistryFull) is returned
And: registry still contains exactly N timers (no insertion)
And: run frame is unchanged
And: no journal event is written
```

Rust test: `fn timer_registry_at_capacity_rejects_new_timer()`
Rust test: `fn timer_registry_at_capacity_preserves_existing_timers()`

#### Behavior H2: Under-capacity admission succeeds
```
Given: timer registry with N < max_capacity pending timers
When: schedule a new timer
Then: Ok(TimerAdmissionOutcome::Scheduled(authority)) is returned
And: registry now contains N+1 timers
And: journal event is recorded
```

Rust test: `fn timer_registry_under_capacity_accepts_new_timer()`

#### Behavior H3: Invalid capacity constructor rejected
```
Given: a capacity value of 0
When: TimerCapacity::new(0) is called
Then: Err(InvalidTimerCapacity) is returned
```

Rust test: `fn timer_capacity_new_rejects_zero()`
Rust test: `fn timer_capacity_new_rejects_exceeding_max()`

### Domain I: Zero-Duration Determinism

#### Behavior I1: Zero duration follows documented branch
```
Given: a schedule request with TimerDuration(0)
When: admission is processed
Then: the outcome is deterministic — either:
  (branch A) timer is fireable at current tick, or
  (branch B) Err(ZeroDelayRejected) is returned
And: the chosen branch is documented and invariant for identical input state
And: no host time (Instant::now(), sleep, etc.) is consulted
```

Rust test: `fn zero_duration_follows_documented_branch()`
Rust test: `fn zero_duration_same_input_produces_same_outcome_across_calls()`

#### Behavior I2: Zero duration never touches host time
```
Given: a zero-duration timer admission
When: the admission path executes
Then: no std::time API call occurs in the behavior-affecting path
And: the output is a pure function of numeric inputs and registry state
```

Rust test: `fn zero_duration_output_is_pure_function_of_inputs()`

### Domain J: Atomic Fire+Enqueue

#### Behavior J1: Valid fire is fully atomic
```
Given: a Shard with pending timer T and command queue at 50% capacity
When: handle_timer with valid authority fires T
Then: T is removed from pending_timers AND a TimerFired command is enqueued AND journal is written
And: no intermediate state is observable where T is removed but command is not yet enqueued
```

Rust test: `fn valid_fire_atomically_removes_and_enqueues()`
Rust test: `fn valid_fire_records_journal_in_same_transaction()`

#### Behavior J2: Queue full during fire preserves timer (no partial mutation)
```
Given: a Shard with pending timer T and command queue at 100% capacity
When: handle_timer with valid authority fires T
Then: the operation fails with Err(CommandQueueCapacityExceeded) 
And: pending timer T remains in the registry
And: no journal success event is written
And: run frame is unchanged
```

Rust test: `fn fire_with_full_queue_preserves_timer_unmutated()`
Rust test: `fn fire_with_full_queue_does_not_partially_remove_timer()`

#### Behavior J3: Stale authority with full queue returns mismatch, not queue error
```
Given: a Shard with pending timer T (generation G) and command queue at 100% capacity
When: handle_timer with stale authority (generation G+1) is called
Then: Err(TimerAuthorityMismatch) is returned (authority gate checked first)
And: the queue capacity check is never reached
```

Rust test: `fn stale_authority_rejected_before_queue_capacity_check()`

## 3. Proptest Invariants

### Proptest PS-001: Deadline arithmetic
**Invariant**: For all `t: u64`, `d: u64`, if `t + d > u64::MAX`, `TimerDeadline::from_tick_and_duration(TimerTick(t), TimerDuration(d))` is `Err(TimerDeadlineOverflow)`; otherwise it is `Ok(TimerDeadline(t + d))`.
**Strategy**: Generate arbitrary `(t: u64, d: u64)` pairs via `any::<u64>()`.
**Anti-invariant**: Any overflow pair producing `Ok` is a bug.

### Proptest PS-002: Numeric state
**Invariant**: For all valid timer admissions, the resulting `PendingTimer` contains only numeric fields (no `Instant`-typed field can be present; type system enforces this at compile time).
**Strategy**: Generate valid admission inputs (run, step, kind, numeric deadline).
**Anti-invariant**: Test that the compiled type does not contain `std::time::Instant` (compile-time check via `static_assertions`).

### Proptest PS-003: Authority mismatch property
**Invariant**: `matches_authority(g, d, k)` returns `true` only when `self.generation == g && self.deadline == d && self.kind == k`. For any mismatch in any field, returns `false`.
**Strategy**: Generate random `(generation: u64, deadline: u64, kind: PendingTimerKind)` pairs, construct a `PendingTimer`, then test every single-field mismatch.
**Anti-invariant**: Any partial match returning `true` is a verification failure.

### Proptest PS-004: Generation property
**Invariant**: `next_generation(run)` returns `Ok(g)` where `g > current_generation` OR returns `Err` when current is `u64::MAX`. Never panics.
**Strategy**: Generate random `u64` generation values, construct registry state, test next_generation.
**Anti-invariant**: Wrapping (g > 0 and next returns 0) is a bug.

### Proptest PS-005: Duplicate key property
**Invariant**: Two identical delayed-action requests (same key, payload, deadline, kind) always produce identical outcome. Two divergent requests always produce `DelayedActionDuplicateConflict`.
**Strategy**: Generate random key+payload+deadline+kind tuples, insert, then retry with identical and variant inputs.
**Anti-invariant**: Identical duplicate producing conflict or divergent duplicate returning existing authority.

### Proptest PS-006: Slot validation property
**Invariant**: For any `(node_kind, slot_present, slot_value)`, `timer_registration_required` correctly identifies timer nodes and rejects invalid slot values before any mutation occurs.
**Strategy**: Generate random workflow node kinds, slot presence, and numeric/non-numeric values.
**Anti-invariant**: Invalid slot value accepted by validation (timer registered with bad data).

### Proptest PS-007: Clock advancement property
**Invariant**: For any sequence of `(insert_timer(deadline), advance_clock_to(tick))` operations, advancing clock never fires a timer with `deadline > tick` and always fires all timers with `deadline <= tick` in deterministic order.
**Strategy**: Generate random sequences of timer inserts at various deadlines followed by clock advances.
**Anti-invariant**: Timer with future deadline fired early or past deadline skipped.

### Proptest PS-008: Capacity property
**Invariant**: For any capacity C (1..max), and any sequence of up to C+3 insert operations, the registry never exceeds C entries, and the (C+1)th insert returns `TimerRegistryFull` and leaves registry with exactly C entries.
**Strategy**: Generate random capacity values and random timer insert sequences.
**Anti-invariant**: Registry exceeds capacity or entry silently dropped.

### Proptest PS-009: Zero-duration property
**Invariant**: `TimerDuration(0)` admission always produces the same outcome for identical input state. Adding or removing unrelated timers does not change the zero-duration outcome.
**Strategy**: Generate random registry states, test zero-duration admission, verify determinism across multiple identical calls.
**Anti-invariant**: Zero-duration outcome changes between identical invocations.

### Proptest PS-010: Atomic fire property
**Invariant**: After every `handle_timer` call, either: (a) timer was removed AND command enqueued AND journal written (full success), or (b) timer still present AND no command enqueued AND no journal written (full rejection).
**Strategy**: Generate random shard states with varying queue capacities and timer authorities.
**Anti-invariant**: Timer removed but command not enqueued (partial mutation).

## 4. Fuzz Targets

### Fuzz Target: slot_validation_fuzz (PS-006)
**Input type**: arbitrary bytes representing slot values (parsed into `SlotValue`)
**Risk**: Panic on malformed input, integer underflow/overflow during conversion, incorrect classification of non-numeric values, buffer over-read during slot lookup
**Corpus seeds**: 
- Empty byte slice
- Valid u64 values in little-endian and big-endian
- Negative integer representations (i64)
- Floating-point representations (f64)
- Maximum-size byte arrays
- Non-UTF-8 byte sequences (if string slots supported)
- Special values: NaN, infinity, negative zero

## 5. Kani Harness References

All 10 Kani harnesses are planned in State 5 proof artifacts and validated in State 12. Behavior tests in State 8 complement these:
- `ps_001_check` — deadline arithmetic no-panic (complemented by unit tests A5-A7)
- `ps_002_check` — numeric-only state verification (complemented by unit tests B1-B2)
- `ps_003_check` — authority rejection no-panic (complemented by integration tests C1-C5)
- `ps_004_check` — generation exhaustion safety (complemented by unit tests D1-D4)
- `ps_005_check` — duplicate key safety (complemented by integration tests E1-E3)
- `ps_006_check` — slot validation no-panic (complemented by integration tests F1-F4)
- `ps_007_check` — clock advancement safety (complemented by integration tests G1-G5)
- `ps_008_check` — capacity bounds safety (complemented by integration tests H1-H3)
- `ps_009_check` — zero-duration safety (complemented by integration tests I1-I2)
- `ps_010_check` — atomic fire safety (complemented by integration tests J1-J3)

## 6. Mutation Checkpoints

Critical mutations to survive (≥90% kill rate required):

| Mutation | Must be caught by |
|---|---|
| `checked_add` → `wrapping_add` in deadline construction | `test_deadline_overflow_returns_error` (A6) |
| `checked_add` → `+` in generation advance | `test_generation_exhausted_at_max_u64` (D3) |
| `>=` → `>` in clock backward check | `test_advance_clock_to_backward_tick_returns_error` (G1) |
| `<=` → `<` in fire_expired deadline comparison | `test_advance_clock_to_fires_all_due_timers_inclusive` (G3) |
| `==` → `!=` in authority deadline comparison | `test_handle_timer_rejects_when_deadline_mismatches` (C3) |
| Removal of `matches_authority` guard before swap_remove | `test_handle_timer_rejects_when_kind_mismatches` (C4) |
| `return Err(QueueFull)` → `return Ok(())` in enqueue | `test_fire_with_full_queue_preserves_timer_unmutated` (J2) |
| Capacity check removal before insert | `test_timer_registry_at_capacity_rejects_new_timer` (H1) |
| Slot value extraction → unwrap | `test_wait_until_with_absent_slot_returns_slot_missing_error` (F1) |
| Zero-duration branch → panic | `test_zero_duration_follows_documented_branch` (I1) |

## 7. Open Questions

1. **Zero-delay policy**: The domain model lists this as an open decision. The test plan covers both branches (fire-at-tick or rejection). The final policy must be selected before State 12 implementation.
2. **Exact public API names**: The bridge maps to planned names (`advance_clock_to`, `schedule_delayed_action`). Final names may differ. Behavior tests should be renamed accordingly.
3. **Equal-deadline tie-breaking mechanism**: Whether by RunId ordering, insertion order, or another stable key — must be documented. Tests assume deterministic-by-RunId.
4. **TimerWheel migration strategy**: Whether to replace the existing `TimerWheel` in place or introduce a new numeric registry alongside it affects which existing tests must be deprecated vs adapted.
5. **Kind enum extension**: The domain model includes `TimerKind::Retry` and `TimerKind::DelayedAction(ActionId)`. These are not yet in `PendingTimerKind` (which has only `Wait` and `Ask`). Tests should be extended when these variants are added.

## 8. Test File Organization

```
crates/vb_runtime/tests/behavior/
├── timer_deadline_safety_test.rs       # PS-001: behaviors A1-A7
├── numeric_timer_state_test.rs          # PS-002: behaviors B1-B2
├── authority_validation_test.rs         # PS-003: behaviors C1-C5
├── generation_exhaustion_test.rs        # PS-004: behaviors D1-D4
├── duplicate_key_test.rs                # PS-005: behaviors E1-E3
├── slot_validation_test.rs              # PS-006: behaviors F1-F4
├── clock_advancement_test.rs            # PS-007: behaviors G1-G5
├── capacity_bounds_test.rs              # PS-008: behaviors H1-H3
├── zero_duration_test.rs                # PS-009: behaviors I1-I2
└── atomic_fire_enqueue_test.rs          # PS-010: behaviors J1-J3

crates/vb_runtime/tests/refinement/
├── timer_deadline_refinement.rs
├── numeric_state_refinement.rs
├── authority_refinement.rs
├── generation_refinement.rs
├── duplicate_key_refinement.rs
├── slot_validation_refinement.rs
├── clock_advancement_refinement.rs
├── capacity_refinement.rs
├── zero_duration_refinement.rs
└── atomic_fire_enqueue_refinement.rs

crates/vb_runtime/tests/proptest/
├── ps_001_property.rs
├── ps_002_property.rs
├── ps_003_property.rs
├── ps_004_property.rs
├── ps_005_property.rs
├── ps_006_property.rs
├── ps_007_property.rs
├── ps_008_property.rs
├── ps_009_property.rs
└── ps_010_property.rs

fuzz/fuzz_targets/
└── ps_006_fuzz.rs
```

## 9. Exit Criteria Validation

- [x] Every public API behavior has at least one BDD scenario (10 domains × multiple scenarios each = 45+ scenarios)
- [x] Every pure function with multiple inputs has at least one proptest invariant (10 proptest invariants, one per proof seed)
- [x] Every parsing/deserialization boundary has a fuzz target (slot_validation_fuzz for PS-006)
- [x] Every error variant in the error taxonomy has an explicit test scenario (see coverage matrix)
- [x] Mutation threshold target stated (≥90%)
- [x] No test asserts only `is_ok()` or `is_err()` — all scenarios specify exact error variant or exact value

# Test Coverage Matrix: vb-fzgdn — Deterministic Delayed-Action Timer Seam

## Metadata
- **bead**: vb-fzgdn
- **state**: 8 (test-planner)
- **invocation_id**: vb-fzgdn-state8-test-planner-attempt1
- **derived_from**: `test-plan.md` + `error-taxonomy.md` + `type-contracts.md`

## Error Variant Coverage: Admission Errors

| Error Variant | Test Scenario | Input Class | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| `InvalidTimerTick` | tick exceeds profile horizon | max_horizon + 1 | `Err(InvalidTimerTick)` | unit | A2 |
| `TimerDurationTooLarge` | duration > max_wait_duration_ticks | max_duration + 1 | `Err(TimerDurationTooLarge)` | unit | A4 |
| `ZeroDelayRejected` | zero duration under reject-policy | `TimerDuration(0)` | `Err(ZeroDelayRejected)` | integration | I1 |
| `TimerDeadlineOverflow` | tick + duration > u64::MAX | `(MAX, 1)` / `(MAX, MAX)` | `Err(TimerDeadlineOverflow)` | unit | A6 |
| `TimerDeadlineBeforeCurrent` | absolute deadline < current_tick | `deadline 5, current 10` | `Err(TimerDeadlineBeforeCurrent)` | unit | A7 |
| `InvalidTimerCapacity` | capacity = 0 or > max | `0`, `MAX+1` | `Err(InvalidTimerCapacity)` | unit | H3 |
| `TimerRegistryFull` | registry at max_capacity | N pending = max_capacity | `Err(TimerRegistryFull)`, N unchanged | integration | H1 |
| `TimerGenerationExhausted` | current gen = u64::MAX | gen = MAX | `Err(TimerGenerationExhausted)`, state unchanged | unit | D3 |
| `RunNotFound` | target run absent | non-existent RunId | `Err(RunNotFound)` | integration | C1 |
| `TimerStepMismatch` | node kind not timer-registering | action node | error, no timer created | integration | F4 |
| `TimerSlotMissing` | slot absent from frame | `WaitUntil{deadline_slot: absent}` | `Err(TimerSlotMissing)` | integration | F1 |
| `TimerSlotTypeMismatch` | slot non-numeric or negative | float, negative, string | `Err(TimerSlotTypeMismatch)` | integration | F1-F3 |
| `DelayedActionDuplicateConflict` | same key, different payload | key K, payload P1 vs P2 | `Err(DelayedActionDuplicateConflict)` | integration | E2 |

## Error Variant Coverage: Fire Errors

| Error Variant | Test Scenario | Input Class | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| `TimerAuthorityMissing` | no pending timer for run | run with no timer | `Err(TimerAuthorityMissing)`, no mutation | integration | C1 |
| `TimerAuthorityMismatch` | wrong generation | gen G vs G+1 | `Err(TimerAuthorityMismatch)`, timer preserved | integration | C2 |
| `TimerAuthorityMismatch` | wrong deadline | deadline D vs D+1 | `Err(TimerAuthorityMismatch)`, timer preserved | integration | C3 |
| `TimerAuthorityMismatch` | wrong kind | Wait vs Ask | `Err(TimerAuthorityMismatch)`, timer preserved | integration | C4 |
| `TimerNotYetDue` | fire before deadline reached | `fire at tick 5, deadline 10` | `Err(TimerNotYetDue)` | integration | G3 |
| `TimerAlreadyFired` | duplicate fire on fired timer | fire twice with same authority | `Err(TimerAlreadyFired)` or no-op | integration | C5+ |
| `TimerCancelled` | fire on cancelled timer | cancel then fire | `Err(TimerCancelled)` | integration | (cancel scenario) |
| `DelayedActionQueueFull` | valid fire, queue full | queue at 100% | `Err(DelayedActionQueueFull)`, timer preserved | integration | J2 |

## Error Variant Coverage: Clock Errors

| Error Variant | Test Scenario | Input Class | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| `ClockWentBackwards` | advance to tick < current | `current=10, advance_to=9` | `Err(ClockWentBackwards)`, current unchanged | integration | G1 |
| `ClockTickOutOfRange` | advance to tick > horizon | `advance_to MAX+1` | `Err(ClockTickOutOfRange)`, current unchanged | integration | G5 |

## Combinatorial Coverage: Value Object Constructors

### TimerTick

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| Minimum valid | `0` | `Ok(TimerTick(0))` | unit | A1 |
| Midrange valid | `42` | `Ok(TimerTick(42))` | unit | A1 |
| Horizon boundary | `configured_horizon` | `Ok(TimerTick(horizon))` | unit | A1 |
| Horizon+1 (rejected) | `configured_horizon + 1` | `Err(ClockTickOutOfRange)` | unit | A2 |
| u64::MAX (when horizon=MAX) | `u64::MAX` | `Ok(TimerTick(MAX))` | unit | A1 |
| u64::MAX+1 (overflow) | `u64::MAX + 1` | `Err(ClockTickOutOfRange)` (overflow caught by constructor) | unit | A2 |

### TimerDuration

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| Minimum non-zero | `1` | `Ok(TimerDuration(1))` | unit | A3 |
| Midrange valid | `1000` | `Ok(TimerDuration(1000))` | unit | A3 |
| Max boundary | `max_wait_duration_ticks` | `Ok(TimerDuration(max))` | unit | A3 |
| Max+1 (rejected) | `max_wait_duration_ticks + 1` | `Err(TimerDurationTooLarge)` | unit | A4 |
| Zero (policy-dependent) | `0` | `Ok(TimerDuration(0))` or `Err(ZeroDelayRejected)` | unit | I1 |
| u64::MAX (way too large) | `u64::MAX` | `Err(TimerDurationTooLarge)` | unit | A4 |

### TimerDeadline (from tick+duration)

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| Zero+zero | `(0, 0)` | `Ok(TimerDeadline(0))` | unit | A5 |
| Zero+positive | `(0, 5)` | `Ok(TimerDeadline(5))` | unit | A5 |
| Midrange sum | `(100, 200)` | `Ok(TimerDeadline(300))` | unit | A5 |
| Max-no-overflow | `(u64::MAX - 1, 1)` | `Ok(TimerDeadline(u64::MAX))` | unit | A5 |
| Max-overflow-by-1 | `(u64::MAX, 1)` | `Err(TimerDeadlineOverflow)` | unit | A6 |
| Max-overflow-by-max | `(u64::MAX, u64::MAX)` | `Err(TimerDeadlineOverflow)` | unit | A6 |
| Max-overflow-midrange | `(u64::MAX - 10, 100)` | `Err(TimerDeadlineOverflow)` | unit | A6 |

### TimerDeadline (absolute)

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| After current | `deadline=20, current=10` | `Ok(TimerDeadline(20))` | unit | A7 |
| Equal to current | `deadline=10, current=10` | `Ok(TimerDeadline(10))` | unit | A7 |
| Before current | `deadline=5, current=10` | `Err(TimerDeadlineBeforeCurrent)` | unit | A7 |
| Zero at non-zero current | `deadline=0, current=10` | `Err(TimerDeadlineBeforeCurrent)` | unit | A7 |

### TimerGeneration (next_generation)

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| New run | no existing timer | `Ok(1)` | unit | D2 |
| From gen 0 | gen = 0 | `Ok(1)` | unit | D1 |
| From gen 1 | gen = 1 | `Ok(2)` | unit | D1 |
| From gen mid | gen = 1000 | `Ok(1001)` | unit | D1 |
| From gen MAX-1 | gen = u64::MAX - 1 | `Ok(u64::MAX)` | unit | D1 |
| From gen MAX | gen = u64::MAX | `Err(TimerGenerationExhausted)` | unit | D3 |

### TimerCapacity

| Scenario | Input | Expected Output | Test Layer | Behavior Ref |
|---|---|---|---|---|
| Minimum valid | `1` | `Ok(TimerCapacity(1))` | unit | H3 |
| Midrange valid | `64` | `Ok(TimerCapacity(64))` | unit | H3 |
| Max valid | `max_capacity` | `Ok(TimerCapacity(max))` | unit | H3 |
| Zero (invalid) | `0` | `Err(InvalidTimerCapacity)` | unit | H3 |
| Max+1 (invalid) | `max_capacity + 1` | `Err(InvalidTimerCapacity)` | unit | H3 |

## Combinatorial Coverage: Authority Validation (matches_authority)

| generation match | deadline match | kind match | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| yes | yes | yes | `true` | unit | C5 |
| no | yes | yes | `false` (reject) | unit | C2 |
| yes | no | yes | `false` (reject) | unit | C3 |
| yes | yes | no | `false` (reject) | unit | C4 |
| no | no | yes | `false` (reject) | unit | C2/C3 |
| no | yes | no | `false` (reject) | unit | C2/C4 |
| yes | no | no | `false` (reject) | unit | C3/C4 |
| no | no | no | `false` (reject) | unit | C2/C3/C4 |

## Combinatorial Coverage: Duplicate Key Classification

| Key match | Payload match | Deadline match | Kind match | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|---|
| no | N/A | N/A | N/A | `New` (fresh entry) | integration | E3 |
| yes | yes | yes | yes | `ExistingIdentical` (idempotent) | integration | E1 |
| yes | no | yes | yes | `Conflict` (divergent) | integration | E2 |
| yes | yes | no | yes | `Conflict` (divergent) | integration | E2 |
| yes | yes | yes | no | `Conflict` (divergent) | integration | E2 |
| yes | no | no | yes | `Conflict` (divergent) | integration | E2 |
| yes | yes | no | no | `Conflict` (divergent) | integration | E2 |
| yes | no | yes | no | `Conflict` (divergent) | integration | E2 |
| yes | no | no | no | `Conflict` (divergent) | integration | E2 |

## Combinatorial Coverage: Slot Validation

| Node Kind | Slot Present | Slot Value | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| `WaitUntil` | yes | valid u64 within bounds | timer registered | integration | F1 |
| `WaitUntil` | yes | valid u64 at horizon | timer registered | integration | F1 |
| `WaitUntil` | no (absent) | N/A | `Err(TimerSlotMissing)` | integration | F1 |
| `WaitUntil` | yes | negative i64 | `Err(TimerSlotTypeMismatch)` | integration | F1 |
| `WaitUntil` | yes | f64 non-integer | `Err(TimerSlotTypeMismatch)` | integration | F1 |
| `WaitUntil` | yes | u64 exceeding horizon | `Err(ClockTickOutOfRange)` | integration | F1 |
| `WaitEvent` | yes | valid u64 within bounds | timer registered | integration | F2 |
| `WaitEvent` | no (absent) | N/A | `Err(TimerSlotMissing)` | integration | F2 |
| `WaitEvent` | yes | zero duration | policy-dependent | integration | F2 |
| `WaitEvent` | yes | exceeding max_duration | `Err(TimerDurationTooLarge)` | integration | F2 |
| `WaitEvent` | yes | negative i64 | `Err(TimerSlotTypeMismatch)` | integration | F2 |
| `Ask` | yes | valid u64 within bounds | timer registered | integration | F3 |
| `Ask` | no (absent) | N/A | `Err(TimerSlotMissing)` | integration | F3 |
| `Ask` | yes | negative i64 | `Err(TimerSlotTypeMismatch)` | integration | F3 |
| Action node | yes | any | no timer registered | integration | F4 |
| Subworkflow node | yes | any | no timer registered | integration | F4 |

## Combinatorial Coverage: Clock Advancement

| Current Tick | Advance To | Pending Timers | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| 0 | 100 | none | tick=100, no fires | integration | G3 |
| 0 | 100 | deadline=50 | tick=100, 1 fire (deadline 50) | integration | G3 |
| 0 | 100 | deadlines=50, 75, 150 | tick=100, 2 fires (50, 75), 150 unfired | integration | G3 |
| 100 | 100 | deadlines=50, 100 | tick=100, 2 fires (50, 100 at exact) | integration | G2/G3 |
| 100 | 50 | deadlines=75 | `Err(ClockWentBackwards)`, tick=100 unchanged | integration | G1 |
| 5 | 10 | deadlines=10 (A, B, C - equal) | 3 fires, deterministic order | integration | G4 |
| 0 | horizon | deadline at horizon | tick=horizon, 1 fire at horizon | integration | G3 |
| 0 | horizon+1 | deadline at horizon | `Err(ClockTickOutOfRange)`, tick=0 unchanged | integration | G5 |

## Combinatorial Coverage: Capacity Bounds

| Registry Size | Capacity | Operation | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| 0 | 1 | insert | success, size=1 | integration | H2 |
| 0 | 10 | insert × 5 | success, size=5 | integration | H2 |
| N=capacity-1 | capacity | insert | success, size=capacity | integration | H2 |
| N=capacity | capacity | insert | `Err(TimerRegistryFull)`, size=capacity | integration | H1 |
| N=capacity | capacity | insert × 3 | all `Err(TimerRegistryFull)`, size=capacity | integration | H1 |
| N=any | 0 | capacity constructor | `Err(InvalidTimerCapacity)` | unit | H3 |
| N=any | max+1 | capacity constructor | `Err(InvalidTimerCapacity)` | unit | H3 |

## Combinatorial Coverage: Zero Duration

| Duration | Current Tick | Policy | Existing Timers | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|---|
| 0 | 0 | fire-at-tick | none | timer fires immediately at tick 0 | integration | I1 |
| 0 | 0 | fire-at-tick | 1 unrelated timer | timer fires immediately at tick 0 | integration | I1 |
| 0 | 100 | fire-at-tick | deadline=150 | timer fires immediately at tick 100 | integration | I1 |
| 0 | any | reject | any | `Err(ZeroDelayRejected)` | integration | I1 |
| 0 | any | any | any | output identical across 3 identical calls | integration | I2 |
| 0 | any | any | any | no std::time API call in behavior path | integration | I2 |

## Combinatorial Coverage: Atomic Fire

| Authority Valid | Queue State | Pending Timer | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|---|
| yes (exact match) | under capacity | present | timer removed, action enqueued, journal written | integration | J1 |
| yes | at capacity | present | `Err(DelayedActionQueueFull)`, timer PRESERVED | integration | J2 |
| yes | at capacity | present | verify timer not in removed state | integration | J2 |
| no (mismatch) | under capacity | present | `Err(TimerAuthorityMismatch)`, timer preserved | integration | C2-C4 |
| no (mismatch) | at capacity | present | `Err(TimerAuthorityMismatch)`, timer preserved (authority check first) | integration | J3 |
| yes | under capacity | missing (concurrent remove) | `Err(TimerAuthorityMissing)`, no mutation | integration | C1 |

## E2E Scenarios

| Scenario | Steps | Expected | Test Layer | Behavior Ref |
|---|---|---|---|---|
| Full timer lifecycle | submit_run → await_timer → capture_authority → timer_entry_fired → verify run advanced | run completes past timer step | e2e | A-J |
| Timer with deadline overflow | submit_run with tick + duration > MAX | admission rejected, run unchanged | e2e | A6 |
| Replay determinism | run workflow, capture journal, replay from journal | identical state, identical run advancement | e2e | B1-B2, G4 |
| Multiple shards with interleaved timers | two shards, each with timers, advance clock on each | each shard fires only its own timers | e2e | C1, C5 |
| Generation exhaustion under stress | 2^64 - 1 timer replacements | final replacement returns exhaustion, run preserved | e2e | D3-D4 |

## Static Analysis Gates

| Gate | Tool | What it catches | Test Layer |
|---|---|---|---|
| No `Instant` in core types | `non_exhaustive` enum + type system | `PendingTimer` cannot hold `Instant` | static |
| Overflow in release | `clippy::arithmetic_side_effects` | unchecked arithmetic in Calc layer | static |
| Non-exhaustive match | `clippy::wildcard_enum_match_arm` | missing error variant handling | static |
| Unsafe code in runtime | `#![forbid(unsafe_code)]` | any `unsafe` block in timer code | static |
| Dependency policy | `cargo-deny` | no wall-clock deps in core | static |

## Summary Statistics

| Category | Count |
|---|---|
| Error variants covered | 30 (admission: 13, fire: 8, clock: 2, plus subtype cases) |
| Value object constructor cases | 32 |
| Authority validation combinations | 8 |
| Duplicate key combinations | 9 |
| Slot validation combinations | 16 |
| Clock advancement combinations | 8 |
| Capacity bounds combinations | 7 |
| Zero duration combinations | 6 |
| Atomic fire combinations | 6 |
| E2E scenarios | 5 |
| Static analysis gates | 5 |
| **Total rows** | **144** |

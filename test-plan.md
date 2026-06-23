# Test Plan: vb-b8i8f Cancel/Kill Lattice Recovery

## Summary
- Bead: `vb-b8i8f`
- State: 8 (test-planner)
- Source checkout (control plane): `/home/lewis/src/velvet-ballistics`
- Isolated workspace: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f`
- Bridge review: APPROVED (State 7, seq 12)
- RRO rows: 22 total (5 genuine evidence, 7 deferred to State 11, 3 blocked, 2 pending exec, 5 Flux/Kani REJECTED)
- Behaviors identified: 46
- Trophy allocation: 22 unit / 18 integration / 3 e2e / 3 static
- Proptest invariants: 12
- Fuzz targets: 2
- Kani harnesses: 8 new (additional to existing wired 12 in kani_record_kind.rs)
- Mutation threshold target: >=90%

---

## 1. Behavior Inventory

### C1: Public Kill API
| B01 | `Runtime::kill_run` enqueues `ShardCommand::Kill` when run routes to existing shard |
| B02 | `Runtime::kill_run` returns `Err(ShardNotFound)` when run routes to non-existent shard |
| B03 | `Runtime::kill_run` returns `Err(QueueFull)` when shard command queue is full |
| B04 | `Runtime::kill_run` returns `Err(RunNotFound)` when run does not exist on target shard (via `handle_kill` returning typed error) |
| B05 | `Runtime::kill_run` returns `Err(RunAlreadyTerminal)` or typed error when run is already terminal |
| B06 | `ShardCommand::Kill` variant is dispatched to `handle_kill` in tick processing |

### C2: Cancel/Kill Missing and Already-Terminal Semantics
| B07 | `handle_cancel` returns `Err(RunNotFound)` when run is not in live `runs` |
| B08 | `handle_cancel` returns `Err(RunAlreadyTerminal)` when run is in `terminal_runs` |
| B09 | `handle_kill` returns `Err(RunNotFound)` when run is not in live `runs` |
| B10 | `handle_kill` returns `Err(RunAlreadyTerminal)` when run is in `terminal_runs` |
| B11 | Cancel on missing run does NOT append `RunCancelled` journal event |
| B12 | Cancel on terminal run does NOT append a second `RunCancelled` journal event |
| B13 | Kill on missing run does NOT append `RunKilled` journal event |
| B14 | Kill on terminal run does NOT append a second `RunKilled` journal event |
| B15 | Cancel on missing run does NOT increment `counters.inc_failed()` |
| B16 | Cancel on terminal run does NOT increment `counters.inc_failed()` a second time |
| B17 | Kill on missing run does NOT increment `counters.inc_failed()` |
| B18 | Kill on terminal run does NOT increment `counters.inc_failed()` a second time |
| B19 | Cancel on missing run does NOT mutate `terminal_runs` |
| B20 | Kill on missing run does NOT mutate `terminal_runs` |
| B21 | Cancel on missing run does NOT push `TraceEvent::RunCancelled` |
| B22 | Kill on missing run does NOT push `TraceEvent::RunKilled` |

### C3: Single Terminal Journal Event
| B23 | Cancel on live run appends exactly one `RunCancelled` journal event |
| B24 | Kill on live run appends exactly one `RunKilled` journal event |
| B25 | Cancel-then-cancel on same run: second cancel is rejected (C2 B08) |
| B26 | Kill-then-kill on same run: second kill is rejected (C2 B10) |
| B27 | Cancel-then-kill on same run: kill after cancel is rejected |
| B28 | Kill-then-cancel on same run: cancel after kill is rejected |
| B29 | Cancel-then-finish: finish after cancel does not produce `RunFinished` (stale authority) |
| B30 | Kill-then-finish: finish after kill does not produce `RunFinished` (stale authority) |

### C4: Stale Action/Timer Cleanup
| B31 | Successful cancel removes `pending_timers[run]` if present |
| B32 | Successful kill removes `pending_timers[run]` if present |
| B33 | Action completion after cancel returns typed error (`RunNotFound` or stale authority) |
| B34 | Action failure after cancel returns typed error |
| B35 | Action completion after kill returns typed error |
| B36 | Action failure after kill returns typed error |
| B37 | Ask answer after cancel returns typed error |
| B38 | Ask answer after kill returns typed error |
| B39 | Timer fire after cancel returns `InvalidTimerFire` or `RunNotFound` |
| B40 | Timer fire after kill returns `InvalidTimerFire` or `RunNotFound` |
| B41 | Stale action does not mutate live frame, journal, counters, or trace |

### C5: Durable Kill Storage Admission
| B42 | `RecordKind::RunKilled.id()` returns `28` (const assertion) |
| B43 | `is_known_record_kind(28)` returns `true` |
| B44 | `validate_kind_family(MAGIC_JOURNAL_EVENT, 28)` returns `Ok(())` |
| B45 | `validate_kind_family(MAGIC_SNAPSHOT, 28)` returns `Err(RecordKindFamilyMismatch)` |
| B46 | `validate_kind_family(MAGIC_BLOB, 28)` returns `Err(RecordKindFamilyMismatch)` |
| B47 | `encode_record(MAGIC_JOURNAL_EVENT, RunKilled, ...)` produces valid bytes |
| B48 | `decode_record::<JournalEvent>(bytes, MAGIC_JOURNAL_EVENT, ...)` round-trips `RunKilled` |
| B49 | `decode_journal_event(bytes, MAGIC_JOURNAL_EVENT, ...)` validates and returns `RunKilled` |
| B50 | `validate_known_kind(28)` returns `Ok(())` |
| B51 | `unknown_record_kind_value(28)` returns `None` |
| B52 | Unknown kind (e.g., 31, 0xFFFF) still rejected by `is_known_record_kind` |
| B53 | `validate_kind_family(MAGIC_JOURNAL_EVENT, 31)` returns `Err` (out of journal range) |

### C6: Replay Integrity
| B54 | `events_for_run` returns `Vec<JournalEvent>` with contiguous `EventSeq` |
| B55 | RunKilled events preserve their `EventSeq` through replay |
| B56 | `validate_replayed_event(run, expected_seq, &RunKilled)` returns `Ok(())` when seq matches |
| B57 | `validate_replayed_event(run, other_seq, &RunKilled)` returns `Err(SequenceGap)` when seq mismatches |
| B58 | `validate_replayed_event(other_run, seq, &RunKilled)` returns `Err(WrongRun)` when run mismatches |
| B59 | Killed terminal events replay as terminal (do not permit side-effect re-execution) |
| B60 | Kind 28/29 admission does not weaken rejection for unknown kind 31 |
| B61 | `next_seq(EventSeq(u64::MAX))` returns `Err(SequenceOverflow)` (existing invariant, must survive) |

---

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|-------|-------|-----------|-----------|
| **Unit / Calc** | 22 | B42-B53, B54-B61 (storage codec), B07-B10 (error type generation functions) | Pure functions with no I/O — `is_known_record_kind`, `validate_kind_family`, `encode_record`, `decode_record`, `decode_journal_event`, `next_seq`, `validate_replayed_event`. Exhaustive combinatorial coverage with boundary values. |
| **Integration** | 18 | B01-B06, B11-B41 (cancel/kill lifecycle), B54-B56 (replay with real journal) | Component interactions using REAL dependencies — `Runtime`, `Shard`, `VolatileRuntimeJournal`. No mocks. Use `Runtime::new_with_journal` with `ShardConfig` and `VolatileRuntimeJournal` for actual state verification. |
| **E2E** | 3 | B23-B24 (full cancel/kill lifecycle with journal+counters), B49 (end-to-end encode-decode through codec pipeline) | Full workflow: submit → tick → cancel/kill → verify journal + counters + trace + terminal state. |
| **Static Analysis** | 3 | B42 (const assertion), compile-time type safety for ShardCommand::Kill dispatch, `#[non_exhaustive]` on RuntimeJournalEvent | Clippy zero-tolerance, cargo-deny, `RecordKind` exhaustive match compilation, `#[must_use]` on `RecordKind::id()`. |

**Target ratios: ~48% unit, ~39% integration, ~7% e2e, ~7% static**

Deviation justification: The cancel/kill lattice has more storage-codec pure functions (unit layer) than average runtime lifecycle features, and the e2e layer is thin because the `Runtime` public API surface for kill is just one new function. This ratio is appropriate for a storage-admission + lifecycle contract correction.

---

## 3. BDD Scenarios

### C1: Public Kill API

#### Behavior: B01 — Runtime::kill_run enqueues ShardCommand::Kill when run routes to existing shard

```
fn kill_run_enqueues_shard_command_when_run_routes_to_shard():
    Given: a Runtime with 1 shard, and a run submitted to that shard
    When: runtime.kill_run(run) is called
    Then: the call returns Ok(())
    And: after tick processing, the journal contains a RunKilled event for that run
    And: the run's terminal state is Killed
```

#### Behavior: B02 — Runtime::kill_run returns Err(ShardNotFound) when run routes to non-existent shard

```
fn kill_run_returns_shard_not_found_when_shard_index_invalid():
    Given: a Runtime with 1 shard, no run submitted, and a RunId that hashes to an out-of-range shard index
    When: runtime.kill_run(run) is called
    Then: returns Err(RuntimeError::ShardNotFound { shard: _ })
    And: no journal events are appended
```

*Note: Requires testing with RunId values that produce shard indices beyond shard_count. The shard-routing function should be verified for index mapping.*

#### Behavior: B03 — Runtime::kill_run returns Err(QueueFull) when shard command queue is full

```
fn kill_run_returns_queue_full_when_command_queue_exhausted():
    Given: a Runtime with 1 shard, command queue capacity = 1, and the queue is already full
    When: runtime.kill_run(run) is called
    Then: returns Err(RuntimeError::QueueFull)
```

*Note: This may require a shard with a deliberately full queue.*

#### Behavior: B04 — Runtime::kill_run returns typed error when run does not exist on target shard

```
fn kill_run_returns_run_not_found_when_run_never_submitted():
    Given: a Runtime with 1 shard, no run submitted
    When: runtime.kill_run(RunId::new(99999)) is called
    Then: after tick processing, the command is processed but handle_kill returns Err(RunNotFound)
    And: the public API result may be Ok (enqueue succeeds) — the error surfaces through counters/journal
```

#### Behavior: B05 — Runtime::kill_run returns typed error when run is already terminal

```
fn kill_run_rejects_already_terminal_run():
    Given: a Runtime with a run that has completed (Finished terminal state)
    When: runtime.kill_run(run) is called and processed
    Then: handle_kill returns Err(RunAlreadyTerminal) (or typed equivalent)
    And: no RunKilled journal event is appended
    And: counters.inc_failed() is NOT incremented
    And: terminal_runs is NOT mutated
```

---

### C2: Cancel/Kill Missing and Already-Terminal Semantics

**CRITICAL: Current production code in `handle_cancel` and `handle_kill` always returns `Ok(())`. Tests B07-B22 must FAIL initially (TDD red) and then PASS after State 10 implementation fixes the error semantics.**

#### Behavior: B07 — handle_cancel returns Err when run is not in live runs

```
fn handle_cancel_returns_run_not_found_when_run_never_submitted():
    Given: a Shard with empty runs map
    When: handle_cancel(RunId::new(1), None) is called
    Then: returns Err(RuntimeError::RunNotFound)
    And: pending_timers is NOT mutated (swap_remove on absent key is a no-op but should be guarded)
    And: no journal event is appended
    And: counters.inc_failed() is NOT called
    And: terminal_runs is NOT mutated
```

#### Behavior: B08 — handle_cancel returns Err when run is already terminal

```
fn handle_cancel_returns_already_terminal_when_run_already_cancelled():
    Given: a Shard where run 1 has been cancelled and is in terminal_runs
    When: handle_cancel(RunId::new(1), None) is called a second time
    Then: returns Err(RuntimeError::RunNotFound) or typed RunAlreadyTerminal error
    And: no journal event is appended
    And: counters.inc_failed() is NOT called
```

#### Behaviors B09-B10: Kill equivalents of B07-B08

```
fn handle_kill_returns_run_not_found_when_run_never_submitted():
    Given: a Shard with empty runs map
    When: handle_kill(RunId::new(1), None) is called
    Then: returns Err(RuntimeError::RunNotFound)
    And: no journal event is appended
    And: counters.inc_failed() is NOT called
    And: terminal_runs is NOT mutated

fn handle_kill_returns_already_terminal_when_run_already_killed():
    Given: a Shard where run 1 has been killed and is in terminal_runs
    When: handle_kill(RunId::new(1), None) is called a second time
    Then: returns Err(RuntimeError::RunNotFound) or typed RunAlreadyTerminal error
    And: no journal event is appended
    And: counters.inc_failed() is NOT called
```

#### Behaviors B11-B22: Side-effect-free rejection

```
fn cancel_missing_run_does_not_append_journal_event():
    Given: a Shard with empty runs map
    When: handle_cancel(RunId::new(1), None) is called
    Then: no RunCancelled journal event is appended

fn cancel_terminal_run_does_not_append_second_journal_event():
    Given: a Shard where run 1 is in terminal_runs (cancelled)
    When: handle_cancel(RunId::new(1), None) is called a second time
    Then: no new RunCancelled journal event is appended (event count unchanged)

fn kill_missing_run_does_not_append_journal_event():
    Given: a Shard with empty runs map
    When: handle_kill(RunId::new(1), None) is called
    Then: no RunKilled journal event is appended

fn kill_terminal_run_does_not_append_second_journal_event():
    Given: a Shard where run 1 is in terminal_runs (killed)
    When: handle_kill(RunId::new(1), None) is called a second time
    Then: no new RunKilled journal event is appended

fn cancel_missing_run_does_not_increment_failed_counter():
fn cancel_terminal_run_does_not_increment_failed_counter_twice():
fn kill_missing_run_does_not_increment_failed_counter():
fn kill_terminal_run_does_not_increment_failed_counter_twice():
    -- (assert counters.inc_failed() not called via snapshot comparison)

fn cancel_missing_run_does_not_mutate_terminal_runs():
fn kill_missing_run_does_not_mutate_terminal_runs():
    -- (assert terminal_runs unchanged before vs after)

fn cancel_missing_run_does_not_push_trace_event():
    Given: a Shard, trace_ring has N events
    When: handle_cancel(RunId::new(missing), None) is called
    Then: trace_ring still has N events (no RunCancelled pushed)

fn kill_missing_run_does_not_push_trace_event():
    Given: a Shard, trace_ring has N events
    When: handle_kill(RunId::new(missing), None) is called
    Then: trace_ring still has N events (no RunKilled pushed)
```

---

### C3: Single Terminal Journal Event

#### Behaviors B23-B24: Successful terminalization

```
fn cancel_live_run_appends_exactly_one_runcancelled_event():
    Given: a Runtime with a live run (submitted + ticked to Running state)
    When: runtime.cancel_run(run) is called and ticks processed
    Then: journal contains exactly one RunCancelled event for that run
    And: terminal_runs contains the run
    And: counters.runs_failed == 1

fn kill_live_run_appends_exactly_one_runkilled_event():
    Given: a Runtime with a live run
    When: runtime.kill_run(run) is called and ticks processed
    Then: journal contains exactly one RunKilled event for that run
    And: terminal_runs contains the run
    And: counters.runs_failed == 1
```

#### Behaviors B25-B28: Mutual exclusion of terminal events

```
fn second_cancel_after_first_cancel_is_rejected():
    Given: a run that has been cancelled (terminal_runs contains it)
    When: runtime.cancel_run(run) is called again and ticks processed
    Then: cancel returns typed error (not Ok(()))
    And: journal RunCancelled count remains 1

fn second_kill_after_first_kill_is_rejected():
    Given: a run that has been killed (terminal_runs contains it)
    When: runtime.kill_run(run) is called again and ticks processed
    Then: kill returns typed error
    And: journal RunKilled count remains 1

fn kill_after_cancel_is_rejected():
    Given: a run that has been cancelled
    When: runtime.kill_run(run) is called and ticks processed
    Then: returns typed error
    And: journal contains RunCancelled but NOT RunKilled

fn cancel_after_kill_is_rejected():
    Given: a run that has been killed
    When: runtime.cancel_run(run) is called and ticks processed
    Then: returns typed error
    And: journal contains RunKilled but NOT RunCancelled
```

---

### C4: Stale Action/Timer Cleanup

```
fn cancel_removes_pending_timer():
    Given: a runtime with a run suspended on a Wait or Ask timer
    When: runtime.cancel_run(run) is called and ticks processed
    Then: pending_timers no longer contains the run
    And: the pending timer is removed before the journal event is appended

fn kill_removes_pending_timer():
    Given: a runtime with a run suspended on a Wait or Ask timer
    When: runtime.kill_run(run) is called and ticks processed
    Then: pending_timers no longer contains the run
    And: the pending timer is removed before the journal event is appended

fn action_completion_after_cancel_returns_error():
    Given: a run that submitted an action-suspended workflow, then cancelled
    When: runtime.complete_action_with_output(ticket, output) is called
    Then: returns Err (typed error, not Ok(()))
    And: journal does NOT contain ActionCompletedEvent for the stale action
    And: the run's frame is NOT mutated

fn action_failure_after_cancel_returns_error():
    Given: a run that submitted an action, then cancelled
    When: runtime.fail_action(ticket, failure) is called
    Then: returns Err (typed error)
    And: journal does NOT contain ActionFailedEvent

fn action_completion_after_kill_returns_error():
fn action_failure_after_kill_returns_error():
    -- (kill equivalents of above)

fn ask_answer_after_cancel_returns_error():
    Given: a run suspended on an Ask, then cancelled
    When: an AskAnswer is dispatched
    Then: returns Err (RunNotFound or InvalidActionCompletion)
    And: no SlotWritten journal event is appended

fn ask_answer_after_kill_returns_error():
    -- (kill equivalent)

fn timer_fire_after_cancel_returns_error():
    Given: a run cancelled, leaving pending_timers empty
    When: handle_timer(run, generation, deadline, kind) is called
    Then: returns Err(InvalidTimerFire)
    And: no journal event is appended
    And: counters are NOT mutated

fn timer_fire_after_kill_returns_error():
    -- (kill equivalent of timer_fire_after_cancel)

fn stale_action_does_not_mutate_state():
    Given: a run that was cancelled, snapshot of frame, counters, journal, terminal_runs
    When: a stale action completion is attempted (returns Err)
    Then: frame, counters, journal, terminal_runs, trace_ring are identical to pre-attempt snapshot
```

---

### C5: Durable Kill Storage Admission

```
fn record_kind_run_killed_id_is_28():
    -- const assertion: assert_eq!(RecordKind::RunKilled.id(), 28)

fn is_known_record_kind_28_returns_true():
    -- unit: assert!(is_known_record_kind(28))

fn validate_kind_family_journal_event_28_returns_ok():
    -- unit: assert_eq!(validate_kind_family(MAGIC_JOURNAL_EVENT, 28), Ok(()))

fn validate_kind_family_snapshot_28_returns_rejection():
    -- unit: assert!(matches!(validate_kind_family(MAGIC_SNAPSHOT, 28), Err(RecordKindFamilyMismatch{..})))

fn validate_kind_family_blob_28_returns_rejection():
    -- unit: assert!(matches!(validate_kind_family(MAGIC_BLOB, 28), Err(RecordKindFamilyMismatch{..})))

fn encode_record_runkilled_produces_valid_bytes():
    Given: a valid JournalEvent::RunKilled { run: non-zero, seq: valid, attempt: >0 }
    When: encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunKilled, seq_n, &event, max_payload_len) is called
    Then: returns Ok(Vec<u8>) with non-empty bytes

fn decode_record_runkilled_roundtrip():
    Given: a RunKilled event, encoded bytes from encode_record
    When: decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, max_payload_len) is called
    Then: returns Ok((envelope, event)) where event == original RunKilled event

fn decode_journal_event_runkilled_passes_validation():
    Given: valid RunKilled encoded bytes (run != 0, seq < MAX, attempt > 0)
    When: decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, max_payload_len) is called
    Then: returns Ok((envelope, JournalEvent::RunKilled{..}))
    And: is_valid() check passes

fn decode_journal_event_runkilled_zero_run_rejected():
    Given: encoded RunKilled with RunId(0)
    When: decode_journal_event is called
    Then: returns Err(JournalError::InvalidEvent) (is_valid() fails on zero run)

fn decode_journal_event_runkilled_zero_attempt_rejected():
    Given: encoded RunKilled with attempt=0
    When: decode_journal_event is called
    Then: returns Err(JournalError::InvalidEvent)

fn validate_known_kind_28_returns_ok():
    -- unit: assert_eq!(validate_known_kind(28), Ok(()))

fn unknown_record_kind_value_28_returns_none():
    -- unit: assert_eq!(unknown_record_kind_value(28), None)

fn is_known_record_kind_31_returns_false():
    -- unit: assert!(!is_known_record_kind(31))

fn is_known_record_kind_0xFFFF_returns_false():
    -- unit: assert!(!is_known_record_kind(0xFFFF))

fn validate_kind_family_journal_event_31_returns_rejection():
    -- unit: assert!(matches!(validate_kind_family(MAGIC_JOURNAL_EVENT, 31), Err(RecordKindFamilyMismatch{..})))
```

---

### C6: Replay Integrity

```
fn validate_replayed_event_match_returns_ok():
    Given: RunKilled event with run=RunId(10), seq=EventSeq(5)
    When: validate_replayed_event(RunId(10), EventSeq(5), &event) is called
    Then: returns Ok(())

fn validate_replayed_event_seq_mismatch_returns_gap():
    Given: RunKilled event with seq=EventSeq(5)
    When: validate_replayed_event(run, EventSeq(3), &event) is called
    Then: returns Err(JournalError::SequenceGap { expected: EventSeq(3), actual: EventSeq(5) })

fn validate_replayed_event_run_mismatch_returns_wrong_run():
    Given: RunKilled event with run=RunId(10)
    When: validate_replayed_event(RunId(20), EventSeq(0), &event) is called
    Then: returns Err(JournalError::WrongRun { expected: RunId(20), actual: RunId(10) })

fn next_seq_max_returns_overflow():
    Given: EventSeq::new(u64::MAX)
    When: next_seq(seq) is called
    Then: returns Err(JournalError::SequenceOverflow)

fn next_seq_zero_returns_one():
    Given: EventSeq::new(0)
    When: next_seq(seq) is called
    Then: returns Ok(EventSeq::new(1))

fn kind_28_and_29_admission_does_not_open_unknown_kind_31():
    Given: is_known_record_kind passes for 28 and 29, validate_kind_family(MAGIC_JOURNAL_EVENT, 28/29) returns Ok
    When: is_known_record_kind(31), validate_kind_family(MAGIC_JOURNAL_EVENT, 31) are called
    Then: is_known_record_kind(31) returns false AND validate_kind_family returns Err
    -- Regression: ensuring the 28/29 fix didn't accidentally admit wildcard kinds
```

---

## 4. Proptest Invariants

### PO-PROP-001 (RRO-004): RunKilled validation properties — PASSING via workspace proptest
- **Existing**: `prop_record_kind_28_is_valid`, `prop_runkilled_valid_event_passes_validation`, `prop_runkilled_zero_run_invalid`, `prop_runkilled_zero_attempt_invalid`, `prop_runkilled_overflow_seq_invalid`
- **Status**: 10/10 tests pass. Non-vacuous. Production-bound.
- **Invariant**: Any `JournalEvent::RunKilled` with non-zero run, non-zero attempt, and non-overflow seq passes `is_valid()`; zero-run, zero-attempt, and overflow-seq events fail validation.

### PO-PROP-002 (RRO-008): RecordKind uniqueness — PASSING
- **Existing**: `prop_record_kind_28_is_unique`, `prop_journal_kinds_in_valid_range`
- **Status**: PASSING. Non-vacuous.
- **Invariant**: All `RecordKind` variant `id()` values are unique when collected into a `BTreeSet`; `RunKilled=28` is not duplicated.

### PO-PROP-003 (RRO-012): RunKilled field consistency — PASSING
- **Existing**: `prop_runkilled_carries_attempt`, `prop_runkilled_record_kind_consistent`, `prop_runkilled_distinct_from_cancelled`
- **Status**: PASSING. Non-vacuous.
- **Invariant**: `RunKilled.attempt()` returns the given attempt; `RunKilled.record_kind()` always returns `RecordKind::RunKilled`; `RunKilled` is not equal to `RunCancelled` with same fields.

### PO-PROP-004 (RRO-016): Kind 28 round-trip — BLOCKED
- **Target**: `encode_record` then `decode_record::<JournalEvent>` for `RunKilled`
- **Invariant**: For any valid `RunKilled { run, seq, attempt }` with `run != RunId(0)`, `seq < EventSeq(u64::MAX)`, `attempt > 0`: `decode(encode(event)) == event` (round-trip equality).
- **Anti-invariant**: `RunKilled { run: RunId(0), .. }` produces `Err(InvalidEvent)` on decode.
- **Current gap**: `proptest_storage.rs:317` compile error blocks execution. State 11 fix required.

### PO-PROP-005 (RRO-021): Replay sequence properties — BLOCKED
- **Target**: `events_for_run`, `validate_replayed_event`
- **Invariant**: `events_for_run` returns contiguous `EventSeq` for all events including `RunKilled`. Gaps detected as `SequenceGap`, duplicates detected.
- **Current gap**: Same compile error.

### New Proptest Invariants (to be written in State 9)

#### PROP-006: Cancel/Kill Side-Effect Invariant
- **Invariant**: For any sequence of `cancel_run`/`kill_run` calls on the same `RunId`, the total number of terminal journal events appended is at most 1.
- **Strategy**: Arbitrary `RunId`, may or may not be submitted first. Apply sequences of Cancel/Kill commands. Assert journal terminal event count <= 1.

#### PROP-007: Kind Family Rejection Invariant
- **Invariant**: For any `(magic, kind)` pair, `validate_kind_family(magic, kind)` returns `Ok` iff `(magic, kind)` is in the accepted set, and `Err(RecordKindFamilyMismatch)` otherwise. Never panics.
- **Strategy**: `proptest::arbitrary::any::<u32>()` for magic, `any::<u16>()` for kind. Assert function is pure and total.

#### PROP-008: Codec Round-Trip Integrity for All Journal Events Including RunKilled
- **Invariant**: For any valid `JournalEvent` (including `RunKilled`), `decode_record::<JournalEvent>(encode_record(...))` returns `Ok(original_event)`.
- **Strategy**: Generate arbitrary `JournalEvent` values (including `RunKilled`). Ensure only valid events pass. Round-trip encode/decode.

#### PROP-009: is_known_record_kind Consistency
- **Invariant**: `is_known_record_kind(k)` is equivalent to `matches!(k, 1|2|3|10..=29|30|40|50)`. For any `u16` value, the result is deterministic and never panics.
- **Strategy**: `any::<u16>()` — exhaustive across the u16 space.

#### PROP-010: validate_known_kind/unknown_record_kind_value Coherence
- **Invariant**: `unknown_record_kind_value(k).is_none()` iff `is_known_record_kind(k)`. `validate_known_kind(k).is_ok()` iff `is_known_record_kind(k)`.
- **Strategy**: `any::<u16>()`.

#### PROP-011: next_seq Monotonicity
- **Invariant**: For any `seq` in `0..u64::MAX`, `next_seq(EventSeq(seq))` returns `Ok(EventSeq(seq+1))`. For `seq = u64::MAX`, returns `Err(SequenceOverflow)`.
- **Strategy**: `any::<u64>()`.

---

## 5. Fuzz Targets

### PO-FUZZ-001 (RRO-017): Kind Validation Fuzz
- **Target**: `validate_kind_family`, `is_known_record_kind`, `validate_known_kind`
- **Input type**: arbitrary `(magic: u32, kind: u16)` pairs (8 bytes)
- **Risk**: Panic on unanticipated magic values, incorrect boolean logic on boundary kind values (0, 1, 28, 29, 30, 50, 51, 0xFFFF), integer overflow in match arms.
- **Corpus seeds**:
  - `(MAGIC_JOURNAL_EVENT, 28)` — known pass
  - `(MAGIC_JOURNAL_EVENT, 27)` — boundary (just below 28)
  - `(MAGIC_JOURNAL_EVENT, 29)` — `AskTimedOut` known pass
  - `(MAGIC_JOURNAL_EVENT, 31)` — unknown kind rejection
  - `(MAGIC_SNAPSHOT, 28)` — known reject
  - `(MAGIC_BLOB, 28)` — known reject
  - `(0x00000000, 0x0000)` — zero magic, zero kind
  - `(0xFFFFFFFF, 0xFFFF)` — max values
- **Fuzz target file**: `fuzz/fuzz_targets/kind_validation.rs`

### PO-FUZZ-002 (RRO-022): Journal Decode Fuzz
- **Target**: `decode_record::<JournalEvent>`, `decode_journal_event`
- **Input type**: arbitrary byte streams (up to 4096 bytes)
- **Risk**: Postcard deserialization panic on malformed bytes; structural invariant violations in decoded `JournalEvent`; unhandled `RecordKind` discriminator values in enum deserialization; integer overflow in payload length calculations; memory exhaustion on crafted payload-length claims.
- **Corpus seeds**:
  - Valid `RunKilled` encoded record
  - Valid `RunCancelled` encoded record
  - Valid `RunFinished` encoded record
  - Zero-length input
  - Header-only (truncated payload)
  - Garbage bytes
  - Bytes with valid 60-byte header but invalid postcard
  - Bytes with valid header + valid postcard but invalid `is_valid()` check
- **Fuzz target file**: `fuzz/fuzz_targets/journal_decode.rs`

---

## 6. Kani Harnesses

### Existing Wired Harnesses (PO-KANI-004, RRO-014; PO-KANI-005, RRO-019)
- **File**: `crates/vb_storage/src/kani_record_kind.rs` (wired via `lib.rs:44`)
- **Harnesses**: `check_kind_28_known`, `check_kind_28_journal_family`, `check_kind_28_snapshot_family_rejected`, `check_kind_28_blob_family_rejected`, `check_unknown_kind_rejected`, `check_all_existing_kinds_known`, `check_journal_family_exhaustive`, `check_replay_contiguity_with_killed`, etc.
- **Status**: Non-vacuous, production-bound, GOD RULE 1 compliant. Uses `kani::any()`.
- **Re-execution needed**: Post BLOCK-001 resolution in production code (isolated workspace already has the fix).

### New Kani Harnesses (State 11)

#### KANI-006: is_known_record_kind Exhaustive across u16
- **Property**: For all `u16` values, `is_known_record_kind(k)` is equivalent to the explicit match set `{1, 2, 3, 10..=29, 30, 40, 50}`.
- **Bound**: Full u16 space (65,536 values) — Kani can handle this with unwind.
- **Rationale**: The `matches!` macro is simple but must be proven correct for all 65,536 possible `u16` values. This is the canonical admission gate for kind 28.

#### KANI-007: validate_kind_family Exactness
- **Property**: For all `(u32, u16)` pairs within bounded space, `validate_kind_family(magic, kind)` returns `Ok(())` iff the pair is in the known accepted set; returns `Err(RecordKindFamilyMismatch)` otherwise.
- **Bound**: All 6 magic constants × all u16 kind values.
- **Rationale**: Formal verification that kind 28 is ONLY admitted for `MAGIC_JOURNAL_EVENT`, not for snapshot, blob, or other families.

#### KANI-008: next_seq No Panic + Overflow Correctness
- **Property**: `next_seq(seq)` never panics for any `u64` input. Returns `Err(SequenceOverflow)` iff `seq == u64::MAX`. Returns `Ok(EventSeq(seq+1))` for all other values.
- **Bound**: u64 space (practical with 2^64 values via symbolic execution).
- **Rationale**: Overflow on sequence numbers would corrupt replay contiguity.

#### KANI-009: validate_replayed_event Correctness
- **Property**: For any `(run, other_run, seq, other_seq)` where all values are symbolic: `validate_replayed_event` returns `Ok` iff `run == event.run_id() && seq == event.seq()`. Returns `Err(WrongRun)` or `Err(SequenceGap)` otherwise. Never panics.
- **Rationale**: Replay integrity is critical for durability.

#### KANI-010: encode_record Panic-Freedom for RunKilled
- **Property**: `encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunKilled, seq, &RunKilled{..}, max_payload_len)` never panics for any valid input configuration. Returns `Ok` for valid payloads, `Err` for payload overflow.
- **Rationale**: Storage encoding must be panic-free for all valid RunKilled inputs.

#### KANI-011: decode_journal_event Panic-Freedom
- **Property**: `decode_journal_event(&bytes, any_u32_magic, any_u32_max_payload_len)` never panics for any arbitrary byte slice within a bounded length (e.g., <= 512 bytes). Returns `Ok` or `Err(JournalError::*)` variants; no panics.
- **Rationale**: Journal decode is an untrusted-input boundary. Panic = denial of service.

#### KANI-012: handle_cancel Error for Missing/Terminal Runs
- **Property**: Given a shard with a known OR absent run, `handle_cancel` returns `Err(RunNotFound)` for absent runs and `Err(RunNotFound)` / typed `Err(RunAlreadyTerminal)` for runs in `terminal_runs`. Never returns `Ok(())` for these cases.
- **Rationale**: Contract C2 requires this behavior. Current production code violates it.
- **BLOCK-002 dependency**: Full shard construction requires `SharedRuntimeJournal → FjallJournal → Keyspace` chain not symbolically executable. May need a `#[cfg(kani)]` simplified shard constructor.

#### KANI-013: handle_kill Error for Missing/Terminal Runs
- **Property**: Same as KANI-012 but for `handle_kill`.
- **Rationale**: Kill must follow the same contract as cancel for missing/terminal runs.

---

## 7. Mutation Checkpoints

### Critical Mutations to Survive

| Mutation Target | Test That Must Catch It |
|----------------|------------------------|
| `is_known_record_kind` — remove `28` from match arm | `is_known_record_kind_28_returns_true` (unit) + proptest PROP-009 |
| `validate_kind_family` — change `10..=29` to `10..=27` | `validate_kind_family_journal_event_28_returns_ok` and AskTimedOut kind-29 tests |
| `validate_kind_family` — change journal branch from `10..=29` to `10..=50` (over-admit) | `validate_kind_family_journal_event_31_returns_rejection` (unit) |
| `validate_kind_family` — change snapshot/branch to admit 28 | `validate_kind_family_snapshot_28_returns_rejection` (unit) |
| `handle_cancel` — remove `terminal_runs` guard before `append_journal_event` | `cancel_live_run_appends_exactly_one_runcancelled_event` (integration) + `second_cancel_after_first_cancel_is_rejected` |
| `handle_cancel` — remove `runs.contains_key` guard for journal append | `cancel_missing_run_does_not_append_journal_event` (integration) |
| `handle_kill` — remove `runs.swap_remove` guard for full terminalization | `handle_kill_returns_run_not_found_when_run_never_submitted` (integration) |
| `handle_cancel` — swap `discard_journal_sequence` before `append_journal_event` | `cancel_live_run_appends_exactly_one_runcancelled_event` — journal must have event, not discarded sequence |
| `handle_kill` — omit `pending_timers.swap_remove` | `kill_removes_pending_timer` (integration) |
| `RecordKind::RunKilled.id()` — change from `28` to any other value | `record_kind_run_killed_id_is_28` (unit) + proptest PROP-002 |
| `decode_journal_event` — remove `is_valid()` check | `decode_journal_event_runkilled_zero_run_rejected` (unit) — zero-run event would pass unchecked |
| `validate_replayed_event` — swap expected/actual in comparison | `validate_replayed_event_seq_mismatch_returns_gap` (unit) — error variant check |
| `unknown_record_kind_value` — invert boolean | PROP-010 (proptest) — must catch inversion |

### Threshold: >=90% mutation kill rate
- Target: `cargo mutants --package vb_storage --files "codec/validation.rs" "codec/mod.rs"`
- Target: `cargo mutants --package vb_runtime --files "shard/lifecycle/chunk_002.rs"`

---

## 8. Combinatorial Coverage Matrix

### Unit: Storage Codec (validation.rs, codec/mod.rs)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy path: kind 28 known | kind=28 | `is_known_record_kind(28) = true` | unit |
| happy path: kind 28 journal family | (MAGIC_JOURNAL_EVENT, 28) | `validate_kind_family = Ok(())` | unit |
| happy path: kind 29 known | kind=29 | `is_known_record_kind(29) = true` | unit |
| happy path: kind 29 journal family | (MAGIC_JOURNAL_EVENT, 29) | `validate_kind_family = Ok(())` | unit |
| happy path: encode RunKilled | RunKilled{valid} | `encode_record = Ok(bytes)` | unit |
| happy path: decode RunKilled | valid bytes | `decode_record = Ok(RunKilled{..})` | unit |
| happy path: decode_journal_event RunKilled | valid bytes, valid event | `decode_journal_event = Ok(RunKilled{..})` | unit |
| happy path: validate_known_kind 28 | kind=28 | `Ok(())` | unit |
| happy path: next_seq normal | EventSeq(5) | `Ok(EventSeq(6))` | unit |
| happy path: validate_replayed_event match | matching run+seq | `Ok(())` | unit |
| error: kind 28 snapshot family | (MAGIC_SNAPSHOT, 28) | `Err(RecordKindFamilyMismatch{..})` | unit |
| error: kind 28 blob family | (MAGIC_BLOB, 28) | `Err(RecordKindFamilyMismatch{..})` | unit |
| error: kind 31 unknown | kind=31 | `is_known_record_kind(31) = false` | unit |
| error: kind 31 journal family | (MAGIC_JOURNAL_EVENT, 31) | `Err(RecordKindFamilyMismatch{..})` | unit |
| error: decode RunKilled zero run | RunKilled{run=0} | `Err(InvalidEvent)` | unit |
| error: decode RunKilled zero attempt | RunKilled{attempt=0} | `Err(InvalidEvent)` | unit |
| error: decode RunKilled overflow seq | RunKilled{seq=MAX} | `Err(InvalidEvent)` | unit |
| error: next_seq overflow | EventSeq(u64::MAX) | `Err(SequenceOverflow)` | unit |
| error: wrong run | (RunId(10), event with RunId(20)) | `Err(WrongRun{..})` | unit |
| error: sequence gap | (seq=3, event with seq=5) | `Err(SequenceGap{..})` | unit |
| boundary: kind 0 | kind=0 | `is_known_record_kind(0) = false` | unit |
| boundary: kind 1 | kind=1 | `is_known_record_kind(1) = true` | unit |
| boundary: kind 3 | kind=3 | `is_known_record_kind(3) = true` | unit |
| boundary: kind 10 | kind=10 | `is_known_record_kind(10) = true` | unit |
| boundary: kind 27 | kind=27 | `is_known_record_kind(27) = true` | unit |
| boundary: kind 28 | kind=28 | `is_known_record_kind(28) = true` | unit |
| boundary: kind 29 | kind=29 | `is_known_record_kind(29) = true` | unit |
| boundary: kind 30 | kind=30 | `is_known_record_kind(30) = true` | unit |
| boundary: kind 31 | kind=31 | `is_known_record_kind(31) = false` | unit |
| boundary: kind 40 | kind=40 | `is_known_record_kind(40) = true` | unit |
| boundary: kind 50 | kind=50 | `is_known_record_kind(50) = true` | unit |
| boundary: kind 51 | kind=51 | `is_known_record_kind(51) = false` | unit |
| boundary: kind 0xFFFF | kind=65535 | `is_known_record_kind(0xFFFF) = false` | unit |
| boundary: constant assertion | `RecordKind::RunKilled.id()` | `== 28` | unit (const) |
| invariant: kind 28 unique | all RecordKind variants | `RunKilled=28` unique in set | proptest |
| invariant: round-trip | any valid JournalEvent+RunKilled | `decode(encode(e)) == e` | proptest |
| invariant: kind family exactness | all (u32, u16) | `Ok` iff in accepted set | kani |
| invariant: no panic on encode | any valid RunKilled | returns `Ok` or `Err`, no panic | kani |
| invariant: no panic on decode | arbitrary bytes (bounded) | returns `Ok` or `Err(JournalError::*)`, no panic | kani |

### Integration: Cancel/Kill Lifecycle (Runtime + Shard + Journal)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| happy: cancel live run | live run, cancel_run() | journal: `RunCancelled`; counters: `runs_failed=1`; terminal_runs: contains run | integration |
| happy: kill live run | live run, kill_run() | journal: `RunKilled`; counters: `runs_failed=1`; terminal_runs: contains run | integration |
| happy: cancel removes pending timer | run suspended on timer, cancel | pending_timers: no longer contains run | integration |
| happy: kill removes pending timer | run suspended on timer, kill | pending_timers: no longer contains run | integration |
| error: cancel missing run | Never-submitted run, cancel | `Err(RunNotFound)`; no journal event; no counter increment | integration |
| error: kill missing run | Never-submitted run, kill | `Err(RunNotFound)`; no journal event; no counter increment | integration |
| error: cancel terminal run | Cancelled run, cancel again | `Err(RunNotFound or RunAlreadyTerminal)`; journal unchanged | integration |
| error: kill terminal run | Killed run, kill again | `Err(RunNotFound or RunAlreadyTerminal)`; journal unchanged | integration |
| error: cancel after kill | Killed run, cancel | `Err`; no `RunCancelled` appended | integration |
| error: kill after cancel | Cancelled run, kill | `Err`; no `RunKilled` appended | integration |
| error: action completion after cancel | Cancelled run, complete_action | `Err`; no journal event; frame unchanged | integration |
| error: action failure after cancel | Cancelled run, fail_action | `Err`; no journal event | integration |
| error: action completion after kill | Killed run, complete_action | `Err`; no journal event | integration |
| error: action failure after kill | Killed run, fail_action | `Err`; no journal event | integration |
| error: ask answer after cancel | Cancelled run, ask_answer | `Err`; no `SlotWritten` appended | integration |
| error: ask answer after kill | Killed run, ask_answer | `Err`; no `SlotWritten` appended | integration |
| error: timer fire after cancel | Cancelled run, handle_timer | `Err(InvalidTimerFire)`; no journal; counters unchanged | integration |
| error: timer fire after kill | Killed run, handle_timer | `Err(InvalidTimerFire)`; no journal; counters unchanged | integration |
| error: kill routing fails shard | RunId hashing to out-of-range shard | `Err(ShardNotFound)` | integration |
| error: kill queue full | RunId on shard with full queue | `Err(QueueFull)` (enqueue failure) | integration |
| invariant: single terminal event | sequence of cancel/kill calls | journal terminal count <= 1 | integration + proptest |
| invariant: stale no-state-mutation | snapshot before vs after stale op | frame, counters, journal, terminal_runs, trace unchanged | integration |

---

## 9. Test File Allocation

### Files to Write (State 9)

| File | Crate | Test Type | Behaviors Covered |
|------|-------|-----------|-------------------|
| `crates/vb_storage/src/codec/tests/kill_kind_admission.rs` | vb_storage | unit | B42-B53 (storage admission) |
| `crates/vb_storage/src/codec/tests/replay_integrity.rs` | vb_storage | unit | B54-B61 (replay integrity) |
| `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` | workspace_tests | integration | B01-B06 (kill API), B11-B22 (side-effect-free), B23-B30 (mutual exclusion), B31-B41 (stale authority) — extends existing file |
| `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` | workspace_tests | proptest | PROP-001 through PROP-005 (existing), PROP-006 through PROP-011 (new) |
| `crates/vb_storage/src/proptest_storage.rs` | vb_storage | proptest | PROP-004, PROP-005 (unblocked by compile fix) |

### Files to Modify (State 9 initialization)

| File | Action |
|------|--------|
| `crates/workspace_tests/Cargo.toml` | Add `kill_lifecycle_tests` test target (or extend existing cancel_kill_lattice_tests.rs) |
| `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs` | Add kill scenarios, error-semantics tests, stale authority tests |
| `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` | Add new PROP-006 through PROP-011 |
| `crates/vb_storage/Cargo.toml` | Ensure `[[test]]` targets exist for new test files |
| `crates/vb_storage/src/proptest_storage.rs` | Fix `proptest_storage.rs:317` compile error |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | NOT a test file, but State 10 implementation changes will make tests pass |

### Existing Tests (No Changes Required)

| File | Status |
|------|--------|
| `crates/workspace_tests/tests/cancel_kill_lattice_props.rs` | 10/10 pass (State 6 evidence). Keep as-is. |
| `crates/vb_storage/src/kani_record_kind.rs` | Wired + non-vacuous. Keep. Re-run post BLOCK-001. |

---

## Open Questions

1. **Q1: Error variant for already-terminal runs.** The contract says cancel/kill on already-terminal runs returns typed error. Should this be `RuntimeError::RunNotFound` (existing) or a new `RuntimeError::RunAlreadyTerminal { run: RunId }`? The domain model says `RunNotFound` may serve both. Resolution needed before State 10 implementation. *Recommendation: Use `RunNotFound` for both missing and terminal to avoid expanding the error surface; document the conflation in error taxonomy.*

2. **Q2: handle_kill journal event.** Currently `handle_kill` does NOT call `self.append_journal_event(RuntimeJournalEvent::RunKilled { run })` — the journal append is inside the `if let Some(state) = self.runs.swap_remove(&run)` block, meaning only live runs get a journal event. But `handle_cancel` calls `append_journal_event` BEFORE the `if let Some` guard (line 121-123). Should `handle_kill` follow the same pre-guard pattern? *Recommendation: Yes, `handle_kill` should mirror `handle_cancel`'s journal-append-before-guard pattern for consistency.*

3. **Q3: Public kill API result type.** Currently `Runtime::cancel_run` returns `RuntimeResult<()>` and always returns `Ok(())` because the enqueue succeeds. The error surfaces later via shard tick processing. Should `Runtime::kill_run` follow the same pattern (enqueue only, errors surface asynchronously), or should it synchronously check run existence before enqueue? *Recommendation: Follow identical pattern to cancel_run for API consistency. The error reaches the caller through trace/journal/counter observation.*

4. **Q4: proptest_storage.rs:317 fix scope.** The pre-existing compile error at line 317 is blocking 2/22 RRO rows. Is fixing this error within State 9 scope, or only in State 11? *The bridge says State 11 for the fix, but the test plan must document the tests that would run after the fix. Recommend treating the compile fix as a State 9 prerequisite for test validation.*

5. **Q5: Shard-level vs Runtime-level test scope.** The existing `cancel_kill_lattice_tests.rs` tests at the Runtime level (using public API). Should the new error-semantics tests (B07-B22) test at the Runtime level or the Shard level? *Recommendation: Runtime-level for integration tests (public API contract), Shard-level for unit tests (internal contract). Both layers are useful — Shard-level for fast deterministic validation of handle_cancel/handle_kill, Runtime-level for end-to-end correctness.*

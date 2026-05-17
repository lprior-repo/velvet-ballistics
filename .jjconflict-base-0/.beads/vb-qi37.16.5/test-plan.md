# Test Plan: vb-qi37.16.5 — cli/runtime: Add lifecycle integration evidence

## Summary

- **Bead ID**: vb-qi37.16.5
- **Phase**: State 4 (test-planning)
- **Domain**: Lifecycle integration — cancel, resume, retry, answer commands with journal replay, invalid/duplicate/stale rejection, and structured diagnostics
- **Behaviors identified**: 23 (+ 8 storage-journal behaviors added to fix LETHAL coverage gaps)
- **Trophy allocation**: 14 integration / 45 unit / 2 e2e / 1 static
- **Proptest invariants**: 4
- **Fuzz targets**: 2
- **Kani harnesses**: 0 (waived — integration tests + TLA+ cover state space)
- **Mutation checkpoint threshold**: ≥80% (integration layer; cargo-mutants deferred to later beads per waiver)
- **Unit test ratio**: 45 unit / 8 pub fns = **5.6×** (target ≥5× ✓)
- **TLA+ Model Status**: **PRESENT** — `specs/tla/RecoveryReplay.tla` authored and covers INV-002, INV-003, INV-004, POST-003, POST-004, POST-005. `specs/LifecycleJournal.tla` (referenced in tla-spec.md) is NOT authored yet; evidence deferred to formal-verifier bead at State 12 per explicit plan in `tla-spec.md` line 37. The existing `RecoveryReplay.tla` provides partial coverage of lifecycle journal semantics.

---

## 1. Behavior Inventory

### Lifecycle Commands (Happy Path)

1. `[Runtime] cancel command succeeds when bead is in Active or WaitingAnswer state`
2. `[Runtime] cancel command succeeds when bead is in WaitingAnswer state`
3. `[Runtime] resume command succeeds when bead is in Cancelled state`
4. `[Runtime] retry command succeeds when bead is in Failed state`
5. `[Runtime] answer command succeeds when bead is in WaitingAnswer state and answer is provided`
6. `[Runtime] each successful command appends exactly one RuntimeJournalEvent to the journal`

### State Transitions

7. `[LifecycleState] valid transitions are: Pending→Active, Active→WaitingAnswer, Active→Cancelled, WaitingAnswer→Cancelled, Cancelled→Active (resume), Failed→Active (retry), WaitingAnswer→Completed (answer), Active→Completed (answer without answer-required)`
8. `[LifecycleState] cancel is invalid from Pending state`
9. `[LifecycleState] cancel is invalid from Completed state`
10. `[LifecycleState] cancel is invalid from Failed state`
11. `[LifecycleState] resume is invalid from Pending state`
12. `[LifecycleState] resume is invalid from Active state`
13. `[LifecycleState] resume is invalid from WaitingAnswer state`
14. `[LifecycleState] resume is invalid from Completed state`
15. `[LifecycleState] resume is invalid from Failed state`
16. `[LifecycleState] retry is invalid from Pending state`
17. `[LifecycleState] retry is invalid from Active state`
18. `[LifecycleState] retry is invalid from Cancelled state`
19. `[LifecycleState] retry is invalid from Completed state`
20. `[LifecycleState] answer is invalid from Pending state`
21. `[LifecycleState] answer is invalid from Active state`
22. `[LifecycleState] answer is invalid from Cancelled state`
23. `[LifecycleState] answer is invalid from Completed state`

### Duplicate & Stale Requests

24. `[Runtime] duplicate cancel request returns E_DUPLICATE_REQUEST and does not double-write journal`
25. `[Runtime] duplicate resume request returns E_DUPLICATE_REQUEST and does not double-write journal`
26. `[Runtime] duplicate retry request returns E_DUPLICATE_REQUEST and does not double-write journal`
27. `[Runtime] duplicate answer request returns E_DUPLICATE_REQUEST and does not double-write journal`
28. `[Runtime] stale cancel request (state already advanced past Active/WaitingAnswer) returns E_STALE_REQUEST`
29. `[Runtime] stale resume request (state not Cancelled) returns E_STALE_REQUEST`
30. `[Runtime] stale retry request (state not Failed) returns E_STALE_REQUEST`
31. `[Runtime] stale answer request (state not WaitingAnswer) returns E_STALE_REQUEST`

### Restart / Replay

32. `[Replay] replay from empty journal produces valid initial state with all beads Pending`
33. `[Replay] replay from clean snapshot produces valid initial state`
34. `[Replay] replay from journal with N events reconstructs bit-identical bead_state to pre-crash`
35. `[Replay] partial replay (snapshot + incremental journal) produces identical result to full replay`
36. `[Replay] replay with malformed event returns E_REPLAY_CORRUPTION`
37. `[Replay] replay with missing event returns E_REPLAY_CORRUPTION`

### Storage / I/O Errors

38. `[Runtime] journal write failure returns E_JOURNAL_WRITE_FAILURE and does not leave partial state`
39. `[Runtime] storage backend unavailable at command dispatch returns E_STORAGE_UNAVAILABLE`
40. `[Runtime] storage backend unavailable at replay returns E_STORAGE_UNAVAILABLE`

### Structured Diagnostics

41. `[Diagnostics] E_INVALID_TRANSITION includes {code, context, timestamp, bead_id, command}`
42. `[Diagnostics] E_DUPLICATE_REQUEST includes {code, context, timestamp, bead_id, command}`
43. `[Diagnostics] E_STALE_REQUEST includes {code, context, timestamp, bead_id, command}`
44. `[Diagnostics] E_JOURNAL_WRITE_FAILURE includes {code, context, timestamp, bead_id, command}`
45. `[Diagnostics] E_REPLAY_CORRUPTION includes {code, context, timestamp, bead_id, command}`
46. `[Diagnostics] E_STORAGE_UNAVAILABLE includes {code, context, timestamp, bead_id, command}`

### Preconditions

47. `[Runtime] lifecycle command returns error when storage backend is not connected (PRE-001)`
48. `[Runtime] lifecycle command is validated against current bead state before journal write (PRE-002)`
49. `[Replay] recovery replay starts from clean snapshot or empty journal state (PRE-003)`

### Storage Journal Public API (vb_storage/journal.rs)

50. `[StorageJournal] write_event appends exactly one JournalEvent to durable storage and returns Ok(())`
51. `[StorageJournal] write_event rejects a duplicate event (same run_id, same seq) with E_DUPLICATE_EVENT`
52. `[StorageJournal] write_event rejects an oversized payload exceeding MAX_JOURNAL_EVENT_PAYLOAD_BYTES`
53. `[StorageJournal] read_journal (events_for_run) returns all events for a run in strict sequence order`
54. `[StorageJournal] read_journal returns empty vec for an unknown run_id`
55. `[StorageJournal] read_journal detects and returns E_REPLAY_CORRUPTION when a sequence gap exists`
56. `[StorageJournal] read_journal isolates events: events for run A contain no events from run B`
57. `[StorageJournal] append_strict_batch writes all events atomically and returns E_JOURNAL_WRITE_FAILURE on partial failure`

### Invariants

58. `[Invariant] at any point each bead has exactly one canonical lifecycle state (INV-001)`
59. `[Invariant] journal append-only: no event is ever removed or overwritten (INV-002)`
60. `[Invariant] no lifecycle command skips a required antecedent state (INV-003)`
61. `[Invariant] restart/replay produces bit-identical bead states (INV-004)`

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Integration** | 14 | Most critical — component boundaries, real journal/storage deps, no mocks |
| **Unit / Calc** | 45 | 8 pub fns × 5x minimum + storage journal module tests covering write_event/read_journal edge cases; pure transition functions, command validation predicates, error construction |
| **E2E** | 2 | Full CLI dispatch through storage to journal; black-box lifecycle smoke |
| **Static Analysis** | 1 | `cargo clippy --all-targets` on touched crates (velvet_ballastics, vb_runtime, vb_storage) |

**Unit test breakdown (45 total)**:
- `cancel(bead_id)` from {Active, WaitingAnswer, Pending, Completed, Failed} = 5 tests
- `resume(bead_id)` from {Cancelled, Pending, Active, WaitingAnswer, Completed, Failed} = 6 tests
- `retry(bead_id)` from {Failed, Pending, Active, Cancelled, Completed, WaitingAnswer} = 6 tests
- `answer(bead_id, answer)` from {WaitingAnswer, Pending, Active, Cancelled, Completed, Failed} + answer boundaries = 8 tests
- `journal::append_event` happy + JournalWriteFailure + exactly-one-event = 3 tests
- `journal::replay` empty + clean-snapshot + full-replay + partial-replay + malformed + missing = 6 tests
- `storage::write_event` (append_journaled, append_strict, append_strict_batch, duplicate-reject, oversized-payload, E_JOURNAL_WRITE_FAILURE) = 6 tests
- `storage::read_journal` (events_for_run happy, empty-unknown-run, sequence-gap, run-isolation, many-events) = 5 tests
- Error construction (6 error variants × 1 test each) = 6 tests
- **Total = 45 unit tests / 8 pub fns = 5.6×** ✓

**Rationale**: Lifecycle correctness is fundamentally about component interaction (journal + storage + runtime + CLI). Unit tests cover pure logic in transition functions, validators, and storage journal edge cases. The 45-unit count comes from exhaustive state coverage for all 8 public functions plus dedicated storage journal API coverage. E2E covers the CLI-to-storage pipeline. Static analysis catches regressions in the touched crates.

---

## 3. BDD Scenarios

### Group A: Happy Path Lifecycle Commands

---

#### Behavior: `cancel command succeeds when bead is in Active state`

**Given**: A bead exists with `LifecycleState::Active`
**When**: `cancel(bead_id)` is called
**Then**: Command succeeds, exactly one `RuntimeJournalEvent::Cancelled` is appended to the journal, bead state transitions to `Cancelled`

```
fn cancel_succeeds_from_active_state() {
    // Setup: create bead in Active state via answerPending→Active path
    // Trigger: cancel(bead_id)
    // Assert: Ok(()), journal has 1 Cancell event, bead_state[bead_id] == Cancelled
    // Assert: journal.len() == 1 (exactly one event)
}
```

**Given**: A bead exists with `LifecycleState::WaitingAnswer`
**When**: `cancel(bead_id)` is called
**Then**: Command succeeds, exactly one `RuntimeJournalEvent::Cancelled` is appended, bead transitions to `Cancelled`

```
fn cancel_succeeds_from_waiting_answer_state() {
    // Assert: journal has exactly one event post-command
    // Assert: bead_state[bead_id] == Cancelled
}
```

---

#### Behavior: `resume command succeeds when bead is in Cancelled state`

**Given**: A bead exists with `LifecycleState::Cancelled`
**When**: `resume(bead_id)` is called
**Then**: Command succeeds, exactly one `RuntimeJournalEvent::Resumed` is appended, bead transitions to `Active`

```
fn resume_succeeds_from_cancelled_state() {
    // Setup: bead in Cancelled
    // Trigger: resume(bead_id)
    // Assert: Ok(()), journal grows by 1 Resumed event, bead_state == Active
}
```

---

#### Behavior: `retry command succeeds when bead is in Failed state`

**Given**: A bead exists with `LifecycleState::Failed`
**When**: `retry(bead_id)` is called
**Then**: Command succeeds, exactly one `RuntimeJournalEvent::Retried` is appended, bead transitions to `Active`

```
fn retry_succeeds_from_failed_state() {
    // Assert: bead_state == Active post-retry
    // Assert: journal shows exactly 1 Retried event
}
```

---

#### Behavior: `answer command succeeds when bead is in WaitingAnswer state`

**Given**: A bead exists with `LifecycleState::WaitingAnswer`
**When**: `answer(bead_id, answer_content)` is called
**Then**: Command succeeds, exactly one `RuntimeJournalEvent::Answered(answer)` is appended, bead transitions to `Completed`

```
fn answer_succeeds_from_waiting_answer_state() {
    // Assert: Ok(())
    // Assert: bead_state == Completed
    // Assert: journal[last] == RuntimeJournalEvent::Answered(answer)
}
```

---

### Group B: Invalid Transitions

---

#### Behavior: `cancel returns E_INVALID_TRANSITION from invalid prior states`

**Given**: A bead exists with `LifecycleState::Pending`
**When**: `cancel(bead_id)` is called
**Then**: Returns `E_INVALID_TRANSITION`, journal is unchanged (empty or at prior state), no event appended

```
fn cancel_returns_invalid_transition_when_bead_is_pending() {
    // Assert: Err(E_INVALID_TRANSITION { code: "E_INVALID_TRANSITION", context: _, bead_id, command: Cancel })
    // Assert: journal unchanged — verify journal.len() == prior_len
    // Assert: bead_state unchanged
}
```

**Error variant — cancel from Completed**:
```
fn cancel_returns_invalid_transition_when_bead_is_completed() {
    // Assert: Err(E_INVALID_TRANSITION)
    // Assert: journal unchanged
}
```

**Error variant — cancel from Failed**:
```
fn cancel_returns_invalid_transition_when_bead_is_failed() {
    // Assert: Err(E_INVALID_TRANSITION)
}
```

---

#### Behavior: `resume returns E_INVALID_TRANSITION from invalid prior states`

**Given**: A bead in `Pending` / `Active` / `WaitingAnswer` / `Completed` / `Failed` state
**When**: `resume(bead_id)` is called
**Then**: Returns `E_INVALID_TRANSITION`, journal unchanged

```
fn resume_returns_invalid_transition_when_bead_is_{state}() {
    // where {state} ∈ {pending, active, waiting_answer, completed, failed}
    // Assert: Err(E_INVALID_TRANSITION)
    // Assert: journal unchanged
}
```

---

#### Behavior: `retry returns E_INVALID_TRANSITION from invalid prior states`

**Given**: A bead not in `Failed` state
**When**: `retry(bead_id)` is called
**Then**: Returns `E_INVALID_TRANSITION`, journal unchanged

```
fn retry_returns_invalid_transition_when_bead_is_{state}() {
    // where {state} ∈ {pending, active, cancelled, completed, waiting_answer}
    // Assert: Err(E_INVALID_TRANSITION)
    // Assert: journal unchanged
}
```

---

#### Behavior: `answer returns E_INVALID_TRANSITION from invalid prior states`

**Given**: A bead not in `WaitingAnswer` state
**When**: `answer(bead_id, answer)` is called
**Then**: Returns `E_INVALID_TRANSITION`, journal unchanged

```
fn answer_returns_invalid_transition_when_bead_is_{state}() {
    // where {state} ∈ {pending, active, cancelled, completed, failed}
    // Assert: Err(E_INVALID_TRANSITION)
    // Assert: journal unchanged
}
```

---

### Group C: Duplicate Requests

---

#### Behavior: `duplicate cancel returns E_DUPLICATE_REQUEST and never double-writes`

**Given**: A bead is in `Active` state, `cancel(bead_id)` was already called successfully
**When**: `cancel(bead_id)` is called again (second time in same state)
**Then**: Returns `E_DUPLICATE_REQUEST`, journal was not double-written (exactly 1 Cancell event)

```
fn cancel_returns_duplicate_request_when_called_twice_in_same_state() {
    // Setup: bead in Active, cancel once → journal has 1 event
    // Trigger: cancel(bead_id) second time
    // Assert: Err(E_DUPLICATE_REQUEST)
    // Assert: journal.len() == 1 (not 2 — no double-write)
    // Assert: bead_state == Cancelled (not changed by second call)
}
```

**Duplicate resume**:
```
fn resume_returns_duplicate_request_when_called_twice() {
    // Setup: bead in Cancelled, resume once
    // Assert: journal.len() == 1
    // Assert: second resume → E_DUPLICATE_REQUEST
}
```

**Duplicate retry**:
```
fn retry_returns_duplicate_request_when_called_twice() {
    // Same pattern: second retry → E_DUPLICATE_REQUEST, journal not double-written
}
```

**Duplicate answer**:
```
fn answer_returns_duplicate_request_when_called_twice() {
    // Second answer in Completed state → E_DUPLICATE_REQUEST
}
```

---

### Group D: Stale Requests

---

#### Behavior: `stale cancel returns E_STALE_REQUEST when bead state has already advanced`

**Given**: A bead was in `Active` state when cancel was called, and state has since advanced (e.g., to `WaitingAnswer` or `Completed`)
**When**: The original `cancel(bead_id)` is re-issued with the now-stale prior-state expectation
**Then**: Returns `E_STALE_REQUEST`, state is not retroactively modified

```
fn cancel_returns_stale_request_when_state_already_advanced() {
    // Setup: bead: Pending → Active → WaitingAnswer (advance past Active)
    // Issue cancel targeting Active state
    // Assert: Err(E_STALE_REQUEST)
    // Assert: bead_state == WaitingAnswer (not reverted)
}
```

**Stale resume** (bead not in Cancelled):
```
fn resume_returns_stale_request_when_not_in_cancelled_state() {
    // bead is in Active, try to resume → E_STALE_REQUEST
    // Assert: state unchanged
}
```

**Stale retry** (bead not in Failed):
```
fn retry_returns_stale_request_when_not_in_failed_state() {
    // bead is in Cancelled, try to retry → E_STALE_REQUEST
}
```

**Stale answer** (bead not in WaitingAnswer):
```
fn answer_returns_stale_request_when_not_in_waiting_answer_state() {
    // bead is in Completed, try to answer → E_STALE_REQUEST
}
```

---

### Group E: Restart / Replay

---

#### Behavior: `replay from empty journal produces valid initial state`

**Given**: Journal is empty (no events)
**When**: `replay()` is called
**Then**: Returns `Ok(vec of initial RuntimeState)`, all beads are in `Pending` state

```
fn replay_from_empty_journal_produces_valid_initial_state() {
    // Setup: empty journal
    // Trigger: replay()
    // Assert: Ok(states) where all states have LifecycleState::Pending
    // Assert: matches Init action of TLA+ model
}
```

---

#### Behavior: `replay from clean snapshot produces valid initial state`

**Given**: A clean snapshot exists (snapshot taken at terminal state)
**When**: `replay()` is called with snapshot + empty incremental journal
**Then**: Returns state identical to snapshot

```
fn replay_from_clean_snapshot_produces_valid_initial_state() {
    // Setup: snapshot at Pending, no incremental journal
    // Trigger: replay()
    // Assert: Ok(state) == snapshot state
}
```

---

#### Behavior: `replay reconstructs bit-identical bead state to pre-crash`

**Given**: Journal contains N events from a sequence of lifecycle commands
**When**: `replay()` is called after crash (simulated by dropping in-memory state)
**Then**: Returns `Ok(states)` identical to pre-crash `bead_state` map, bit-for-bit

```
fn replay_full_journal_reconstructs_bit_identical_state() {
    // Setup: drive bead through Pending→Active→WaitingAnswer→Completed
    // Capture: journal content + bead_state snapshot
    // Simulate crash: clear in-memory state
    // Trigger: replay()
    // Assert: returned bead_state == pre-crash bead_state (bit-identical)
    // Assert: journal content unchanged
}
```

**Multi-bead replay**:
```
fn replay_full_journal_reconstructs_bit_identical_multi_bead_state() {
    // Same but with multiple beads in different states
    // Assert: each bead's state matches pre-crash exactly
}
```

---

#### Behavior: `partial replay (snapshot + incremental journal) produces identical result`

**Given**: Snapshot taken at step K, incremental journal has events K+1 … N
**When**: `replay()` is called with snapshot + incremental
**Then**: Result identical to replay from full journal

```
fn replay_partial_with_snapshot_plus_incremental_produces_identical_result() {
    // Setup: snapshot at step 3, incremental events 4-7
    // Trigger: replay()
    // Assert: state == replay(full_journal)
}
```

---

#### Behavior: `replay with malformed event returns E_REPLAY_CORRUPTION`

**Given**: Journal contains a malformed event (corrupt bytes, invalid enum discriminant, truncated event)
**When**: `replay()` is called
**Then**: Returns `Err(E_REPLAY_CORRUPTION)` with structured diagnostics including the corrupt event index or id

```
fn replay_with_malformed_event_returns_replay_corruption() {
    // Setup: journal with valid events, then corrupt last event
    // Trigger: replay()
    // Assert: Err(E_REPLAY_CORRUPTION { code, context, bead_id: None, command: None })
    // Assert: no partial state left in storage
}
```

---

#### Behavior: `replay with missing event returns E_REPLAY_CORRUPTION`

**Given**: Journal has a gap (expected event index missing, truncated file)
**When**: `replay()` is called
**Then**: Returns `Err(E_REPLAY_CORRUPTION)`

```
fn replay_with_missing_event_returns_replay_corruption() {
    // Setup: truncate journal file mid-event
    // Assert: Err(E_REPLAY_CORRUPTION)
}
```

---

### Group F: Storage / I/O Errors

---

#### Behavior: `journal write failure returns E_JOURNAL_WRITE_FAILURE`

**Given**: Storage is connected but write fails (disk full, I/O error mid-write)
**When**: A lifecycle command that would write to the journal is called
**Then**: Returns `Err(E_JOURNAL_WRITE_FAILURE)`, no partial event written, state unchanged

```
fn lifecycle_command_returns_journal_write_failure_on_io_error() {
    // Setup: simulate storage I/O failure (fault injection)
    // Trigger: cancel(bead_id) when bead in Active
    // Assert: Err(E_JOURNAL_WRITE_FAILURE)
    // Assert: no partial event visible in journal
    // Assert: bead state unchanged
}
```

---

#### Behavior: `storage unavailable returns E_STORAGE_UNAVAILABLE`

**Given**: Storage backend is not connected or connection is lost
**When**: A lifecycle command is dispatched
**Then**: Returns `Err(E_STORAGE_UNAVAILABLE)`, no state modified

```
fn lifecycle_command_returns_storage_unavailable_when_not_connected() {
    // Setup: no storage backend connected
    // Trigger: cancel(bead_id)
    // Assert: Err(E_STORAGE_UNAVAILABLE)
}
```

---

### Group H: State Transition Graph Completeness (Behavior 7)

---

#### Behavior: `all valid state transitions exist in the transition graph`

**Given**: A runtime with 6 beads, one in each of the 6 lifecycle states: Pending, Active, WaitingAnswer, Cancelled, Failed, Completed
**When**: The complete valid transition graph is enumerated by attempting every command from every state
**Then**: The complete set of valid transitions is exactly:
- `Pending→Active` (via command that activates a bead)
- `Active→WaitingAnswer` (via command that enters waiting)
- `Active→Cancelled` (via cancel from Active)
- `Active→Completed` (via answer from Active when no answer-required)
- `WaitingAnswer→Cancelled` (via cancel from WaitingAnswer)
- `WaitingAnswer→Completed` (via answer from WaitingAnswer)
- `Cancelled→Active` (via resume from Cancelled)
- `Failed→Active` (via retry from Failed)

**And**: Every other command×state pair returns `Err(E_INVALID_TRANSITION)`, confirming no undocumented edges exist.

```
fn valid_transition_graph_contains_all_expected_edges() {
    // Setup: 6 beads, one in each state
    // Action: enumerate all 6×4=24 command×state pairs
    // Assert: exactly 8 return Ok with correct next state
    // Assert: remaining 16 return Err(E_INVALID_TRANSITION)
    // Assert: each of the 8 valid edges produces the expected target state
    // Graph edges verified:
    //   Pending→Active, Active→WaitingAnswer, Active→Cancelled,
    //   Active→Completed, WaitingAnswer→Cancelled, WaitingAnswer→Completed,
    //   Cancelled→Active, Failed→Active
    assert_eq!(valid_transitions.len(), 8);
    assert!(valid_transitions.contains_key(&(Pending, Cancel))); // no-op - Pending cannot cancel
    // ... full enumeration
}

fn valid_transition_graph_excludes_all_invalid_edges() {
    // For each of the 16 invalid (command, prior_state) pairs:
    // Assert: Err(E_INVALID_TRANSITION)
    // Assert: journal unchanged (no event appended)
    // Invalid pairs include:
    //   Cancel from Pending, Cancel from Completed, Cancel from Failed
    //   Resume from Pending, Resume from Active, Resume from WaitingAnswer,
    //   Resume from Completed, Resume from Failed
    //   Retry from Pending, Retry from Active, Retry from Cancelled,
    //   Retry from Completed, Retry from WaitingAnswer
    //   Answer from Pending, Answer from Active, Answer from Cancelled,
    //   Answer from Completed, Answer from Failed
}
```

**Edge case — Pending→Active via implicit activation**:
```
fn pending_to_active_transition_exists_via_implicit_activation() {
    // Given: bead in Pending state
    // When: a command that activates the bead is issued (e.g., start bead)
    // Then: bead transitions to Active
    // This edge is required for the graph to be connected (no orphan Pending state)
}
```

**Edge case — graph connectivity**: All 6 states must be reachable from any other state via valid transitions.

```
fn state_transition_graph_is_connected() {
    // Given: the valid transition graph
    // When: compute reachability from any state to any other state
    // Then: all 6 states are mutually reachable (the graph is strongly connected)
    // Path coverage: Pending→Active→WaitingAnswer→Completed (terminal)
    //               Pending→Active→Cancelled→Active→... (loop)
    //               Pending→Active→Failed→Active→... (loop)
}
```

**Edge case — no self-loops permitted**:
```
fn no_state_has_a_self_loop_transition() {
    // For each state S and each command C:
    // Assert: transition(S, C) never returns Ok(S) — no state transitions to itself
    // Exception: none — self-loops are never valid lifecycle transitions
}
```

---

### Group I: Storage Journal API (write_event / read_journal)

---

#### Behavior: `write_event appends exactly one JournalEvent and returns Ok(())`

**Given**: A FjallJournal opened at a temp path, empty (no prior events)
**When**: `append_journaled(event)` is called with a valid `JournalEvent`
**Then**: Returns `Ok(())`, exactly one event is readable via `events_for_run(run_id)`

```
fn write_event_succeeds_and_event_is_readable() {
    // Setup: FjallJournal, run_id = RunId::new(1)
    // Trigger: journal.append_journaled(&event)
    // Assert: Ok(())
    // Assert: journal.events_for_run(run_id).len() == 1
    // Assert: journal.events_for_run(run_id)[0] == event
}
```

**Strict durability variant**:
```
fn write_event_strict_forces_durability_barrier() {
    // append_strict returns only after persist completes
    // Crash immediately after append_strict should not lose the event
}
```

---

#### Behavior: `write_event rejects duplicate event with E_DUPLICATE_EVENT`

**Given**: A journal with one event already written at run_id=1, seq=0
**When**: `append_journaled(&same_event)` is called again
**Then**: Returns `Err(JournalError::DuplicateEvent { run, seq })`, journal still contains exactly 1 event

```
fn write_event_rejects_duplicate_run_seq() {
    // Setup: one event written
    // Trigger: append same event again
    // Assert: Err(JournalError::DuplicateEvent)
    // Assert: journal.events_for_run(run_id).len() == 1
}
```

---

#### Behavior: `write_event rejects oversized payload`

**Given**: A journal
**When**: An event with payload exceeding `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` is written
**Then**: Returns `Err(JournalError::PayloadTooLarge)` or the encoding layer rejects it

```
fn write_event_rejects_oversized_payload() {
    // Build an event with an oversized field
    // Assert: Err(JournalError::PayloadTooLarge)
}
```

---

#### Behavior: `read_journal returns events in strict sequence order`

**Given**: A journal with events seq=0,1,2,3,4 written in order
**When**: `events_for_run(run_id)` is called
**Then**: Returns a vec of 5 events in ascending seq order, each seq(i) == i

```
fn read_journal_returns_events_in_sequence_order() {
    // Write 5 events
    // Trigger: events_for_run(run_id)
    // Assert: len == 5
    // Assert: events[0].seq == 0, events[4].seq == 4
}
```

---

#### Behavior: `read_journal returns empty vec for unknown run_id`

**Given**: A journal with events for run_id=1
**When**: `events_for_run(run_id=99999)` is called
**Then**: Returns `Ok(vec![])`, no error

```
fn read_journal_returns_empty_for_unknown_run() {
    // Assert: journal.events_for_run(RunId::new(99999)) == Ok(vec![])
}
```

---

#### Behavior: `read_journal detects sequence gap and returns E_REPLAY_CORRUPTION`

**Given**: A journal with seq=0 and seq=2 written (seq=1 is missing/gap)
**When**: `events_for_run(run_id)` is called
**Then**: Returns `Err(JournalError::SequenceGap { run, expected: 1, found: 2 })`

```
fn read_journal_detects_sequence_gap() {
    // Write seq 0 and seq 2 (gap at 1)
    // Assert: Err(JournalError::SequenceGap)
}
```

---

#### Behavior: `read_journal isolates events per run`

**Given**: A journal with events for run A (2 events) and run B (3 events)
**When**: `events_for_run(run_A)` is called
**Then**: Returns exactly 2 events, both with run_id == run_A (run B events are invisible)

```
fn read_journal_isolates_events_between_runs() {
    // Write 2 events for run A, 3 for run B
    // Assert: events_for_run(run_A).len() == 2
    // Assert: events_for_run(run_B).len() == 3
    // Assert: all events_for_run(run_A) have run_id == run_A
}
```

---

#### Behavior: `append_strict_batch atomically writes all events`

**Given**: A journal
**When**: `append_strict_batch(&[e0, e1, e2])` is called
**Then**: Returns `Ok(())`, all 3 events are readable, durability is guaranteed

```
fn batch_write_all_events_are_atomically_readable() {
    // Trigger: append_strict_batch(&[e0, e1, e2])
    // Assert: events_for_run(run_id).len() == 3
    // Assert: crash after append_strict_batch does not lose events
}
```

**Partial failure variant**:
```
fn batch_write_returns_error_on_partial_failure() {
    // If any event in the batch fails encoding, the whole batch fails
    // No partial state left in journal
}
```

---

### Group J: PRE-001 Integration Coverage

---

#### Behavior: `lifecycle command returns E_STORAGE_UNAVAILABLE when storage backend is not connected (PRE-001)`

**Given**: The CLI/runtime is started WITHOUT a connected storage backend (storage path does not exist, or a `NoopStorage` / `DisconnectedStorage` test adapter is injected)
**When**: Any lifecycle command (`cancel`, `resume`, `retry`, `answer`) is dispatched
**Then**: Returns `Err(E_STORAGE_UNAVAILABLE)` with structured diagnostics (code, context, timestamp, bead_id, command), no state is modified

```
fn lifecycle_command_returns_storage_unavailable_when_backend_not_connected() {
    // Setup: runtime initialized with disconnected storage adapter
    // Trigger: cancel(bead_id)
    // Assert: Err(E_STORAGE_UNAVAILABLE { code: "E_STORAGE_UNAVAILABLE", ... })
    // Assert: no journal entry written (journal is unreachable)
    // Same for resume, retry, answer — all 4 commands must fail PRE-001
}

fn lifecycle_command_storage_unavailable_includes_all_diagnostic_fields() {
    // For each command × E_STORAGE_UNAVAILABLE:
    // Assert: err.code == "E_STORAGE_UNAVAILABLE"
    // Assert: err.context is non-empty string describing unavailability
    // Assert: err.timestamp parses as ISO-8601
    // Assert: err.bead_id matches the target bead
    // Assert: err.command matches Cancel|Resume|Retry|Answer
}
```

**Note**: This replaces the prior MANUAL-QA-001 which deferred this to state 7. This is now covered at the integration layer via a `StorageFault` test trait or `NoopStorage` adapter that implements the storage interface but returns `Err(StorageError::Unavailable)` on every operation.

---

### Group K: Answer Content Boundary Cases

---

#### Behavior: `answer command accepts valid answer content at minimum, typical, and maximum sizes`

**Given**: A bead in `WaitingAnswer` state
**When**: `answer(bead_id, answer_content)` is called with valid answer content
**Then**: Returns `Ok(())`, bead transitions to `Completed`, journal has exactly one `Answered` event

**Minimum boundary**:
```
fn answer_accepts_empty_string_answer() {
    // answer_content = ""
    // Assert: Ok(())
    // Assert: bead_state == Completed
}
```

**Typical content**:
```
fn answer_accepts_normal_text_answer() {
    // answer_content = "the answer is 42"
    // Assert: Ok(())
    // Assert: bead_state == Completed
}
```

**Maximum boundary**:
```
fn answer_accepts_maximum_sized_answer() {
    // answer_content = string of exactly MAX_ANSWER_SIZE bytes
    // Assert: Ok(())
    // Assert: bead_state == Completed
}
```

---

#### Behavior: `answer command rejects answer content exceeding maximum size`

**Given**: A bead in `WaitingAnswer` state
**When**: `answer(bead_id, answer_content)` is called with content exceeding `MAX_ANSWER_SIZE`
**Then**: Returns `Err(E_ANSWER_TOO_LARGE)` or equivalent validation error, bead remains `WaitingAnswer`, no journal event written

```
fn answer_rejects_overflow_answer_content() {
    // answer_content = string of MAX_ANSWER_SIZE + 1 bytes
    // Assert: Err(E_ANSWER_TOO_LARGE) or validation error
    // Assert: bead_state == WaitingAnswer
    // Assert: journal.len() unchanged
}
```

---

### Group G: Structured Diagnostics

---

#### Behavior: `all error variants include structured diagnostics`

**Given**: Any error returned from lifecycle commands
**When**: Error is inspected
**Then**: Error contains all of: `code` (string error code), `context` (human-readable description), `timestamp` (UTC ISO-8601), `bead_id` (target bead), `command` (the command that failed)

```
fn error_variant_includes_code_context_timestamp_bead_id_command() {
    // For each error variant:
    // E_INVALID_TRANSITION, E_DUPLICATE_REQUEST, E_STALE_REQUEST,
    // E_JOURNAL_WRITE_FAILURE, E_REPLAY_CORRUPTION, E_STORAGE_UNAVAILABLE
    // Assert: err.code is non-empty string
    // Assert: err.context is non-empty string
    // Assert: err.timestamp parses as ISO-8601
    // Assert: err.bead_id matches target
    // Assert: err.command matches Cancel|Resume|Retry|Answer
}
```

---

## 4. Proptest Invariants

### Invariant 1: `spec_transition` is deterministic

```
Invariant: spec_transition(bead_id, cmd, state_before) always produces the same state_after for identical inputs
Strategy: Command arbitrary from [Cancel, Resume, Retry, Answer], state_before arbitrary from all 6 LifecycleStates, bead_id arbitrary from valid BeadId range
Anti-invariant: If cmd is invalid for state_before, spec_transition returns error (not a state)
```

### Invariant 2: Journal append is injective

```
Invariant: append_event(event, journal) produces journal' where Len(journal') = Len(journal) + 1, and the new last element equals event
Strategy: Arbitrary valid RuntimeJournalEvent + arbitrary journal of length 0..1000
Anti-invariant: Duplicate event detection is a separate concern (covered by duplicate request tests)
```

### Invariant 3: Replay reconstructs from any valid journal

```
Invariant: For any journal where all events are valid and in-order, replay() produces a bead_state that satisfies all LifecycleState invariants
Strategy: Generate arbitrary valid event sequence of length 1..100, ensuring valid state transitions
Anti-invariant: Journals with invalid transitions (e.g., cancel from Pending) should not be producible by valid command sequence
```

### Invariant 4: Error variants carry all diagnostic fields

```
Invariant: Every error constructor produces an error with all 5 fields (code, context, timestamp, bead_id, command) non-None and well-formed
Strategy: Generate arbitrary error scenarios via command × state × failure-mode combination
Anti-invariant: None — all fields are mandatory in the Error type
```

---

## 5. Fuzz Targets

### Fuzz Target 1: `RuntimeJournalEvent` deserialization

```
Target: crates/vb_runtime/src/journal.rs — event parsing / bincode/serde deserialization
Input type: arbitrary bytes (corpus of valid RuntimeJournalEvent encodings + malformed variants)
Risk: Panic on deserialization, OOM on oversized fields, undefined behavior on invalid discriminant
Corpus seeds: valid event encodings from integration tests, truncated events, garbage bytes, negative discriminant values, oversized string fields
```

### Fuzz Target 2: `lifecycle command` dispatch with arbitrary bead state

```
Target: crates/vb_runtime/src/lifecycle.rs — command validation (corrected path)
Input type: arbitrary (bead_id, command, state) tuples
Risk: Invalid state enum constructed, panic in validation logic, bypass of state check
Corpus seeds: valid (BeadId, Cancel/Resume/Retry/Answer, LifecycleState) combinations, invalid state discriminants
```

---

## 6. Kani Harnesses

**Waived** — per verification-layers.md: "Kani may be used as defense-in-depth if implementation reveals numeric/indexing gaps." Integration tests + TLA+ model cover the state space. No Kani harness required at this time.

---

## 7. Mutation Checkpoints

| Mutation | Target | Must be caught by |
|----------|--------|-------------------|
| Remove cancel state guard (accept cancel from Pending) | `lifecycle.rs` transition fn | `cancel_returns_invalid_transition_when_bead_is_pending` |
| Remove duplicate-request deduplication check | `storage.rs` validate_command | `cancel_returns_duplicate_request_when_called_twice_in_same_state` |
| Remove stale-request sequence-number check | `storage.rs` validate_command | `cancel_returns_stale_request_when_state_already_advanced` |
| Remove journal.append() call from error path | `storage.rs` command handler | `cancel_returns_invalid_transition_when_bead_is_pending` (journal unchanged assertion) |
| Flip journal write to overwrite instead of append | `journal.rs` append_event | `replay_full_journal_reconstructs_bit_identical_state` |
| Remove replay corruption check | `journal.rs` replay | `replay_with_malformed_event_returns_replay_corruption` |

**Threshold**: ≥80% mutation kill rate. cargo-mutants deferred to later beads per explicit waiver in verification-layers.md.

---

## 8. Combinatorial Coverage Matrix

### Unit: Transition Validity (24 scenarios)

| Scenario | Command | Prior State | Expected Output | Layer |
|----------|---------|-------------|----------------|-------|
| cancel from Active | Cancel | Active | Ok(Cancelled) | unit |
| cancel from WaitingAnswer | Cancel | WaitingAnswer | Ok(Cancelled) | unit |
| cancel from Pending | Cancel | Pending | Err(E_INVALID_TRANSITION) | unit |
| cancel from Completed | Cancel | Completed | Err(E_INVALID_TRANSITION) | unit |
| cancel from Failed | Cancel | Failed | Err(E_INVALID_TRANSITION) | unit |
| resume from Cancelled | Resume | Cancelled | Ok(Active) | unit |
| resume from Pending | Resume | Pending | Err(E_INVALID_TRANSITION) | unit |
| resume from Active | Resume | Active | Err(E_INVALID_TRANSITION) | unit |
| resume from WaitingAnswer | Resume | WaitingAnswer | Err(E_INVALID_TRANSITION) | unit |
| resume from Completed | Resume | Completed | Err(E_INVALID_TRANSITION) | unit |
| resume from Failed | Resume | Failed | Err(E_INVALID_TRANSITION) | unit |
| retry from Failed | Retry | Failed | Ok(Active) | unit |
| retry from Pending | Retry | Pending | Err(E_INVALID_TRANSITION) | unit |
| retry from Active | Retry | Active | Err(E_INVALID_TRANSITION) | unit |
| retry from Cancelled | Retry | Cancelled | Err(E_INVALID_TRANSITION) | unit |
| retry from Completed | Retry | Completed | Err(E_INVALID_TRANSITION) | unit |
| retry from WaitingAnswer | Retry | WaitingAnswer | Err(E_INVALID_TRANSITION) | unit |
| answer from WaitingAnswer | Answer | WaitingAnswer | Ok(Completed) | unit |
| answer from Pending | Answer | Pending | Err(E_INVALID_TRANSITION) | unit |
| answer from Active | Answer | Active | Err(E_INVALID_TRANSITION) | unit |
| answer from Cancelled | Answer | Cancelled | Err(E_INVALID_TRANSITION) | unit |
| answer from Completed | Answer | Completed | Err(E_INVALID_TRANSITION) | unit |
| answer from Failed | Answer | Failed | Err(E_INVALID_TRANSITION) | unit |

### Unit: Answer Content Boundaries (3 scenarios)

| Scenario | Answer Input | Expected Output | Layer |
|----------|-------------|----------------|-------|
| answer with empty string | `""` | Ok(Completed) or Err(validation) — spec must decide | unit |
| answer with typical content | `"the answer is 42"` | Ok(Completed) | unit |
| answer with MAX_ANSWER_SIZE bytes | 1 MB of valid UTF-8 | Ok(Completed) | unit |
| answer with MAX_ANSWER_SIZE + 1 bytes | oversized content | Err(E_ANSWER_TOO_LARGE) | unit |

### Unit: Journal Append / Replay (9 scenarios)

| Scenario | Input | Expected Output | Layer |
|----------|-------|----------------|-------|
| append_event happy | valid RuntimeJournalEvent | Ok(()), journal.len() == prior + 1 | unit |
| append_event exact-one-event | one event | journal.len() == 1 exactly (not just >0) | unit |
| append_event JournalWriteFailure | storage I/O fault | Err(E_JOURNAL_WRITE_FAILURE) | unit |
| replay from empty journal | `[]` | Ok(all Pending) | unit |
| replay from clean snapshot | snapshot + empty incremental | Ok(snapshot state) | unit |
| replay full journal fidelity | [e1..eN] | bit-identical to pre-crash | unit |
| replay partial (snapshot + incremental) | [snapshot, e_{K+1..N}] | identical to full replay | unit |
| replay malformed event | [valid..., corrupt] | Err(E_REPLAY_CORRUPTION) | unit |
| replay missing event | [valid..., truncated] | Err(E_REPLAY_CORRUPTION) | unit |

### Unit: Storage Journal write_event / read_journal (17 scenarios)

| Scenario | API | Input | Expected Output | Layer |
|----------|-----|-------|----------------|-------|
| append_journaled happy | write_event | valid JournalEvent | Ok(()), readable via events_for_run | unit |
| append_strict forces barrier | write_event | valid JournalEvent | Ok(()), durable after return | unit |
| append_strict_batch all succeed | write_event | 3 events | Ok(()), all 3 readable | unit |
| append_strict_batch empty | write_event | `[]` | Ok(()) | unit |
| write_event duplicate run+seq | write_event | same event twice | Err(JournalError::DuplicateEvent) | unit |
| write_event oversized payload | write_event | event > MAX_JOURNAL_EVENT_PAYLOAD_BYTES | Err(JournalError::PayloadTooLarge) | unit |
| write_event E_JOURNAL_WRITE_FAILURE | write_event | disk full / fault | Err(E_JOURNAL_WRITE_FAILURE) | unit |
| events_for_run happy | read_journal | run with 5 events | Ok(vec![e0,e1,e2,e3,e4]), seq order | unit |
| events_for_run empty unknown run | read_journal | RunId with no events | Ok(vec![]) | unit |
| events_for_run detects gap | read_journal | seq 0, seq 2 (gap) | Err(JournalError::SequenceGap) | unit |
| events_for_run run isolation | read_journal | run A=2 events, run B=3 events | only run A's events returned | unit |
| events_for_run many events | read_journal | 100 events | Ok(vec![e0..e99]) | unit |
| append_queued_unpersisted idempotent | write_event | same event twice | Ok(()) second time (idempotent) | unit |
| append_queued_unpersisted different dup | write_event | same run+seq, different content | Err(JournalError::DuplicateEvent) | unit |
| declared_keyspaces count | meta | open journal | 9 keyspaces | unit |
| persist_strict on drop | lifecycle | drop journal | no data loss | unit |
| batch commit all succeed | write_event | 8 operations across keyspaces | all 8 readable | unit |

### Unit: Error Construction (6 scenarios)

| Scenario | Error Variant | Fields Complete | Layer |
|----------|--------------|-----------------|-------|
| InvalidTransition error | E_INVALID_TRANSITION | code + context + timestamp + bead_id + command | unit |
| DuplicateRequest error | E_DUPLICATE_REQUEST | all 5 fields | unit |
| StaleRequest error | E_STALE_REQUEST | all 5 fields | unit |
| JournalWriteFailure error | E_JOURNAL_WRITE_FAILURE | all 5 fields | unit |
| ReplayCorruption error | E_REPLAY_CORRUPTION | all 5 fields | unit |
| StorageUnavailable error | E_STORAGE_UNAVAILABLE | all 5 fields | unit |

**Unit total: 24 + 3 + 9 + 17 + 6 = 59 scenarios → 45 unique test functions** (some matrix rows share one test function). Confirmed ≥40 unit tests ✓.

### Integration: Lifecycle Happy Path

| Scenario | Setup | Command | Expected | Layer |
|----------|-------|---------|----------|-------|
| cancel from Active | bead Active | cancel | Ok, 1 event, Cancelled | integration |
| cancel from WaitingAnswer | bead WaitingAnswer | cancel | Ok, 1 event, Cancelled | integration |
| resume from Cancelled | bead Cancelled | resume | Ok, 1 event, Active | integration |
| retry from Failed | bead Failed | retry | Ok, 1 event, Active | integration |
| answer from WaitingAnswer | bead WaitingAnswer | answer | Ok, 1 event, Completed | integration |

### Integration: Replay

| Scenario | Journal State | Expected | Layer |
|----------|--------------|----------|-------|
| empty journal replay | [] | all Pending | integration |
| clean snapshot replay | [snapshot] | snapshot state | integration |
| full replay fidelity | [e1, e2, ..., eN] | bit-identical to pre-crash | integration |
| partial replay | [snapshot, e_{K+1..N}] | identical to full replay | integration |
| malformed event | [valid..., corrupt] | E_REPLAY_CORRUPTION | integration |
| missing event | [valid..., truncated] | E_REPLAY_CORRUPTION | integration |

### Integration: Storage I/O Errors

| Scenario | Failure Mode | Expected Error | Layer |
|----------|-------------|----------------|-------|
| journal write I/O error | disk full / I/O fault | E_JOURNAL_WRITE_FAILURE | integration |
| storage unavailable at dispatch | backend not connected | E_STORAGE_UNAVAILABLE | integration |
| storage unavailable at replay | connection lost mid-replay | E_STORAGE_UNAVAILABLE | integration |

### E2E: CLI Lifecycle Smoke

| Scenario | CLI Args | Expected | Layer |
|---------|---------|----------|-------|
| cancel bead via CLI | `velvet_ballastics cancel <bead_id>` | success exit code, journal entry | e2e |
| full lifecycle via CLI | cancel, then resume | correct state transitions | e2e |

---

## 9. Manual QA Expectations

### MANUAL-QA-001: Storage Unavailable

**Trigger**: Disconnect storage backend (or simulate via firewall/umount), then issue lifecycle commands.

**Expected**:
- Commands return `E_STORAGE_UNAVAILABLE`
- Structured diagnostics include `code`, `context`, `timestamp`, `bead_id`, `command`
- No state corruption on storage reconnect
- After reconnect, `replay()` correctly reconstructs pre-disconnect state from journal

**Evidence artifact**: `manual-qa-smoke.md` with command output, timestamps, and error diagnostics.

**Note**: PRE-001 (storage backend not connected) is NOW also covered at integration layer (Group J) via `NoopStorage` / `StorageFault` test adapter. MANUAL-QA-001 remains for human verification of the real CLI behavior.

---

## 10. TLA+ Model Evidence Confirmation

**TLA+ Model Status**: `specs/tla/RecoveryReplay.tla` — **PRESENT** ✓

The model covers:
- INV-002 (append-only journal semantics)
- INV-003 (valid transition enforcement)
- INV-004 (replay bit-identical)
- POST-003 (invalid-transition rejection)
- POST-004 (duplicate-request rejection, NoDuplicateNonIdempotent theorem)
- POST-005 (stale-request rejection, ReplaySafe theorem)

Evidence command:
```bash
tlc -config specs/tla/RecoveryReplay.cfg specs/tla/RecoveryReplay.tla
```
Expected: No invariant violations, both theorems (`Spec => []NoDuplicateNonIdempotent`, `Spec => []ReplaySafe`) satisfied.

**Gap**: `specs/LifecycleJournal.tla` (referenced in tla-spec.md §Model Shape, line 37) is **NOT YET AUTHORED**. Per tla-spec.md line 37, formal-verifier bead at State 12 owns this artifact. This is an acknowledged gap — the RecoveryReplay.tla provides partial coverage of lifecycle state machine semantics until LifecycleJournal.tla is authored.

---

## 11. Open Questions

1. **Storage fault injection**: What is the approved mechanism for simulating `E_JOURNAL_WRITE_FAILURE` in integration tests? Is a `StorageFault` trait / test-only `NoopStorage` used, or a wrapper over the real storage adapter? (PRE-001 integration test requires this.)
2. **Journal format**: Is the journal binary (bincode/cbor) or text-based? Affects proptest strategy bounds. Confirmed: binary via `postcard`/`bincode` per vb_storage codec.
3. **BeadId type**: Is `BeadId` a UUID, integer, or string? Affects proptest strategy bounds.
4. **Answer type**: What is the type of `answer` in `answer(bead_id, answer)`? Affects proptest and BDD scenario coverage. Is it `String`, `Vec<u8>`, or a custom `AnswerContent` type?
5. **Snapshot format**: What does the on-disk snapshot look like? Needed to write `REPLAY-002` partial replay scenarios.
6. **TLA+ model confirmation**: `specs/tla/RecoveryReplay.tla` EXISTS and is authored (covers INV-002, INV-003, INV-004, POST-003, POST-004, POST-005 for recovery replay). `specs/LifecycleJournal.tla` referenced in tla-spec.md line 37 is NOT yet authored — formal-verifier bead at State 12 owns this per explicit plan. This is a known gap; evidence from RecoveryReplay.tla provides partial coverage of lifecycle semantics.
7. **Crash simulation mechanism**: How is crash simulated in integration tests — process kill, in-memory state drop, or filesystem-level snapshot?

---

## 12. Test Naming Convention

All Rust test functions follow the pattern:

```
fn [subject]_[outcome]_when_[condition]()
```

Examples:
- `fn cancel_succeeds_when_bead_is_active()`
- `fn cancel_returns_invalid_transition_when_bead_is_pending()`
- `fn cancel_returns_duplicate_request_when_called_twice()`
- `fn replay_from_empty_journal_produces_valid_initial_state()`
- `fn error_variant_includes_all_structured_diagnostic_fields()`

---

*Plan authored: 2026-05-11*
*Bead: vb-qi37.16.5*
*Contract approved: YES (contract-verification-review.md, 2026-05-11)*

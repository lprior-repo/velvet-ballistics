# Test Plan: CLI Lifecycle Tracker Event-Applied (vb-0253.7)

## Summary

| Metric | Value |
|--------|-------|
| Bead ID | vb-0253.7 |
| Title | cli: Make lifecycle tracker event-applied |
| Primary Crate | `vb_cli` (`crates/vb_cli/src/lifecycle.rs`) |
| Behaviors identified | 12 |
| Trophy allocation | 8 unit / 14 integration / 2 e2e |
| Proptest invariants | 3 |
| Fuzz targets | 2 |
| Kani harnesses | 3 (existing) |

---

## 1. Behavior Inventory

### B-001: `cancel` cancels a run from Active/WaitingAnswer
**Subject**: `cancel(run, journal)`
**Action**: Writes `JournalEvent::RunCancelled`, derives state `Cancelled`
**Outcome**: Returns `Ok(())`, journal contains exactly one `RunCancelled` event
**Condition**: Run in `Active` or `WaitingAnswer` state; run exists in journal

### B-002: `cancel` rejects invalid prior states
**Subject**: `cancel(run, journal)`
**Action**: Validates current state before any write
**Outcome**: Returns `Err(LifecycleInvalidTransition)`; journal unmodified
**Condition**: Run in `Pending`, `Failed`, `Completed`, or `Cancelled` state

### B-003: `cancel` rejects duplicate requests
**Subject**: `cancel(run, journal)` called twice
**Action**: Detects `Cancelled` state from tracker (current) or journal (post-refactor)
**Outcome**: Returns `Err(LifecycleDuplicateRequest)`; journal has exactly 1 event
**Condition**: Run already cancelled

### B-004: `cancel` rejects stale terminal state
**Subject**: `cancel(run, journal)`
**Action**: Checks `is_terminal()` before transition validation
**Outcome**: Returns `Err(LifecycleStaleRequest)`; journal unmodified
**Condition**: Run in `Completed` state

### B-005: `resume` resumes a run from Cancelled/WaitingAnswer
**Subject**: `resume(run, journal)`
**Action**: Writes `JournalEvent::RunResumed`, derives state `Active`
**Outcome**: Returns `Ok(())`, journal contains exactly one `RunResumed` event
**Condition**: Run in `Cancelled` or `WaitingAnswer` state; run exists in journal

### B-006: `resume` rejects invalid prior states
**Subject**: `resume(run, journal)`
**Action**: Validates current state before any write
**Outcome**: Returns `Err(LifecycleInvalidTransition)`; journal unmodified
**Condition**: Run in `Pending`, `Active`, `Failed`, or `Completed` state

### B-007: `retry` retries a run from Failed state
**Subject**: `retry(run, journal)`
**Action**: Writes `JournalEvent::RunRetried`, derives state `Active`
**Outcome**: Returns `Ok(())`, journal contains exactly one `RunRetried` event
**Condition**: Run in `Failed` state; run exists in journal

### B-008: `retry` rejects invalid prior states
**Subject**: `retry(run, journal)`
**Action**: Validates current state before any write
**Outcome**: Returns `Err(LifecycleInvalidTransition)`; journal unmodified
**Condition**: Run not in `Failed` state

### B-009: `answer` answers a run from WaitingAnswer state
**Subject**: `answer(run, answer, journal)`
**Action**: Writes `JournalEvent::RunAnswered`, derives state `Completed`
**Outcome**: Returns `Ok(())`, journal contains exactly one `RunAnswered` event
**Condition**: Run in `WaitingAnswer` state; run exists in journal

### B-010: `answer` rejects non-WaitingAnswer states
**Subject**: `answer(run, answer, journal)`
**Action**: Validates current state before any write
**Outcome**: Returns `Err(LifecycleInvalidTransition)` or `Err(LifecycleStaleRequest)`; journal unmodified
**Condition**: Run not in `WaitingAnswer` state

### B-011: `replay` derives all run states from journal
**Subject**: `replay(journal)`
**Action**: Iterates all run headers, derives final state from last event per run
**Outcome**: Returns `Vec<RunState>` where each `lifecycle` matches `derive_lifecycle_state_from_events`
**Condition**: Journal accessible; no corruption

### B-012: `derive_lifecycle_state_from_events` maps last event to correct state
**Subject**: `derive_lifecycle_state_from_events(events)`
**Action**: Pure function, no side effects
**Outcome**: Returns correct `LifecycleState` per event type; defaults to `Pending` on empty
**Condition**: Any `&[JournalEvent]` slice

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Unit / Calc** | 8 | `derive_lifecycle_state_from_events` is pure; error variant returns; `check_lifecycle_transition` truth table |
| **Integration** | 14 | Journal writes + state derivation + replay; real FjallJournal; all command × state combos |
| **E2E / CLI** | 2 | Full `replay` from CLI; journal integrity after multiple commands |
| **Static Analysis** | — | clippy/semver are separate proof obligations (STATIC-LINT-001, SEMVER-001) |

**Rationale**: This is a journal-dependent system. The critical property (INV-001: state-journal consistency) can only be proven with real journal I/O. Unit tests cover the pure derivation function exhaustively.

---

## 3. BDD Scenarios

### B-001: cancel from Active

**Given**: Run exists in journal with `Active` state (via `RunAccepted` event)
**When**: `cancel(run, journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunCancelled` event; replay derives `Cancelled`

**Given**: Run exists in journal with `WaitingAnswer` state
**When**: `cancel(run, journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunCancelled` event; replay derives `Cancelled`

### B-002: cancel rejects invalid states

**Given**: Run exists in journal with no events (implicit `Pending`)
**When**: `cancel(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Completed` state
**When**: `cancel(run, journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

**Given**: Run in `Cancelled` state
**When**: `cancel(run, journal)` is called
**Then**: Returns `Err(LifecycleDuplicateRequest)`; journal unchanged (already has `RunCancelled`)

**Given**: Run in `Failed` state
**When**: `cancel(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

### B-005: resume from Cancelled

**Given**: Run in journal with `Cancelled` state (has `RunCancelled` event)
**When**: `resume(run, journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunResumed` event; replay derives `Active`

**Given**: Run in `WaitingAnswer` state
**When**: `resume(run, journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunResumed` event; replay derives `Active`

### B-006: resume rejects invalid states

**Given**: Run in `Pending` state
**When**: `resume(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Active` state
**When**: `resume(run, journal)` is called
**Then**: Returns `Err(LifecycleDuplicateRequest)`; journal unchanged

**Given**: Run in `Completed` state
**When**: `resume(run, journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

**Given**: Run in `Failed` state
**When**: `resume(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

### B-007: retry from Failed

**Given**: Run in journal with `Failed` state (has `RunFailedEvent`)
**When**: `retry(run, journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunRetried` event; replay derives `Active`

### B-008: retry rejects invalid states

**Given**: Run in `Pending` state
**When**: `retry(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Active` state
**When**: `retry(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Cancelled` state
**When**: `retry(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Completed` state
**When**: `retry(run, journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

**Given**: Run in `WaitingAnswer` state
**When**: `retry(run, journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

### B-009: answer from WaitingAnswer

**Given**: Run in `WaitingAnswer` state (has `AskScheduledEvent` or `WaitScheduledEvent`)
**When**: `answer(run, "answer_content", journal)` is called
**Then**: Returns `Ok(())`; journal has exactly 1 `RunAnswered` event; replay derives `Completed`

### B-010: answer rejects non-WaitingAnswer

**Given**: Run in `Pending` state
**When**: `answer(run, "x", journal)` is called
**Then**: Returns `Err(LifecycleInvalidTransition)`; journal unchanged

**Given**: Run in `Active` state
**When**: `answer(run, "x", journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

**Given**: Run in `Cancelled` state
**When**: `answer(run, "x", journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

**Given**: Run in `Completed` state
**When**: `answer(run, "x", journal)` is called
**Then**: Returns `Err(LifecycleDuplicateRequest)`; journal unchanged

**Given**: Run in `Failed` state
**When**: `answer(run, "x", journal)` is called
**Then**: Returns `Err(LifecycleStaleRequest)`; journal unchanged

### B-011: replay derives states from journal

**Given**: Empty journal
**When**: `replay(journal)` is called
**Then**: Returns `Ok(Vec::new())`

**Given**: Journal with 2 runs: run1→Cancelled, run2→Active
**When**: `replay(journal)` is called
**Then**: Returns `Vec<RunState>` with 2 entries; run1.lifecycle=Cancelled, run2.lifecycle=Active

### B-012: derive_lifecycle_state_from_events

| Input last event | Expected state |
|-----------------|---------------|
| `RunCancelled` | `Cancelled` |
| `RunResumed` | `Active` |
| `RunRetried` | `Active` |
| `RunAnswered` | `Completed` |
| `RunFinished` | `Completed` |
| `RunFailedEvent` | `Failed` |
| `RunAccepted` | `Active` |
| `RunAdmission` | `Active` |
| `StepStarted` | `Active` |
| `StepSucceeded` | `Active` |
| `ActionScheduled` | `Active` |
| `SlotWrittenEvent` | `Active` |
| `ActionCompletedEvent` | `Active` |
| `WaitScheduledEvent` | `WaitingAnswer` |
| `AskScheduledEvent` | `WaitingAnswer` |
| `AskAnsweredEvent` | `WaitingAnswer` |
| `RetryScheduledEvent` | `Active` |
| `ActionFailedEvent` | `Failed` |
| empty slice | `Pending` |

---

## 4. Proptest Invariants

### P-001: `derive_lifecycle_state_from_events` idempotence
**Function**: `derive_lifecycle_state_from_events`
**Invariant**: For any non-empty event sequence, appending an event and deriving produces the same result as deriving from the original sequence with the new event as last
**Strategy**: `prop_oneof![vec(arb_event())]` where last element is varied
**Anti-invariant**: Empty sequence always returns `Pending`

### P-002: `derive_lifecycle_state_from_events` last-event-only
**Function**: `derive_lifecycle_state_from_events`
**Invariant**: The function result depends ONLY on the last element; modifying earlier elements does not change outcome
**Strategy**: `vec(arb_event(), 3..10)` with fixed last element

### P-003: `check_lifecycle_transition` total function
**Function**: `check_lifecycle_transition`
**Invariant**: Returns `bool` for all 24 `(LifecycleState, LifecycleCommand)` combinations; never panics
**Strategy**: `any::<(LifecycleState, LifecycleCommand)>()`

---

## 5. Fuzz Targets

### F-001: `derive_lifecycle_state_from_events` with arbitrary event sequences
**Input type**: `Vec<JournalEvent>`
**Risk**: Panic on invalid enum variant (should be impossible if `JournalEvent` is closed), logic error in event→state mapping
**Corpus seeds**: One sequence per event type as last element; empty sequence; single-element sequences

### F-002: Journal event roundtrip through `replay`
**Input type**: Journal with generated events (via `inject_raw_event` or `inject_seq_gap`)
**Risk**: `replay` panics on malformed events; returns incorrect states on sequence gaps
**Corpus seeds**: Empty journal; single event; multiple runs; malformed byte sequences

---

## 6. Kani Harnesses

*Note: Existing harnesses in `verification/kani/` cover critical paths. These are referenced from proof-obligations.planned.jsonl and are NOT written here — they are already written by the proof-writer role.*

| Harness | Target | Property |
|---------|--------|----------|
| `lifecycle_preconditions` | KANI-002 | All valid (state, command) pairs pass preconditions |
| `lifecycle_commands` | KANI-001 | Bounded transition sequences never panic; invalid paths return correct errors |
| `lifecycle_commands_valid` | KANI-002 | All valid run scenarios complete with 0 failures |

---

## 7. Mutation Checkpoints

| Mutation | Must be caught by |
|----------|------------------|
| `derive_lifecycle_state_from_events`: change `RunCancelled` mapping from `Cancelled` to `Active` | `test_cancel_produces_cancelled_state` |
| `cancel`: remove `is_terminal()` check before duplicate detection | `test_cancel_returns_stale_request_when_state_already_advanced` |
| `cancel`: skip `check_lifecycle_transition` call | `test_cancel_returns_invalid_transition_when_bead_is_pending` |
| `resume`: change `Cancelled` check to `Completed` | `test_resume_returns_stale_request_when_not_in_cancelled_state` |
| `retry`: change `Failed` check to `Active` | `test_retry_returns_invalid_transition_when_bead_is_active` |
| `answer`: remove `WaitingAnswer` guard | `test_answer_returns_invalid_transition_when_bead_is_pending` |
| `replay`: skip `derive_lifecycle_state_from_events` call, return default | `test_replay_state_matches_journal` |

**Threshold**: ≥90% mutation kill rate

---

## 8. Combinatorial Coverage Matrix

### Unit: `derive_lifecycle_state_from_events`

| Scenario | Input | Expected Output |
|----------|-------|-----------------|
| Happy: RunCancelled | `[RunCancelled]` | `Cancelled` |
| Happy: RunResumed | `[RunResumed]` | `Active` |
| Happy: RunRetried | `[RunRetried]` | `Active` |
| Happy: RunAnswered | `[RunAnswered]` | `Completed` |
| Happy: RunFinished | `[RunFinished]` | `Completed` |
| Happy: RunFailedEvent | `[RunFailedEvent]` | `Failed` |
| Happy: AskScheduledEvent | `[AskScheduledEvent]` | `WaitingAnswer` |
| Happy: ActionFailedEvent | `[ActionFailedEvent]` | `Failed` |
| Empty | `[]` | `Pending` |
| Mixed: last wins | `[RunAccepted, RunCancelled]` | `Cancelled` |

### Unit: `check_lifecycle_transition` (24 combos)

| State \ Command | Cancel | Resume | Retry | Answer |
|----------------|--------|--------|-------|--------|
| Pending | false | false | false | false |
| Active | true | false | false | false |
| WaitingAnswer | true | true | false | true |
| Cancelled | false | false | false | false |
| Completed | false | false | false | false |
| Failed | false | false | true | false |

### Integration: cancel × 6 prior states

| Prior State | Expected Result | Journal Events |
|-------------|-----------------|---------------|
| Pending | `InvalidTransition` | 0 |
| Active | `Ok` | 1 (RunCancelled) |
| WaitingAnswer | `Ok` | 1 (RunCancelled) |
| Cancelled | `DuplicateRequest` | 1 (unchanged) |
| Completed | `StaleRequest` | 0 |
| Failed | `InvalidTransition` | 0 |

### Integration: answer × 6 prior states

| Prior State | Expected Result | Journal Events |
|-------------|-----------------|---------------|
| Pending | `InvalidTransition` | 0 |
| Active | `StaleRequest` | 0 |
| WaitingAnswer | `Ok` | 1 (RunAnswered) |
| Cancelled | `StaleRequest` | 0 |
| Completed | `DuplicateRequest` | 1 (unchanged) |
| Failed | `StaleRequest` | 0 |

---

## 9. Mapping: Requirements → Test Cases → Proof Obligations

| Requirement | Test Cases | Proof Obligations |
|-------------|-----------|-------------------|
| INV-001: State-Journal Consistency | `test_state_derivation_from_events_matches_journal`, `test_cancel_produces_cancelled_state`, `test_resume_produces_active_state`, `test_retry_produces_active_state`, `test_answer_produces_completed_state` | TLA-LIFECYCLE-001, VERUS-DERIVE-001 |
| INV-002: No Divergence | `test_no_divergence_between_tracker_and_journal`, `test_state_persists_across_restarts` | TLA-LIFECYCLE-002, STATIC-LINT-001 |
| INV-003: Valid Transitions Only | `test_invalid_transition_rejected`, `test_cannot_cancel_completed_run`, `test_cannot_answer_non_waiting_run` | VERUS-TRANSITION-001, KANI-001 |
| INV-004: Event Immutability | `test_events_are_append_only`, `test_event_order_preserved` | TLA-LIFECYCLE-001, MIRI-001 |
| INV-005: Terminal States Final | `test_completed_run_accepts_no_commands`, `test_cancelled_run_accepts_no_commands` | TLA-LIFECYCLE-003, VERUS-TRANSITION-001 |
| PRE-001: RunId exists | `test_returns_error_for_nonexistent_run` | KANI-002, MIRI-001 |
| PRE-002: WaitingAnswer for answer | `test_answer_requires_waiting_answer_state`, `test_answer_on_active_run_returns_error` | KANI-002, VERUS-TRANSITION-001 |
| PRE-003: Non-terminal for cancel/resume/retry | `test_cannot_cancel_completed_run`, `test_cannot_resume_completed_run`, `test_cannot_retry_completed_run` | KANI-002, VERUS-TRANSITION-001 |
| POST-001: cancel → Cancelled | `test_cancel_append_correct_event`, `test_cancel_produces_cancelled_state` | POST-CANCEL-001, TLA-LIFECYCLE-001 |
| POST-002: resume → Active | `test_resume_append_correct_event`, `test_resume_produces_active_state` | POST-RESUME-001, TLA-LIFECYCLE-001 |
| POST-003: retry → Active | `test_retry_append_correct_event`, `test_retry_produces_active_state` | POST-RETRY-001, TLA-LIFECYCLE-001 |
| POST-004: answer → Completed | `test_answer_append_correct_event`, `test_answer_produces_completed_state` | POST-ANSWER-001, TLA-LIFECYCLE-001 |
| POST-005: Ok/Err returns | `test_cancel_returns_ok_on_success`, `test_returns_lifecycle_error_variants` | VERUS-TRANSITION-001, KANI-001 |
| POST-006: replay pure derivation | `test_replay_returns_all_run_states`, `test_replay_state_matches_journal` | POST-REPLAY-001, TLA-LIFECYCLE-001 |
| API-001: Public API unchanged | `test_public_api_compiles` | SEMVER-001 |

---

## 10. Open Questions

| ID | Question | Status |
|----|---------|--------|
| Q1 | Does `journal.events_for_run(run)` return events in guaranteed chronological order? | Assumed YES (A1) — test with `test_event_order_preserved` |
| Q2 | Is there any existing code that bypasses journal writes and directly calls `with_tracker_mut`? | Must be checked via code search before refactoring lands |
| Q3 | Are there external consumers of the in-memory tracker state besides CLI commands? | Must be verified before `static TRACKER` removal |

---

## 11. Gate Criteria

| Gate | Command | Criteria |
|------|---------|----------|
| **Compile** | `cargo check -p vb_cli` | Zero errors |
| **Unit tests** | `cargo test -p vb_cli --lib` | All pass |
| **Integration tests** | `cargo test -p vb_cli --test lifecycle_integration` | All pass |
| **Clippy** | `cargo clippy -p vb_cli --lib --bins -- -D warnings` | Zero warnings |
| **Kani** | `cargo kani -p vb_cli` | 0 unproven targets |
| **Miri** | `cargo miri test -p vb_cli --lib` | 0 UB violations |
| **Semver** | `cargo semver-checks -p vb_cli` | 0 violations |

---

*Generated by test-planner for vb-0253.7*
*Inputs: contract.md, proof-obligations.planned.jsonl, traceability-matrix.jsonl, proof-strategy.md*

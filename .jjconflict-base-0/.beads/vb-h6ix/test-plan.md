# Test Plan: vb-h6ix — Replay Latest Execution Attempt Only

## Beacon

- **Bead**: vb-h6ix
- **Feature**: Runtime recovery replay that reconstructs the latest execution attempt per run and ignores stale attempt events without losing diagnostic evidence.
- **Verified kernel**: `vb_storage/src/recovery/replay/core.rs`
- **Target crate**: `vb_storage`
- **Status**: implementation planned

---

## Section 1 — Behavior Inventory

All behaviors are expressed as `[Subject] [action] [outcome] when [condition]`.

### Core Replay

1. **replay_events returns all events** when given a valid ordered event slice (including stale ones for diagnostics).
2. **replay_events detects out-of-order steps** and returns `Err(ReplayDivergence)` when a `StepStarted` has a lower index than the previously observed step.
3. **replay_events blocks duplicate action scheduling** and returns `Err(NonIdempotentActionBlocked)` when an `ActionScheduled` matches an already-resolved `(action, step)` pair.
4. **replay_events marks action completed** in `ActionReplayTracker` when `ActionCompletedEvent` is processed.
5. **replay_events marks action failed** in `ActionReplayTracker` when `ActionFailedEvent` is processed.
6. **replay_events is deterministic** — same input events in same order yield identical tracker state.
7. **recover_full_journal returns NoRecoveryData** when the journal has no events for the given run.
8. **recover_full_journal propagates journal errors** as `Err(RecoveryError::Journal(...))`.
9. **recover_snapshot_plus_tail rejects tail events** with `seq <= snapshot.seq` and returns `Err(ReplayDivergence)`.
10. **recover_snapshot_plus_tail accepts valid tail events** and returns them with tracker populated.
11. **load_snapshot returns CorruptSnapshot** when the snapshot is `None` or decode fails with `PostcardDecodeFailed`.
12. **load_snapshot propagates other journal errors** as `Err(RecoveryError::Journal(...))`.

### Latest-Attempt Filtering (vb-h6ix extension)

13. **replay_events filters to latest attempt** — only events with `attempt = max_attempt(sequence)` affect live state.
14. **replay_events preserves stale events in output** — all input events (including stale) are returned in the replayed list.
15. **max_attempt is computed from action-scheduling and action-completion events** only.
16. **stale events do not populate ActionReplayTracker** — only latest-attempt completions are recorded.
17. **stale RunFinished does not win** — `extract_terminal` returns the terminal from the latest attempt, not the earliest.
18. **stale timer/wait/suspend events do not allocate live pending actions** — stale `WaitScheduledEvent`, `AskScheduledEvent`, `RetryScheduledEvent` are excluded from pending_actions.
19. **stale slot writes do not appear in frame seed** — `SlotWrittenEvent` from older attempts are excluded from slot recovery.

### Terminal Extraction

20. **is_terminal_event returns true** for `RunFinished`, `RunCancelled`, `RunFailedEvent`.
21. **is_terminal_event returns false** for all other event kinds.
22. **extract_terminal returns the last terminal event** (highest seq) from the event slice, or `None` if no terminal event exists.
23. **extract_terminal returns the latest-attempt terminal** when stale terminals exist earlier in the sequence.

### Error Variants

24. **RecoveryError::ReplayDivergence** carries the step index and a detail string.
25. **RecoveryError::NonIdempotentActionBlocked** carries the `action` and `step`.
26. **RecoveryError::NoRecoveryData** carries the `run` identifier.
27. **RecoveryError::CorruptSnapshot** carries the `run` and `seq`.
28. **RecoveryError::Journal** wraps the underlying `JournalError`.

---

## Section 2 — Trophy Allocation

| Behavior | Layer | Justification |
|----------|-------|---------------|
| replay_events core logic (determinism, divergence, blocking) | unit + integration | Pure function, exhaustive event-kind coverage |
| Latest-attempt filtering | unit + proptest | Core new behavior; mixed-attempt journal generation |
| stale events excluded from tracker | proptest + kani | Invariant: property + bounded proof |
| stale terminal does not win | proptest + kani | INV-005; critical correctness |
| extract_terminal latest-attempt semantics | unit + proptest | Pure function with clear input/output |
| recover_full_journal empty journal | unit (integration with tempfile) | FjallJournal integration |
| recover_snapshot_plus_tail seq validation | unit | Boundary condition on seq comparison |
| load_snapshot decode failures | unit | Error path coverage |
| All RecoveryError variants | unit | Every enum variant must be tested |
| INV-001 determinism | proptest + kani | Broad input space + bounded model check |
| INV-003 stale no allocation | kani + proptest | Formal proof + adversarial interleaving |
| PRE-001 attempt number extraction | proptest + cargo-fuzz | Property + malformed sequence fuzzing |
| PRE-002 deterministic ordering | proptest | Already covered by existing journal tests |

**Target ratio**: ~60% integration, ~30% unit, ~5% proptest, ~5% kani/fuzz.

---

## Section 3 — BDD Scenarios (per behavior)

### Behavior: replay_events returns all events including stale when given valid event slice
Given: A journal with events from attempt 1 and attempt 2 for run `R`
When: `replay_events(events, &mut tracker)` is called
Then: The returned `Vec<JournalEvent>` has the same length as the input, and all input events are present in order

### Behavior: replay_events filters to latest attempt and ignores stale events
Given: A journal with interleaved attempt 1 and attempt 2 events for run `R` where attempt 2 is max
When: `replay_events` processes the events
Then: Only events with `attempt = 2` populate the tracker; events with `attempt = 1` are returned in output but do not affect tracker state

### Behavior: replay_events detects out-of-order step execution
Given: Events where `StepStarted { step: 2 }` appears before `StepStarted { step: 1 }`
When: `replay_events(events, &mut tracker)` is called
Then: Returns `Err(RecoveryError::ReplayDivergence { step: 1, detail: "step 1 executed before previous step 2" })`

### Behavior: replay_events blocks non-idempotent action re-execution
Given: `ActionScheduled { action: A, step: S }` followed by `ActionCompletedEvent { action: A, step: S }` already in tracker, then a second `ActionScheduled { action: A, step: S }`
When: `replay_events(events, &mut tracker)` is called
Then: Returns `Err(RecoveryError::NonIdempotentActionBlocked { action: A, step: S })`

### Behavior: replay_events marks action completed in tracker
Given: `ActionScheduled { action: A, step: S }` followed by `ActionCompletedEvent { action: A, step: S }`
When: `replay_events` processes these events
Then: `tracker.is_resolved(A, S)` returns `true` after replay

### Behavior: replay_events marks action failed in tracker
Given: `ActionScheduled { action: A, step: S }` followed by `ActionFailedEvent { action: A, step: S }`
When: `replay_events` processes these events
Then: `tracker.is_resolved(A, S)` returns `true` after replay

### Behavior: recover_full_journal returns NoRecoveryData for empty journal
Given: An empty FjallJournal temp directory and a run `R` with no events
When: `recover_full_journal(&journal, R, &mut tracker)` is called
Then: Returns `Err(RecoveryError::NoRecoveryData { run: R })`

### Behavior: recover_snapshot_plus_tail rejects events before snapshot seq
Given: A snapshot with `seq = 5` and tail events with `seq = 3`
When: `recover_snapshot_plus_tail(&snapshot, tail_events, &mut tracker)` is called
Then: Returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "tail event seq 3 is not after snapshot seq 5" })`

### Behavior: extract_terminal returns last terminal event
Given: Events ending with `RunFinished { result: slot_1 }` at seq 10 and `RunCancelled` at seq 5
When: `extract_terminal(events)` is called
Then: Returns `Some(&RunFinished { result: slot_1 })`

### Behavior: extract_terminal returns latest-attempt terminal when stale exists
Given: Events with stale `RunFinished` from attempt 1 at seq 5 and `RunFailedEvent` from attempt 2 (latest) at seq 8
When: `extract_terminal(events)` is called
Then: Returns `Some(&RunFailedEvent)` — the latest-attempt terminal, not the stale one

### Behavior: is_terminal_event correctly classifies all terminal kinds
Given: `RunFinished`, `RunCancelled`, and `RunFailedEvent` events
When: `is_terminal_event` is called on each
Then: Returns `true` for all three; returns `false` for `RunAccepted`, `StepStarted`, `ActionScheduled`, and all other kinds

### Behavior: load_snapshot returns CorruptSnapshot on decode failure
Given: A journal where `journal.snapshot(run, seq)` returns `Err(JournalError::PostcardDecodeFailed)`
When: `load_snapshot(&journal, run, seq)` is called
Then: Returns `Err(RecoveryError::CorruptSnapshot { run, seq })`

### Behavior: stale slot writes do not appear in frame seed
Given: A mixed-attempt journal where attempt 1 writes slot 0 and attempt 2 writes slot 1
When: Frame seed is recovered
Then: Only slot 1 from attempt 2 appears in `RecoveryFrameSeed::slots`

### Behavior: stale pending actions are excluded from recovered state
Given: A mixed-attempt journal where attempt 1 schedules a wait and attempt 2 schedules an ask
When: Frame seed is recovered
Then: Only the ask from attempt 2 appears in `RecoveryFrameSeed::pending_actions`

### Behavior: empty event slice returns empty replay output
Given: An empty `&[]` event slice and a fresh tracker
When: `replay_events(&[], &mut tracker)` is called
Then: Returns `Ok(Vec::new())` — empty vector, no error

### Error variant: RecoveryError::ReplayDivergence
Given: Out-of-order step events
When: `replay_events` processes them
Then: Returns `Err(RecoveryError::ReplayDivergence { step: StepIdx, detail: String })`

### Error variant: RecoveryError::NonIdempotentActionBlocked
Given: A duplicate action scheduled from stale attempt
When: `replay_events` processes it
Then: Returns `Err(RecoveryError::NonIdempotentActionBlocked { action: ActionId, step: StepIdx })`

### Error variant: RecoveryError::Journal
Given: A journal read failure from `events_for_run`
When: `recover_full_journal` is called
Then: Returns `Err(RecoveryError::Journal(underlying))`

### Error variant: RecoveryError::NoRecoveryData
Given: A run with no events in the journal
When: `recover_full_journal` is called
Then: Returns `Err(RecoveryError::NoRecoveryData { run: RunId })`

### Error variant: RecoveryError::CorruptSnapshot
Given: A snapshot decode that returns `None` or `PostcardDecodeFailed`
When: `load_snapshot` is called
Then: Returns `Err(RecoveryError::CorruptSnapshot { run, seq })`

---

## Section 4 — Proptest Invariants

### Invariant: Deterministic replay (INV-001)
**Property**: For any fixed `&[JournalEvent]` input, calling `replay_events(events, &mut tracker_a)` and `replay_events(events, &mut tracker_b)` twice produces:
- Identical returned `Vec<JournalEvent>` length and ordering
- `tracker_a.is_resolved(a,s) == tracker_b.is_resolved(a,s)` for all `(action, step)` pairs

**Strategy**: Use `proptest!` with `Vec<JournalEvent>` input, shrinking to minimal counterexamples.

### Invariant: Stale events do not allocate live state (INV-003)
**Property**: Given a mixed-attempt journal, after `replay_events`:
- No `SlotWrittenEvent` from `attempt < max_attempt` contributes to any output structure
- No `WaitScheduledEvent`, `AskScheduledEvent`, or `RetryScheduledEvent` from stale attempts appears in pending_actions

**Strategy**: Generate journals with attempt numbers 1..N interleaved. Verify that only max_attempt events affect tracker.

### Invariant: Tracker records only latest attempt (INV-004)
**Property**: After replay of a mixed-attempt journal:
- `tracker.is_resolved(a,s)` is true only if the completing `ActionCompletedEvent` or `ActionFailedEvent` has `attempt = max_attempt`

**Strategy**: Generate arbitrary interleaved attempts. Verify tracker contents against expected max-attempt completions.

### Invariant: Stale terminal does not win (INV-005)
**Property**: For any journal with terminal events from multiple attempts, `extract_terminal(events)` returns the terminal whose attempt number equals the max attempt number across the journal (falling back to highest seq among max-attempt terminals).

**Strategy**: Generate journals with `RunFinished`/`RunFailedEvent`/`RunCancelled` at various seqs and attempts. Verify `extract_terminal` returns the max-attempt terminal.

### Invariant: Replay divergence on out-of-order steps (ERR-ReplayDivergence)
**Property**: Any journal where `StepStarted { step: N }` appears after `StepStarted { step: M }` where `N < M` produces `Err(ReplayDivergence)`.

**Strategy**: Generate step sequences with intentional ordering violations. Verify error is returned with correct step index.

### Invariant: Non-idempotent action blocking (ERR-NonIdempotentActionBlocked)
**Property**: Any journal where the same `(action, step)` pair is completed and then a subsequent `ActionScheduled` for that pair appears produces `Err(NonIdempotentActionBlocked)`.

**Strategy**: Generate journals where an action is completed then re-scheduled. Verify blocking error.

### Invariant: All events returned including stale (POST-004)
**Property**: For any input `events: Vec<JournalEvent>`, `replay_events(events.clone(), &mut tracker)` returns a vector of the same length with all original events preserved in original order.

**Strategy**: `proptest!` with arbitrary event sequences, verify output.len() == input.len() and output содержит all input events.

---

## Section 5 — Fuzz Targets

### Target: `replay_fuzz`
**Risk**: Malformed event sequences fed to `replay_events`
**Input type**: `&[JournalEvent]` encoded as a byte stream
**Corpus seeds**:
- Minimal: empty slice
- Single attempt: `[RunAccepted, StepStarted, ActionScheduled, ActionCompletedEvent, RunFinished]`
- Mixed attempt: `[RunAccepted, ActionScheduled(attempt=1), ActionCompletedEvent(attempt=1), ActionScheduled(attempt=2), ActionCompletedEvent(attempt=2)]`
- Out-of-order steps: `[StepStarted(step=2), StepStarted(step=1)]`
- Duplicate action: `[ActionScheduled(A,S), ActionCompletedEvent(A,S), ActionScheduled(A,S)]`

**What to test**:
- No panic on any input
- Returns `Ok` or `Err` but never panics
- Output length equals input length
- Tracker state is consistent with processed events

### Target: `extract_terminal_fuzz`
**Risk**: Malformed event slices to `extract_terminal`
**Input type**: `&[JournalEvent]`
**Corpus seeds**:
- Empty slice → `None`
- Single non-terminal → `None`
- Single terminal → `Some(&terminal)`
- Multiple terminals → last one by seq order

**What to test**:
- Never panics
- Returns `None` for non-terminal input
- Returns the correct terminal

### Target: `action_tracker_fuzz`
**Risk**: Adversarial tracker operations
**Input type**: Sequence of tracker operations (mark_completed, mark_failed, is_resolved)
**What to test**:
- `is_resolved` is consistent with mark operations
- No panic on HashSet collisions

---

## Section 6 — Kani Harnesses

### Harness: `replay_determinism`
**Property**: `replay_events` is deterministic for fixed input
**Bound**: Max 20 events, bounded attempt numbers (1..3), bounded step indices (0..5)
**Target**: `vb_storage/src/recovery/replay/core.rs::replay_events`
**Proof obligation**: INV-001b

### Harness: `stale_no_allocation`
**Property**: Stale events cannot allocate live timers, pending action tickets, or slot values
**Bound**: Max 15 events, 2 attempts, 3 steps
**Target**: `replay_events` with `RecoveryFrameSeed` construction
**Proof obligation**: INV-003

### Harness: `tracker_latest_only`
**Property**: `ActionReplayTracker` only records completed/failed actions from the latest attempt
**Bound**: Max 20 events, attempts 1..3, 5 steps
**Target**: `replay_events` with tracker inspection
**Proof obligation**: INV-004

### Harness: `stale_terminal_blocked`
**Property**: Stale `RunFinished`/`RunFailedEvent` from older attempt does not cause recovered run to appear finished when newer attempt shows in-progress or failed
**Bound**: Max 10 events, 2 attempts
**Target**: `extract_terminal` post-replay
**Proof obligation**: INV-005 / POST-005b

### Harness: `latest_attempt_state`
**Property**: Recovered run state reflects only the latest attempt's events
**Bound**: Max 15 events, 2 attempts, 4 steps
**Target**: Full `replay_events` → frame seed construction
**Proof obligation**: POST-001b

### Harness: `stale_excluded`
**Property**: Stale events excluded from live hydration
**Bound**: Max 12 events, 2 attempts
**Target**: Post-replay tracker + frame seed inspection
**Proof obligation**: POST-002b

### Harness: `event_ordering`
**Property**: Event ordering is deterministic via EventSeq
**Bound**: Max 10 events
**Target**: `replay_events` step ordering check
**Proof obligation**: PRE-002b

### Harness: `replay_divergence`
**Property**: Out-of-order step events are detected and return error
**Bound**: Max 8 events, 5 steps
**Target**: `replay_events` with injected out-of-order steps
**Proof obligation**: ERR-DIVERGENCEb

### Harness: `nonidempotent_blocked`
**Property**: Duplicate action from stale attempt is blocked
**Bound**: Max 10 events
**Target**: `replay_events` with duplicate action scheduling
**Proof obligation**: ERR-NONIDEMb

---

## Section 7 — Mutation Testing Checkpoints

| Checkpoint | Mutation Target | Mutant Behaviour | Kill Condition |
|------------|----------------|-------------------|----------------|
| MC-01 | `replay_events` — remove step ordering check | Out-of-order steps accepted | `test_replay_divergence_on_out_of_order_steps` fails |
| MC-02 | `replay_events` — remove `is_resolved` check in `ActionScheduled` | Duplicate action allowed | `test_stale_action_duplicate_is_blocked` fails |
| MC-03 | `replay_events` — remove `mark_completed` call | Tracker not updated | `test_action_tracker_blocks_non_idempotent_replay` fails |
| MC-04 | `replay_events` — remove `mark_failed` call | Tracker not updated for failed | `test_action_tracker_tracks_failed_actions` fails |
| MC-05 | `extract_terminal` — reverse iteration order | First terminal returned instead of last | `test_extract_terminal_finds_last_terminal` fails |
| MC-06 | `recover_snapshot_plus_tail` — flip `seq <=` to `seq <` | Event at exact snapshot seq accepted as tail | Boundary test fails |
| MC-07 | `replay_events` — filter out non-latest attempt events from output | Stale events dropped from output | `test_all_events_returned_including_stale` fails |
| MC-08 | `replay_events` — process stale events into tracker | Stale events pollute tracker | `test_tracker_only_records_latest_attempt_actions` fails |
| MC-09 | `load_snapshot` — treat `Ok(None)` as `Ok(snapshot)` instead of error | None returned as valid snapshot | `test_snapshot_decode_none_returns_corrupt_snapshot` fails |
| MC-10 | `recover_full_journal` — skip empty check | Empty journal returns `Ok([])` instead of error | `test_full_journal_recovery_with_no_data_fails` fails |

**Target kill rate**: ≥ 90%

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Happy path: single attempt | `events = [accepted, started, succeeded, finished]` | `Ok([all events])`, tracker populated | unit |
| Happy path: latest attempt wins | `events = [attempt1 events, attempt2 events]` | Only attempt2 in tracker | unit + proptest |
| Stale events preserved | Any mixed-attempt input | `output.len() == input.len()` | unit |
| Out-of-order step divergence | `StepStarted(2) → StepStarted(1)` | `Err(ReplayDivergence)` | unit |
| Duplicate action blocked | `Scheduled(A,S), Completed(A,S), Scheduled(A,S)` | `Err(NonIdempotentActionBlocked)` | unit |
| Empty journal | `events = []` | `Err(NoRecoveryData)` | integration |
| Empty replay | `events = []` | `Ok([])` | unit |
| Snapshot seq boundary | `snapshot.seq = N`, `tail seq = N` | `Err(ReplayDivergence)` | unit |
| Snapshot valid tail | `snapshot.seq = N`, `tail seq > N` | `Ok(tail events)` | unit |
| Terminal: last wins | `RunFinished(seq=5), RunFailed(seq=10)` | `Some(&RunFailed)` | unit |
| Terminal: none | `events = [accepted, started]` | `None` | unit |
| Tracker: completed | `ActionCompletedEvent` processed | `is_resolved == true` | unit |
| Tracker: failed | `ActionFailedEvent` processed | `is_resolved == true` | unit |
| Error: Journal failure | `events_for_run` returns Err | `Err(Journal(...))` | integration |
| Error: CorruptSnapshot | `snapshot` decode returns `None` | `Err(CorruptSnapshot)` | unit |
| Attempt number extraction | Events with attempt numbers | Max correctly identified | proptest |
| Determinism | Same events twice | Identical output | proptest |
| Stale no allocation | Mixed-attempt journal | Stale events excluded from tracker | kani + proptest |
| INV-001 determinism | Fixed event sequence | Identical RecoveryFrameSeed | kani |
| INV-005 stale terminal | Stale RunFinished + newer RunFailed | RunFailed wins | proptest + kani |

---

## Section 9 — Proof Obligations Addressed

| Obligation ID | Target | Claim | Test Strategy | Status |
|--------------|--------|-------|---------------|--------|
| INV-001 | `replay_events` | Deterministic replay | `proptest!` with identical input → identical output | Planned |
| INV-001b | `replay_events` | Kani bounded model check | `kani` harness `replay_determinism` | Planned |
| INV-002 | `core.rs` | Attempt selection independent of wall clock | Lean theorem `latest_attempt_deterministic` | Planned |
| INV-003 | `replay_events` | Stale no live allocation | `kani` harness `stale_no_allocation` + `proptest` | Planned |
| INV-003b | `replay_events` | Stale no allocation proptest | `proptest!` adversarial interleaving | Planned |
| INV-004 | `replay_events` | Tracker only latest attempt | `kani` harness `tracker_latest_only` + `proptest` | Planned |
| INV-004b | `replay_events` | Tracker latest only proptest | `proptest!` mixed-attempt journal | Planned |
| INV-005 | `extract_terminal` | Stale terminal blocked | `proptest!` + `kani` harness `stale_terminal_blocked` | Planned |
| INV-005b | `extract_terminal` | Stale terminal blocked kani | `kani` harness `stale_terminal_blocked` | Planned |
| PRE-001 | `core.rs` | Attempt numbers extracted | `proptest!` + `cargo-fuzz replay_fuzz` | Planned |
| PRE-001b | `core.rs` | Attempt numbers fuzz | `cargo-fuzz` malformed sequences | Planned |
| PRE-002 | `events_for_run` | Deterministic EventSeq order | Proptest (existing journal tests) | Covered |
| PRE-002b | `core.rs` | Event ordering Kani | `kani` harness `event_ordering` | Planned |
| PRE-003 | `events_for_run` | Consistent ordered slice | **WAIVER** — covered by existing 40+ journal.rs tests | Waived |
| POST-001 | `replay_events` | Latest attempt state only | `proptest!` + `kani` harness `latest_attempt_state` | Planned |
| POST-001b | `replay_events` | Latest attempt state Kani | `kani` harness `latest_attempt_state` | Planned |
| POST-002 | `replay_events` | Stale excluded from tracker/seed | `proptest!` + `kani` harness `stale_excluded` | Planned |
| POST-002b | `replay_events` | Stale excluded Kani | `kani` harness `stale_excluded` | Planned |
| POST-003 | `core.rs` | Max attempt wins | Lean theorem + proptest | Planned |
| POST-003b | `core.rs` | Max attempt empirical | `proptest!` | Planned |
| POST-004 | `replay_events` | All events returned including stale | `proptest!` | Planned |
| POST-005 | `extract_terminal` | Stale RunFinished does not win | `proptest!` + `kani` | Planned |
| POST-005b | `extract_terminal` | Stale terminal kani | `kani` harness `stale_terminal_blocked` | Planned |
| ERR-DIVERGENCE | `replay_events` | Out-of-order detected | `proptest!` + `kani` harness `replay_divergence` | Planned |
| ERR-DIVERGENCEb | `replay_events` | Kani divergence check | `kani` harness `replay_divergence` | Planned |
| ERR-NONIDEM | `replay_events` | Duplicate blocked | `proptest!` + `kani` harness `nonidempotent_blocked` | Planned |
| ERR-NONIDEMb | `replay_events` | Kani nonidempotent check | `kani` harness `nonidempotent_blocked` | Planned |
| GATE-001 | workspace | All verification layers pass | `moon run :verify-all` | Planned |

---

## Section 10 — Test Execution Order

```
# Fast gate (unit + proptest)
moon run :test --package vb_storage

# Standard gate (+ Kani)
moon run :verify-standard

# Deep gate (+ Miri on pure crates)
moon run :verify-deep

# Proof gate (+ Lean)
moon run :verify-proof

# Full gauntlet
moon run :verify-all
```

---

## Section 11 — Existing Tests Reference

The following existing tests in `crates/vb_storage/src/recovery/tests.rs` cover baseline replay behavior and should be preserved and extended:

- `action_tracker_blocks_non_idempotent_replay` — ERR-NonIdempotentActionBlocked
- `action_tracker_allows_first_execution` — Happy path tracker
- `action_tracker_tracks_failed_actions` — Tracker for failed
- `snapshot_tail_matches_full_journal_lifecycle_summary` — Snapshot+tail integration
- `snapshot_tail_matches_full_journal_action_summary` — Action tracking in snapshot+tail
- `snapshot_plus_tail_rejects_event_before_snapshot` — ERR-ReplayDivergence for seq boundary
- `replay_detects_out_of_order_step` — ERR-ReplayDivergence
- `full_journal_recovery_with_no_data_fails` — ERR-NoRecoveryData
- `replay_events_produces_correct_final_state_from_empty` — Empty replay
- `extract_terminal_finds_last_terminal` — extract_terminal semantics
- `is_terminal_event_identifies_terminals` — is_terminal_event coverage

**vb-h6ix adds**: latest-attempt filtering, stale event exclusion from tracker, stale terminal override prevention, and max-attempt selection correctness.

---

*Plan generated: 2026-05-09*
*Next state: Implementation (State 2)*

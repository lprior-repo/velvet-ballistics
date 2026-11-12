# Test Plan: vb-qi37.1.6 — Runtime Recovery Crash Restart Integration Evidence

## Summary

- Bead: `vb-qi37.1.6`
- Feature: durable runtime recovery — crash restart from persisted run headers, journal events, snapshots, waits, asks, actions, and collect pagination state.
- Behaviors identified: 9 primary + 9 error variants
- Trophy allocation: ~25 unit / ~35 integration / ~2 e2e / ~5 static
- Proptest invariants: 4
- Fuzz targets: 2
- Kani harnesses: 0 (waived PO-003)
- Mutation checkpoints: 9 error variants + PRE-006 boundary

---

## 1. Behavior Inventory

### B-001: Persisted Header Bind
"Recovery reconstructs the target run identity from persisted header and admission digests when restart begins."

### B-002: Full-Journal Replay Exactness
"Full-journal replay reconstructs pc, step states, executed counts, slot values, slot taint, terminal status, and latest-attempt state exactly from durable events."

### B-003: Snapshot-plus-Tail Monotonicity
"Snapshot-plus-tail replay preserves snapshot facts and applies only tail events after the snapshot watermark, with no tail-before-snapshot or sequence-gap acceptance."

### B-004: Wait State Continuity
"Wait recovery preserves the waiting state and resumes only from the durable wait event identity."

### B-005: Ask State and Answer Taint Continuity
"Ask recovery preserves the asking state, answer slot value, and answer taint across restart."

### B-006: Action Ticket Identity — No Duplicate Execution
"Resolved action tickets are not re-executed; non-idempotent or unsupported pending actions fail closed."

### B-007: Collect Pagination Cursor and Extra Survival
"Collect pagination cursor, current page, ordering, and identity recover from durable `SlotWrittenEvent.extra`; corrupt or wrong-identity extra returns a typed error."

### B-008: No Empty Success Frame for Non-Empty Run
"Restart never fabricates an empty successful frame for a non-empty run."

### B-009: Invariant-Driven Idempotent Replay
"Replaying the same persisted journal and snapshot twice yields equivalent recovery summaries and frame seeds; latest-attempt filtering never mixes stale state."

### B-010: Digest Mismatch Typed Rejection
"Workflow source digest mismatch or compiled IR digest mismatch returns a precise typed failure."

### B-011: Snapshot Dimension Overflow Typed Rejection
"Recovered dimensions exceeding representable or configured frame bounds return `FrameDimensionOverflow`."

### B-012: Corrupt Snapshot Typed Rejection
"Corrupt snapshot bytes, run id, dimensions, slot payload, or snapshot metadata return `CorruptSnapshot`."

### B-013: Replay Divergence Typed Rejection
"Event order violation, step state mismatch, latest-attempt violation, sequence gap, or tail watermark rule violation returns `ReplayDivergence`."

### B-014: No Recovery Data Typed Rejection
"Persisted header exists but no usable recovery events or snapshot facts exist return `NoRecoveryData`."

### B-015: Non-Idempotent Action Blocked Typed Rejection
"Pending action that cannot be replayed without duplicate effects returns `NonIdempotentActionBlocked`."

### B-016: Unsupported Recovery State Typed Rejection
"Unsupported or incomplete runtime recovery state that cannot be hydrated returns `InvalidRecoveryHydration` at the runtime boundary."

### B-017: Corrupt Collect Extra Typed Rejection
"Collect extra that is missing, corrupt, or bound to the wrong collect identity returns `CollectExtraHydrationFailed`."

### B-018: Taint Exactness Preservation
"A recovered secret slot remains secret; missing durable taint evidence cannot silently default a required secret fact to clean."

### B-019: Fail-Closed Unsupported State
"Unsupported pending actions, missing event sets, corrupt encodings, digest mismatch, and unsupported variants cannot produce runnable state."

### B-020: Unsequenced Lifecycle Diagnostics Non-Authority
"Unsequenced lifecycle diagnostics (RunResumed, RunRetried, RunAnswered) do not alter recovered state."

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Static Analysis** | 5 | clippy (vb_storage, vb_runtime), cargo-deny, rustfmt, miri NA gate. All recovery code must pass lint/safety gates before integration runs. |
| **Unit / Calc** | 25 | 9 typed error constructors, 4 pure replay functions, 3 digest-check functions, 4 summary builders, 3 collect hydration helpers, 2 taint-derivation paths. Exhaustive error-variant coverage per constructor. |
| **Integration** | 35 | Fjall drop/reopen (INT-REC-001/002/003), full-journal round-trip, snapshot+tail, wait/ask/action/resume, collect pagination. Real I/O, no mocks. |
| **E2E** | 2 | `moon_run_test_after_bead_changes` (GA-001 gate), CLI-recovery-or-crate-boundary smoke. |
| **Proptest** | 4 invariants | Deterministic replay, snapshot-tail monotonicity, taint no-downgrade, collect identity preservation. |
| **Mutation** | 9+1 | One mutation class per typed error variant + PRE-006 fallible-boundary branch. |

**Ratio**: ~60% integration, ~37% unit/static, ~3% e2e. Aligned with testing trophy.

---

## 3. BDD Scenarios

### Behavior: B-001 — Persisted Header Bind

**Scenario: GA-001a — Full header with matching digests**
```
Given: a recoverable run has a persisted run header with matching run id,
       workflow source digest, compiled IR digest, and accepted artifact digest
When:  recovery begins with expected_digests matching the persisted header
Then:  recovery summary bind confirms target run identity
And:   no empty successful frame is returned for the non-empty run
```

**Scenario: GA-001b — Workflow source digest mismatch**
```
Given: a persisted run header with run id X but workflow source digest D_wrong
When:  recovery is invoked with expected_digests.source_digest != D_wrong
Then:  RecoveryError::WorkflowSourceDigestMismatch is returned
```

**Scenario: GA-001c — Compiled IR digest mismatch**
```
Given: a persisted run header with run id X but compiled artifact digest D_wrong
When:  recovery is invoked with expected_digests.compiled_digest != D_wrong
Then:  RecoveryError::CompiledIrDigestMismatch is returned
```

**Rust test name**: `fn header_binds_target_run_when_digests_match()` / `fn header_rejects_workflow_source_digest_mismatch()` / `fn header_rejects_compiled_ir_digest_mismatch()`

---

### Behavior: B-002 — Full-Journal Replay Exactness

**Scenario: GA-002a — Full journal after crash reconstructs exact state**
```
Given: a run has persisted journal events covering step starts, step successes,
       slot writes with taint, a terminal event, and a latest attempt marker
When:  recover_full_journal is called with the complete event slice
Then:  the resulting RecoveryHydration contains pc, step states, executed counts,
       slot values, slot taint, and terminal status exactly matching the events
```

**Scenario: GA-002b — Full journal replay rejects sequence gap**
```
Given: a journal event sequence with a missing sequence number between events
When:  recover_full_journal processes the events
Then:  RecoveryError::ReplayDivergence is returned
```

**Rust test name**: `fn full_journal_reconstructs_exact_pc_steps_slots_taint_terminal()` / `fn full_journal_rejects_sequence_gap()`

---

### Behavior: B-003 — Snapshot-plus-Tail Monotonicity

**Scenario: GA-003a — Snapshot plus tail applies after watermark**
```
Given: a run has a persisted snapshot with sequence watermark W and
       tail events with sequence numbers strictly greater than W
When:  recover_snapshot_plus_tail is called with the snapshot and tail
Then:  snapshot facts are preserved and tail events are applied in order
And:  no fact from the tail overwrites a snapshot fact without an ordered replacement
```

**Scenario: GA-003b — Tail before snapshot is rejected**
```
Given: tail events whose sequence numbers are not strictly greater than the snapshot watermark
When:  recover_snapshot_plus_tail is called
Then:  RecoveryError::ReplayDivergence is returned
```

**Scenario: GA-003c — Same snapshot and tail replays equivalently twice**
```
Given: an idempotent snapshot and tail event set
When:  recover_snapshot_plus_tail is called twice with identical inputs
Then:  both calls return equivalent RecoveryHydration summaries and frame seeds
```

**Rust test name**: `fn snapshot_plus_tail_applies_tail_after_watermark()` / `fn snapshot_plus_tail_rejects_tail_before_snapshot()` / `fn snapshot_plus_tail_idempotent_on_same_input()`

---

### Behavior: B-004 — Wait State Continuity

**Scenario: GA-004a — Waiting run resumes from durable wait identity**
```
Given: a run with a durable wait event with identity W_id and waiting state S
When:  recovery replays the journal after reopen
Then:  the recovered frame preserves wait identity W_id and waiting state S
And:  no in-memory wait state is used
```

**Rust test name**: `fn wait_identity_and_state_survive_across_restart()`

---

### Behavior: B-005 — Ask State and Answer Taint Continuity

**Scenario: GA-005a — Asking run and answer event preserve answer slot and taint**
```
Given: a run with a durable ask event and an answer slot write with taint T
When:  recovery replays the journal after reopen
Then:  the recovered frame preserves the asking state, answer slot value, and taint T
```

**Rust test name**: `fn ask_answer_slot_value_and_taint_survive_across_restart()`

---

### Behavior: B-006 — Action Ticket No Duplicate Execution

**Scenario: GA-006a — Resolved action ticket is not re-executed**
```
Given: a run with a durable ActionResolved event for ticket T with result R
When:  recovery replays the journal after reopen
Then:  ticket T is marked resolved with result R
And:  no re-execution of T's side effects occurs
```

**Scenario: GA-006b — Non-idempotent pending action fails closed**
```
Given: a run with a durable ActionScheduled event for a non-idempotent pending action
When:  recovery attempts to hydrate the frame
Then:  RecoveryError::NonIdempotentActionBlocked is returned
And:  no partial successful frame is produced
```

**Rust test name**: `fn resolved_action_not_reexecuted_on_restart()` / `fn non_idempotent_pending_action_fails_closed()`

---

### Behavior: B-007 — Collect Pagination Cursor and Extra Survival

**Scenario: GA-007a — Mid-collect pagination state survives restart**
```
Given: a run at collect pagination point P with cursor C, page number N,
       ordering O, and durable SlotWrittenEvent.extra carrying the collect identity
When:  hydrate_collect_state is called with the events and collect identity
Then:  the returned CollectStates has cursor C, page N, ordering O, and identity matches
```

**Scenario: GA-007b — Corrupt collect extra returns typed error**
```
Given: a SlotWrittenEvent.extra that is corrupt or does not bind to the expected collect identity
When:  hydrate_collect_state is called
Then:  EngineError::CollectExtraHydrationFailed is returned
```

**Scenario: GA-007c — Wrong collect identity extra returns typed error**
```
Given: a SlotWrittenEvent.extra that binds to a different collect identity than requested
When:  hydrate_collect_state is called
Then:  EngineError::CollectExtraHydrationFailed is returned
```

**Rust test name**: `fn collect_cursor_page_order_survive_restart()` / `fn corrupt_collect_extra_returns_typed_error()` / `fn wrong_collect_identity_returns_typed_error()`

---

### Behavior: B-008 — No Empty Success Frame for Non-Empty Run

**Scenario: GA-008a — Non-empty run with header only returns typed error**
```
Given: a persisted run header with no recovery events and no snapshot
When:  recover_runtime_frame_seed is called
Then:  RecoveryError::NoRecoveryData is returned
And:  no empty successful RunFrame is produced
```

**Rust test name**: `fn non_empty_run_with_header_only_returns_no_recovery_data()`

---

### Behavior: B-009 — Invariant-Driven Idempotent Replay

**Scenario: GA-009a — Same journal and snapshot replays equivalently twice**
```
Given: a deterministic set of journal events and a snapshot
When:  replay is executed twice with identical inputs
Then:  both RecoveryHydration outputs are equivalent
And:  both RecoveryFrameSeed outputs are equivalent
```

**Scenario: GA-009b — Stale attempt terminal state is not mixed into active attempt**
```
Given: journal events with attempt N terminal state and attempt N+1 active state
When:  recovery replays the journal
Then:  only attempt N+1 state appears in the recovery output
And:  attempt N terminal state is not mixed in
```

**Rust test name**: `fn same_journal_and_snapshot_replayed_twice_equivalent()` / `fn stale_attempt_state_not_mixed_into_active_attempt()`

---

### Behavior: B-010 — Digest Mismatch Typed Rejection

**Scenario: GA-010a — Workflow source digest mismatch**
```
Given: persisted header with source digest D1, recovery called with expected digest D2 != D1
When:  recovery boundary check runs
Then:  RecoveryError::WorkflowSourceDigestMismatch is returned
```

**Scenario: GA-010b — Compiled IR digest mismatch**
```
Given: persisted header with compiled digest D1, recovery called with expected digest D2 != D1
When:  recovery boundary check runs
Then:  RecoveryError::CompiledIrDigestMismatch is returned
```

**Rust test name**: `fn workflow_source_digest_mismatch_returns_typed_error()` / `fn compiled_ir_digest_mismatch_returns_typed_error()`

---

### Behavior: B-011 — Snapshot Dimension Overflow Typed Rejection

**Scenario: GA-011a — Frame dimension overflow returns typed error**
```
Given: a recovered frame whose dimensions exceed configured or representable bounds
When:  hydrate_run_frame processes the RecoveryHydration
Then:  RecoveryError::FrameDimensionOverflow is returned
```

**Rust test name**: `fn frame_dimension_overflow_returns_typed_error()`

---

### Behavior: B-012 — Corrupt Snapshot Typed Rejection

**Scenario: GA-012a — Corrupt snapshot returns CorruptSnapshot error**
```
Given: a persisted snapshot whose bytes, run id, dimensions, slot payload, or metadata are corrupt
When:  recover_snapshot_plus_tail is called
Then:  RecoveryError::CorruptSnapshot is returned
```

**Rust test name**: `fn corrupt_snapshot_returns_corrupt_snapshot_error()`

---

### Behavior: B-013 — Replay Divergence Typed Rejection

**Scenario: GA-013a — Sequence gap returns ReplayDivergence**
```
Given: journal events with a non-consecutive sequence number
When:  recover_full_journal is called
Then:  RecoveryError::ReplayDivergence is returned
```

**Scenario: GA-013b — Tail before snapshot returns ReplayDivergence**
```
Given: tail events with sequence numbers that do not strictly follow the snapshot watermark
When:  recover_snapshot_plus_tail is called
Then:  RecoveryError::ReplayDivergence is returned
```

**Rust test name**: `fn sequence_gap_returns_replay_divergence()` / `fn tail_before_snapshot_returns_replay_divergence()`

---

### Behavior: B-014 — No Recovery Data Typed Rejection

**Scenario: GA-014a — Header without recovery events returns NoRecoveryData**
```
Given: a persisted run header with no usable recovery events and no snapshot
When:  recover_runtime_frame_seed is called
Then:  RecoveryError::NoRecoveryData is returned
```

**Rust test name**: `fn header_without_recovery_events_returns_no_recovery_data()`

---

### Behavior: B-015 — Non-Idempotent Action Blocked Typed Rejection

**Scenario: GA-015a — Non-idempotent pending action blocked returns typed error**
```
Given: a pending non-idempotent action in the durable journal
When:  recovery attempts to hydrate the pending action
Then:  RecoveryError::NonIdempotentActionBlocked is returned
And:  no partial successful frame is produced
```

**Rust test name**: `fn non_idempotent_pending_action_returns_non_idempotent_action_blocked()`

---

### Behavior: B-016 — Unsupported Recovery State Typed Rejection (PRE-006)

**Scenario: GA-016a — Unsupported recovery state returns InvalidRecoveryHydration**
```
Given: a recovery hydration that is incomplete or unsupported for a runnable RunFrame
When:  recovery_boundary_from_hydration is called
Then:  RuntimeError::InvalidRecoveryHydration is returned
And:  no partial successful frame is produced
```

**Scenario: GA-016b — Unsupported state at runtime boundary fails closed**
```
Given: a RecoveryHydration with a live-frame component that is unsupported
When:  hydrate_run_frame is called at the runtime boundary
Then:  the result is Err(InvalidRecoveryHydration)
And:  the caller consumes the Result rather than constructing a partial frame
```

**Rust test name**: `fn unsupported_recovery_state_returns_invalid_recovery_hydration()` / `fn unsupported_live_frame_component_fails_closed_at_boundary()`

---

### Behavior: B-017 — Corrupt Collect Extra Typed Rejection

**Scenario: GA-017a — Corrupt collect extra returns CollectExtraHydrationFailed**
```
Given: a SlotWrittenEvent.extra that is corrupt
When:  hydrate_collect_state processes the extra
Then:  EngineError::CollectExtraHydrationFailed is returned
```

**Rust test name**: `fn corrupt_collect_extra_returns_collect_extra_hydration_failed()`

---

### Behavior: B-018 — Taint Exactness Preservation

**Scenario: GA-018a — Secret slot taint is preserved across restart**
```
Given: a durable slot value with secret taint T_secret in the journal
When:  recovery replays the slot write event
Then:  the recovered slot has taint T_secret
And:  no missing-taint evidence silently downgrades the slot to clean
```

**Scenario: GA-018b — Missing taint evidence fails closed**
```
Given: a durable slot value whose taint metadata is absent or corrupt
When:  recovery replays the slot write event
Then:  recovery fails with a typed error
And:  the slot is not silently defaulted to clean
```

**Rust test name**: `fn secret_slot_taint_preserved_across_restart()` / `fn missing_taint_evidence_fails_closed()`

---

### Behavior: B-019 — Fail-Closed Unsupported State

**Scenario: GA-019a — Unsupported pending actions cannot produce runnable state**
```
Given: a recovery input containing an unsupported live-frame state variant
When:  hydrate_run_frame is called
Then:  a typed error is returned
And:  no runnable frame is produced
```

**Rust test name**: `fn unsupported_live_frame_state_cannot_produce_runnable_frame()`

---

### Behavior: B-020 — Unsequenced Lifecycle Diagnostics Non-Authority

**Scenario: GA-020a — Unsequenced lifecycle events do not change recovered state**
```
Given: RunResumed, RunRetried, or RunAnswered events appear in the journal
       without ordered journal sequence semantics
When:  recovery replays the journal
Then:  the recovered frame state is unchanged by these events
And:  they are treated as diagnostic, not authoritative
```

**Rust test name**: `fn unsequenced_lifecycle_events_do_not_change_recovered_state()`

---

## 4. Proptest Invariants

### PPI-001: Deterministic Replay Invariant
**Function**: `recover_full_journal`, `recover_snapshot_plus_tail`
**Property**: Replaying the same event slice twice produces bit-equivalent `RecoveryHydration` outputs.
**Strategy**: `journal_events_seq(any::<JournalEvent>())` where generator produces non-gapping sequences. Anti-invariant: sequence gaps must return `Err(ReplayDivergence)`.

### PPI-002: Snapshot-Tail Monotonicity Invariant
**Function**: `recover_snapshot_plus_tail`
**Property**: For all tail event slices T and snapshot S, applying T after S's watermark never erases any durable slot, taint, wait, ask, action, or collect fact without an ordered replacement event in T.
**Strategy**: `prop_compose_snapshot_tail()` generating snapshot + tail where tail events may overwrite specific slots. Anti-invariant: tail-before-watermark returns error.

### PPI-003: Taint No-Downgrade Invariant
**Function**: `decode_snapshot_slots`, `apply_tail_events`
**Property**: Any durable slot value that was secret (taint=T) before restart remains secret after recovery; no fallback to clean taint is permitted.
**Strategy**: `secret_slot_value_with_JournalEvent()` generator. Anti-invariant: secret→clean downgrade returns counterexample.

### PPI-004: Collect Identity Preservation Invariant
**Function**: `hydrate_collect_state`
**Property**: For all `JournalEvent::SlotWrittenEvent` with valid collect extra, `hydrate_collect_state` returns `Ok(CollectStates)` with identity matching the input identity.
**Strategy**: `collect_slot_written_event_with_extra()` generator producing valid extra bytes via postcard. Anti-invariant: wrong-identity extra → `Err(CollectExtraHydrationFailed)`.

---

## 5. Fuzz Targets

### FT-001: Journal Event Deserialization
- **Target**: `JournalEvent::try_decode` / `JournalEvent::try_from_slice` boundary
- **Risk**: Panic, OOM on malformed bytes, wrong variant construction, sequence number corruption
- **Corpus seeds**: Valid `SlotWrittenEvent`, `WaitScheduledEvent`, `AskScheduledEvent`, `ActionScheduled`, `RunAdmission`, `StepStarted`, `StepSucceeded`, terminal events with valid and boundary sequence numbers
- **Mutations**: Truncate bytes, swap variant discriminant, overflow sequence number, corrupt taint byte, zero-valid extra

### FT-002: SlotWrittenEvent Extra Deserialization
- **Target**: `hydrate_extra` / `capture_extra` boundary for collect pagination
- **Risk**: Panic on corrupt postcard bytes, wrong collect identity construction, cursor/page corruption
- **Corpus seeds**: Valid `CollectPaginationState` encoded via postcard, boundary page/cursor values, max ordering values
- **Mutations**: Truncated postcard bytes, wrong collect identity, page number overflow, cursor corruption

---

## 6. Kani Verification Harnesses

**Status**: 0 active harnesses. `KANI-REC-001 / PO-003` was explicitly waived in State 5 with compensating `VERUS-REC-001` evidence (10 verified, 0 errors in `verification/verus/recovery_hydration_contracts.rs`). Waiver accepted per `proof-obligations.planned.jsonl` PO-003 row.

If the waiver is rejected in a future State 6 re-review, the following harness is required:

### KHI-001: Bounded Dimension and Error Totality (CONDITIONAL)
- **Property**: `hydrate_run_frame` returns `Ok` only when dimensions are within bounds and all required durable facts are present; returns `Err` with exact variant for all out-of-bounds or missing-fact cases.
- **Bound**: 4 steps × 4 slots × 2 attempts × 1 snapshot watermark
- **Rationale**: Arithmetic overflow and exhaustive error-variant coverage cannot be fully guaranteed by integration testing alone.

---

## 7. Mutation Testing Checkpoints

**Threshold**: ≥90% mutation kill rate for scoped recovery paths.

### MCP-001: Typed Error Constructors
Each of the 9 typed error constructors must be reachable by at least one integration test that asserts the exact variant:

| Error Constructor | Must Be Caught By |
|---|---|
| `RecoveryError::NoRecoveryData` | `fn header_without_recovery_events_returns_no_recovery_data()` |
| `RecoveryError::CorruptSnapshot` | `fn corrupt_snapshot_returns_corrupt_snapshot_error()` |
| `RecoveryError::ReplayDivergence` | `fn sequence_gap_returns_replay_divergence()`, `fn tail_before_snapshot_returns_replay_divergence()` |
| `RecoveryError::WorkflowSourceDigestMismatch` | `fn workflow_source_digest_mismatch_returns_typed_error()` |
| `RecoveryError::CompiledIrDigestMismatch` | `fn compiled_ir_digest_mismatch_returns_typed_error()` |
| `RecoveryError::NonIdempotentActionBlocked` | `fn non_idempotent_pending_action_returns_non_idempotent_action_blocked()` |
| `RecoveryError::FrameDimensionOverflow` | `fn frame_dimension_overflow_returns_typed_error()` |
| `RuntimeError::InvalidRecoveryHydration` | `fn unsupported_recovery_state_returns_invalid_recovery_hydration()` |
| `EngineError::CollectExtraHydrationFailed` | `fn corrupt_collect_extra_returns_collect_extra_hydration_failed()` |

### MCP-002: PRE-006 Fallible Boundary
Mutation of the fallible `Result` return path at `recovery_boundary_from_hydration` and `hydrate_run_frame` must be caught by at least one test that asserts `Err(InvalidRecoveryHydration)` for unsupported state — not just `is_err()`.

### MCP-003: Action Ticket No-Reexecution
Mutation of the resolved-action deduplication branch must be caught by the `resolved_action_not_reexecuted_on_restart` scenario.

### MCP-004: Taint No-Downgrade
Mutation of taint derivation that would silently downgrade secret→clean must be caught by the `secret_slot_taint_preserved_across_restart` scenario.

---

## 8. Combinatorial Coverage Matrix

### CM-001: Recovery Entry Points

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `recover_runtime_summary` — header+events | valid header + events | `Ok(RecoveryRuntimeSummary)` | unit |
| `recover_runtime_summary` — digest mismatch | wrong source digest | `Err(WorkflowSourceDigestMismatch)` | unit |
| `recover_runtime_summary` — no data | header only | `Err(NoRecoveryData)` | unit |
| `recover_runtime_frame_seed` — full journal | full event set | `Ok(RecoveryFrameSeed)` | integration |
| `recover_runtime_frame_seed` — snapshot+tail | snapshot + tail | `Ok(RecoveryFrameSeed)` | integration |
| `recover_runtime_frame_seed` — corrupt snapshot | corrupt bytes | `Err(CorruptSnapshot)` | unit |
| `recover_runtime_frame_seed` — seq gap | gapped events | `Err(ReplayDivergence)` | unit |
| `hydrate_run_frame` — valid hydration | valid RecoveryHydration | `Ok(RunFrame)` | unit |
| `hydrate_run_frame` — dimension overflow | oversized dims | `Err(FrameDimensionOverflow)` | unit |
| `hydrate_run_frame` — unsupported state | unsupported variant | `Err(InvalidRecoveryHydration)` | unit |
| `recovery_boundary_from_hydration` — valid | valid RecoveryHydration | `Ok(RuntimeRecoveryBoundary)` | unit |
| `recovery_boundary_from_hydration` — PRE-006 fail | unsupported hydration | `Err(InvalidRecoveryHydration)` | unit |
| `hydrate_collect_state` — valid extra | valid CollectIdentity + extra | `Ok(CollectStates)` | unit |
| `hydrate_collect_state` — corrupt extra | corrupt extra bytes | `Err(CollectExtraHydrationFailed)` | unit |
| `hydrate_collect_state` — wrong identity | extra binds to different id | `Err(CollectExtraHydrationFailed)` | unit |

### CM-002: Journal Replay Functions

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `recover_full_journal` — in-order events | valid sequential events | `Ok(RecoveryHydration)` | unit |
| `recover_full_journal` — seq gap | missing sequence N | `Err(ReplayDivergence)` | unit |
| `recover_full_journal` — stale attempt | mixed attempt events | Latest-attempt state only | unit |
| `recover_full_journal` — idempotent twice | same events × 2 | Equivalent output | unit+proptest |
| `recover_snapshot_plus_tail` — valid | snapshot + tail > watermark | `Ok(RecoveryHydration)` | unit |
| `recover_snapshot_plus_tail` — tail before watermark | tail ≤ watermark | `Err(ReplayDivergence)` | unit |
| `recover_snapshot_plus_tail` — monotonic | tail overwrites slot | Slot replaced, not erased | unit+proptest |
| `recover_snapshot_plus_tail` — idempotent | same snapshot+tail × 2 | Equivalent output | proptest |

### CM-003: Taint and Slot Recovery

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Slot with secret taint in snapshot | secret slot write event | Recovered slot has secret taint | unit |
| Slot with clean taint in snapshot | clean slot write event | Recovered slot has clean taint | unit |
| Missing taint metadata | slot write without taint | Fails closed | unit |
| Slot overwritten in tail | snapshot slot + tail slot write | Tail value with preserved taint | unit |
| Secret slot in full journal replay | secret slot in event sequence | Recovered taint matches event | unit |

### CM-004: Action and Collect Recovery

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Resolved action ticket | ActionResolved durable event | Ticket marked resolved, no re-execution | integration |
| Pending idempotent action | ActionScheduled idempotent | Action preserved in frame | integration |
| Pending non-idempotent action | ActionScheduled non-idempotent | `Err(NonIdempotentActionBlocked)` | integration |
| Collect valid extra | valid `SlotWrittenEvent.extra` | `Ok(CollectStates)` with correct cursor | integration |
| Collect corrupt extra | corrupt postcard bytes | `Err(CollectExtraHydrationFailed)` | integration |
| Collect wrong identity | extra binds to different id | `Err(CollectExtraHydrationFailed)` | integration |

---

## 9. Static Analysis Gates

| Gate | Command | Target | Pass Criteria |
|------|---------|--------|---------------|
| clippy | `cargo clippy -p vb_storage -p vb_runtime --all-features -- -D warnings` | `crates/vb_storage/src/recovery/`, `crates/vb_runtime/src/recovery.rs`, `crates/vb_runtime/src/primitives/collect.rs` | exit 0 |
| cargo-deny | `cargo deny check` | `Cargo.lock`, `deny.toml` | no recovery-relevant advisories |
| rustfmt | `cargo fmt --check` | all modified source | exit 0 |
| miri NA | `cargo miri test -p vb_storage -- recovery` | recovery paths | NA gate — no unsafe in scope |
| test compile | `cargo test --no-run -p vb_storage -p vb_runtime --all-features` | recovery tests | compiles without errors |

---

## 10. Traceability Mapping

Each BDD scenario maps to contract clauses via `traceability-matrix.jsonl` rows:

| BDD Scenario | Contract Clause | Proof Obligation | Test Layer |
|---|---|---|---|
| GA-001a/b/c | PRE-001 | INT-REC-001 | unit+integration |
| GA-002a/b | PRE-002, POST-002, INV-002 | TLA-REC-001, INT-REC-001, PROP-REC-001 | unit+integration+proptest |
| GA-003a/b/c | PRE-003, POST-003, INV-004 | TLA-REC-001, INT-REC-001, PROP-REC-001 | unit+integration+proptest |
| GA-004a | POST-004 | TLA-REC-001, INT-REC-002 | integration |
| GA-005a | POST-005 | TLA-REC-001, INT-REC-002 | integration |
| GA-006a/b | POST-006 | TLA-REC-001, INT-REC-002 | integration |
| GA-007a/b/c | POST-007 | TLA-REC-001, INT-REC-003 | unit+integration |
| GA-008a | POST-001 | VERUS-REC-001, INT-REC-001 | unit |
| GA-009a/b | INV-002, INV-003 | TLA-REC-001, PROP-REC-001 | unit+proptest |
| GA-010a/b | POST-008 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-011a | POST-008 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-012a | POST-008 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-013a/b | POST-008, INV-007 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-014a | POST-008 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-015a | POST-008, POST-006 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-016a/b | POST-008, PRE-006 | VERUS-REC-001, INT-REC-002, MUT-REC-001 | unit+integration+mutation |
| GA-017a | POST-008 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-018a/b | INV-005 | VERUS-REC-001, PROP-REC-001 | unit+proptest |
| GA-019a | INV-006 | VERUS-REC-001, MUT-REC-001 | unit+mutation |
| GA-020a | INV-007 | TLA-REC-001 | unit+integration |

---

## Open Questions

1. **CLI restart path**: `delivery-scope.jsonl` marks public CLI restart command as `UNKNOWN`. If a CLI path is discovered in State 7, an additional E2E smoke test must be added.
2. **PO-003 waiver acceptance**: The `KANI-REC-001` / `PO-003` waiver must be accepted by the next State 6 reviewer or the Kani harness must be authored and executed before State 7 completion.
3. **TLC tooling (PO-015)**: `java -jar tla2tools.jar` remains blocked. State 7 evidence must either provide the jar or record an explicit `BLOCKED_TOOLING` waiver with a follow-up trigger from the next State 6 reviewer.
4. **Canonical proof gate (PO-009)**: `moon run :verify-proof` remains blocked. State 7 must either repair the gauntlet script invocation or record a new waiver with explicit expiry before the next State 6 review.
5. **Collect extra test discovery**: `INT-REC-003` targets `crates/vb_runtime/src/primitives/collect.rs` and `collect_tests.rs`. If `hydrate_journal_events` is not directly invokable from integration tests, a test wrapper must be added in State 7 before `cargo nextest` can pass.

---

## Evidence Requirements for State 7 Completion

| Obligation | Mode | Evidence Required |
|---|---|---|
| PO-004 | proptest | `proptest-recovery-output.txt` — deterministic replay, monotonic tail, taint no-downgrade, collect identity pass |
| PO-005 | cargo-nextest | `test-output-vb-storage-recovery-integration.txt` — header, full-journal, snapshot-tail, pc/slots/taint pass |
| PO-006 | cargo-nextest | `test-output-vb-storage-replay-resume.txt` — wait/ask/action fail-closed pass |
| PO-007 | cargo-nextest | `test-output-vb-runtime-collect.txt` — collect cursor/extra hydration pass |
| PO-008 | mutation-smoke | `mutation-recovery-output.txt` — 9 typed error variants + PRE-006 boundary killed or waived |
| GA-001 | moon ci | `moon_run_test_after_bead_changes` — canonical test gate passes |

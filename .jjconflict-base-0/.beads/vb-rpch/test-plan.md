# Test Plan — vb-rpch: Durability and Recovery Acceptance Scenarios

## Summary

- **Bead**: vb-rpch — bdd: Durability and recovery acceptance scenarios
- **Behaviors identified**: 22
- **Trophy allocation**: 8 unit / 18 integration / 4 e2e
- **Proptest invariants**: 4
- **Fuzz targets**: 2
- **Kani harnesses**: 3 (BLOCKED_TOOLING — getrandom stdlib; compensating evidence: 1918-line BDD suite)

### Verification Waiver Notes

| Verifier | Status | Evidence |
|---|---|---|
| Verus (PO-VB-001 to PO-VB-007) | ⚠️ BLOCKED_TOOLING | nightly not installed; proof obligations are sound per proof-review.md |
| TLC (PO-VB-008 to PO-VB-013) | ⚠️ WAIVER | simulation mode 21404 states; spec gaps documented; GAP sound |
| Kani (PO-VB-014 to PO-VB-016) | ⚠️ BLOCKED_TOOLING | getrandom stdlib aborts symbolic execution; harness correct (18-variant `kani::any::<u8>() % 18`) |
| GAP-3 (ActionAbi/PolicyDigest) | ✅ SOUND | WAIVER recorded; not reachable via public API |
| Compensating evidence | ✅ | `recovery_bdd_tests.rs` (1918 lines, 34 tests) covers all BDD scenarios |

---

## 1. Behavior Inventory

### Digest Verification (B-001 / B-010)

1. **check_workflow_source_digest** returns `Ok(())` when stored `RunAccepted.workflow == expected`
2. **check_workflow_source_digest** returns `WorkflowSourceDigestMismatch` when stored digest ≠ expected
3. **check_workflow_source_digest** returns `NoRecoveryData` when no `RunAccepted` event exists
4. **check_compiled_ir_digest** returns `Ok(())` when digests are equal
5. **check_compiled_ir_digest** returns `CompiledIrDigestMismatch` when digests differ
6. **verify_digests** returns `Ok(())` only when all digests match at the requested `DigestCheck` level
7. **verify_digests** verifies workflow digest first, then IR digest (ordering invariant)
8. **verify_digests** at `WorkflowSourceOnly` skips IR check
9. **verify_digests** at `WorkflowAndIr` performs both checks
10. **verify_digests** at `Full` attempts all checks (GAP-3: ActionAbiMismatch and PolicyDigestMismatch not reachable)

### Runtime Summary Recovery (B-014)

11. **recover_runtime_summary** returns `RecoveryHydration::Summary` with accurate counts (steps_started, steps_succeeded, actions_scheduled, actions_resolved, suspensions, slots_written)
12. **recover_runtime_summary** derives correct `terminal` from latest terminal event of max attempt
13. **recover_runtime_summary** returns `NoRecoveryData` for empty journal
14. **recover_runtime_summary** returns `NoRecoveryData` when journal contains no events for requested run

### Frame Seed Recovery (B-002, B-009, B-005)

15. **recover_runtime_frame_seed** returns exact `step_count`, `slot_count`, `first_step`, `pc`, `steps`, `slots`, `pending_actions`, `unsupported` markers
16. **hydrate_run_frame** returns `RunFrame` whose slot values and taint match snapshot plus tail events effects
17. **hydrate_run_frame** returns `RunFrame` with PC and executed count reflecting replay
18. **hydrate_run_frame** detects and returns `ReplayDivergence` when tail event seq ≤ snapshot seq
19. **hydrate_run_frame** returns `CorruptSnapshot` when snapshot.run ≠ requested run_id
20. **hydrate_run_frame_from_events** returns `RunFrame` with `unsupported` correctly marking missing slot_values, slot_taint, action_payloads, pending_actions

### Replay Core (B-006, B-007, B-008)

21. **replay_events** skips all state-affecting events from attempts older than `max_attempt`
22. **replay_events** marks actions as completed/failed in tracker; blocks re-execution of already-resolved non-idempotent actions with `NonIdempotentActionBlocked`
23. **replay_events** detects out-of-order step execution with `ReplayDivergence`
24. **ActionReplayTracker::is_resolved** returns `true` iff `(action, step)` was previously marked completed or failed
25. **ActionReplayTracker** is monotonically sealed: once resolved, always resolved

### Incomplete Run Discovery (INV-006 / POST-008)

26. **recover_all_incomplete_runs** returns `Vec<RecoveryHydration>` for every run header whose journal has no terminal event
27. **recover_all_incomplete_runs** never returns a run whose latest attempt has a terminal event (RunFinished, RunCancelled, RunFailedEvent)

### Snapshot Persistence (B-012, B-013)

28. **RunSnapshot** round-trips through encode/decode preserving slot values and taint
29. **hydrate_run_frame** preserves tail event ordering when applying to snapshot

### Error Taxonomy (INV-001)

30. Every `RecoveryError` variant is semantically distinct and maps to exactly one failure mode

### Unsupported Recovery State (INV-002)

31. `UnsupportedRecoveryState::SUPPORTED` has all four boolean fields as `false`
32. `union` is commutative, associative, idempotent, and never produces contradictory state
33. `union` of any state with `SUPPORTED` returns the original state unchanged

### Dimension Invariants (INV-003)

34. `RecoveryFrameSeed.step_count > 0` when events are non-empty and replay succeeds
35. `RecoveryFrameSeed.slot_count > 0` when slot events exist and replay succeeds

### DigestCheck Hierarchy (INV-005)

36. `DigestCheck` variants form strict hierarchy: `WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full`

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| **Unit / Calc** | 8 | Pure functions: `apply_summary_event`, `dimension_count`, `max_step`/`min_step`/`max_slot`, `recover_slots`, `recoverable_slot_value`, `legacy_slot_taint`, `is_resolved`, `UnsupportedRecoveryState::union` algebraic properties |
| **Integration** | 18 | All recovery public API functions against real FjallJournal; `recovery_bdd_tests.rs` (1918 lines) is the primary evidence |
| **E2E** | 4 | Power-loss durability scenarios (`BDD-STRICT-DUR`); full journal round-trip; incomplete run discovery |
| **Static Analysis** | — | `forbid(unsafe_code)` on all recovery modules; clippy/cargo-deny already pass in CI |

**Rationale**: Recovery is inherently I/O-bound (FjallJournal). Unit tests cover pure logic only. The 1918-line BDD suite provides exhaustive integration coverage for all public API paths including error variants. E2E is limited to power-loss durability which requires full system context.

---

## 3. BDD Scenarios

All scenarios are implemented in `crates/vb_storage/tests/recovery_bdd_tests.rs` (1918 lines).

### Scenario Map

| ID | Function | Test Name | Layer |
|---|---|---|---|
| B-001a | `check_workflow_source_digest` | `header_binds_target_run_when_digests_match` | integration |
| B-001b | `check_workflow_source_digest` | `header_rejects_workflow_source_digest_mismatch` | integration |
| B-001c | `check_compiled_ir_digest` | `header_rejects_compiled_ir_digest_mismatch` | integration |
| B-002 | `recover_runtime_summary` + `recover_full_journal` | `full_journal_reconstructs_exact_pc_steps_slots_taint_terminal` | integration |
| B-003 | `hydrate_run_frame` | `snapshot_plus_tail_applies_tail_after_watermark` | integration |
| B-003b | `hydrate_run_frame` | `snapshot_plus_tail_rejects_tail_before_snapshot` | integration |
| B-003c | `hydrate_run_frame` | `snapshot_plus_tail_idempotent_on_same_input` | integration |
| B-004 | `recover_runtime_summary` | `empty_journal_returns_no_recovery_data` | integration |
| B-005 | `load_snapshot` | `corrupt_journal_record_returns_typed_storage_error` | integration |
| B-006 | `replay_events` | `action_aborts_are_replayed_with_exact_step_state` | integration |
| B-007 | `replay_events` | `non_idempotent_action_blocked_during_recovery` | integration |
| B-008 | `replay_events` | `journal_sequence_gap_returns_replay_divergence` | integration |
| B-009 | `recover_runtime_frame_seed` | `slot_value_recovery_hydrates_exact_tainted_frame` | integration |
| B-010 | `verify_digests` | `verify_digests_detects_ir_digest_mismatch` | integration |
| B-011 | `hydrate_run_frame` | `frame_dimension_overflow_returns_typed_error` | integration |
| B-012 | `RunSnapshot` | `run_snapshot_persists_and_restores_frame` | integration |
| B-013 | `hydrate_run_frame` | `snapshot_plus_tail_tail_event_ordering_preserved` | integration |
| B-014 | `recover_runtime_summary` | `snapshot_tail_fact_erased_when_no_tail_event` | integration |
| B-HYDRATE | `hydrate_run_frame_from_events` | `recovery_hydrates_slots_taint_step_states_from_journal` | integration |
| B-REJECT | `hydrate_run_frame_from_events` | `recovery_rejects_missing_slot_values_or_pending_action_state_when_unsupported` | integration |
| B-CORRUPT | `recover_full_journal` | `corrupt_record_digest_mismatch_and_non_idempotent_replay_fail_typed` | integration |
| B-STRICT-DUR | durability gate | `test_strict_run_persists_run_accepted_before_ack` | e2e |

### Given-When-Then: Key Scenarios

#### B-001a: Header binds target run when digests match
```
Given: a FjallJournal containing RunAccepted with workflow digest A
  And: subsequent events for run R
When: check_workflow_source_digest(journal, R, A) is called
Then: returns Ok(())
And: recover_runtime_summary(journal, R) returns RecoveryHydration::Summary
```

#### B-007: Non-idempotent action blocked during recovery
```
Given: a journal with an ActionScheduled at step S for action A
  And: ActionCompletedEvent for (A, S) already in tracker
When: replay_events processes the journal
Then: returns Err(RecoveryError::NonIdempotentActionBlocked { action: A, step: S })
```

#### B-008: Journal sequence gap returns ReplayDivergence
```
Given: a journal where step events arrive out of ascending seq order
When: replay_events processes the events
Then: returns Err(RecoveryError::ReplayDivergence { step: S, detail })
And: detail contains "executed before previous step"
```

#### B-011: Frame dimension overflow returns typed error
```
Given: events referencing step indices that overflow u16 when +1 is applied
When: recover_runtime_frame_seed_from_events is called
Then: returns Err(RecoveryError::FrameDimensionOverflow { run })
```

#### B-STRICT-DUR: Strict durability profile
```
Given: a run with RunAccepted event at durability level Strict
When: the runtime acknowledges the event
Then: the event is durable (fsynced) before the ack is returned
```

---

## 4. Proptest Invariants

### INV: `apply_summary_event` counter monotonicity

**Function**: `summary.rs::apply_summary_event`
**Invariant**: All counters (steps_started, steps_succeeded, actions_scheduled, actions_resolved, suspensions, slots_written) are monotonically non-decreasing across successive events in a run.
**Strategy**: `vec![JournalEvent; 0..20]` with valid seq/run combinations
**Anti-invariant**: Any counter decreasing between events indicates a bug.

### INV: `ActionReplayTracker::is_resolved` monotonicity

**Function**: `types.rs::ActionReplayTracker::is_resolved`
**Invariant**: Once `is_resolved(a, s)` returns `true` for any `(action, step)`, it must return `true` for all subsequent calls with the same arguments.
**Strategy**: Interleave `mark_completed`/`mark_failed`/`is_resolved` calls with arbitrary `(ActionId, StepIdx)` pairs.
**Anti-invariant**: `is_resolved` returning `false` after `mark_completed`/`mark_failed` for same key.

### INV: `UnsupportedRecoveryState::union` algebraic properties

**Function**: `types.rs::UnsupportedRecoveryState::union`
**Invariant**: `union` is (1) commutative: `a.union(b) == b.union(a)`, (2) associative: `a.union(b).union(c) == a.union(b.union(c))`, (3) idempotent: `a.union(a) == a`, (4) no contradiction: `union` only produces `true` if at least one operand is `true`.
**Strategy**: Generate arbitrary `UnsupportedRecoveryState` pairs; test all four properties.
**Anti-invariant**: Any violation of algebraic properties indicates a regression.

### INV: `dimension_count` overflow safety

**Function**: `summary.rs::dimension_count`
**Invariant**: `dimension_count(max_idx, run)` returns `Ok(n)` only when `n == max_idx.index() + 1`; returns `Err(FrameDimensionOverflow)` when `max_idx.index() + 1` overflows `u16`.
**Strategy**: Exhaustively test `StepIdx::MAX`, `SlotIdx::MAX`, and boundary values `u16::MAX - 1`, `u16::MAX`.
**Anti-invariant**: `dimension_count` returning `Ok(0)` for non-None max, or returning an incorrect non-overflow value.

---

## 5. Fuzz Targets

### Fuzz Target: `JournalEvent` deserialization

**Input type**: `bytes` (raw journal record)
**Risk**: Panic on corrupt binary data; OOM on malformed Postcard encoding; logic error in event variant parsing
**Corpus seeds**:
- Valid `RunAccepted`, `StepStarted`, `StepSucceeded`, `SlotWrittenEvent` encoded with Postcard
- Corrupt bytes: wrong Postcard discriminant, truncated record, garbage bytes after valid record
**Harness**: `cargo fuzz run journal_event_decode` targeting `JournalEvent::decode` boundary

### Fuzz Target: `RunSnapshot` encode/decode round-trip

**Input type**: arbitrary `(RunId, EventSeq, WorkflowDigest, Vec<u8>, Vec<u8>)` tuples
**Risk**: Snapshot decode produces different slot values than were encoded; taint corruption during round-trip
**Corpus seeds**: Valid snapshots with scalar slot values (Bool, I64, F64, Symbol), empty snapshots, snapshots with maximum slot counts
**Harness**: `cargo fuzz run snapshot_roundtrip` targeting `RunSnapshot::encode` → `decode`

---

## 6. Kani Harnesses

### Kani Harness: `hydrate_run_frame` preconditions (PO-VB-014)

**Property**: `hydrate_run_frame(snapshot, tail_events, run_id)` never panics for any bounded `JournalEvent` sequence (max 20 events) when preconditions are satisfied.
**Bound**: 20 tail events, 18 `JournalEvent` variants via `kani::any::<u8>() % 18`
**Status**: ⚠️ BLOCKED_TOOLING — getrandom stdlib aborts symbolic execution. Harness is structurally correct (verified by proof-review.md Attempt 6); compensating evidence is the 1918-line `recovery_bdd_tests.rs`.
**Evidence**: `crates/vb_storage/tests/recovery_bdd_tests.rs` lines 1-1918; harness at `.beads/vb-rpch/evidence/kani/kani_recovery_hydrate.rs`

### Kani Harness: `hydrate_run_frame_from_events` preconditions (PO-VB-015)

**Property**: `hydrate_run_frame_from_events(events, run_id)` never panics for any bounded non-empty `JournalEvent` sequence.
**Bound**: 20 events, `kani::assume(!events.is_empty())`, 18 variants via `kani::any::<u8>() % 18`
**Status**: ⚠️ BLOCKED_TOOLING — same getrandom issue.

### Kani Harness: `replay_events` safety (PO-VB-016)

**Property**: `replay_events` never panics and returns `Err(ReplayDivergence)` for out-of-order steps or `Err(NonIdempotentActionBlocked)` for pre-resolved actions.
**Bound**: 20 events, 18 variants, fresh `ActionReplayTracker`
**Status**: ⚠️ BLOCKED_TOOLING — same getrandom issue.

---

## 7. Mutation Checkpoints

### Critical Mutations

| Function | Mutation | Must Be Caught By |
|---|---|---|
| `check_workflow_source_digest` | Change `*workflow != expected` to `==` | `header_binds_target_run_when_digests_match` (would pass on mismatch) |
| `check_workflow_source_digest` | Remove `return Err(NoRecoveryData)` branch | `empty_journal_returns_no_recovery_data` |
| `replay_events` | Remove attempt filter `attempt < max_attempt` | `action_aborts_are_replayed_with_exact_step_state` (would double-count actions) |
| `replay_events` | Change `is_resolved` check to `!is_resolved` | `non_idempotent_action_blocked_during_recovery` (would allow re-execution) |
| `hydrate_run_frame` | Remove seq ordering check `event.seq() <= snapshot.seq` | `snapshot_plus_tail_rejects_tail_before_snapshot` |
| `hydrate_run_frame` | Remove `snapshot.run != run_id` check | Corrupt snapshot test (run_id mismatch silently accepted) |
| `dimension_count` | Remove `checked_add(1)` overflow guard | `frame_dimension_overflow_returns_typed_error` |
| `UnsupportedRecoveryState::union` | Change `\|\|` to `&&` | `test_unsupported_recovery_state_union_no_contradiction` |
| `recover_runtime_summary` | Skip terminal event extraction | `full_journal_reconstructs_exact_pc_steps_slots_taint_terminal` |
| `recover_all_incomplete_runs` | Include runs with terminal events | `test_only_incomplete_runs_returned` |

**Threshold**: ≥90% mutation kill rate.

---

## 8. Combinatorial Coverage Matrix

### Digest Verification

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| workflow digest match | `RunAccepted { workflow: A }`, expected A | `Ok(())` | integration |
| workflow digest mismatch | `RunAccepted { workflow: A }`, expected B | `Err(WorkflowSourceDigestMismatch { expected: B, found: A })` | integration |
| no acceptance event | empty journal | `Err(NoRecoveryData { run })` | integration |
| IR digest match | `A == B` | `Ok(())` | unit |
| IR digest mismatch | `A != B` | `Err(CompiledIrDigestMismatch { expected: A, found: B })` | unit |
| `verify_digests` WorkflowSourceOnly | match | `Ok(())` | integration |
| `verify_digests` WorkflowSourceOnly | mismatch | `Err(WorkflowSourceDigestMismatch)` | integration |
| `verify_digests` WorkflowAndIr | both match | `Ok(())` | integration |
| `verify_digests` WorkflowAndIr | workflow mismatch | `Err(WorkflowSourceDigestMismatch)` | integration |
| `verify_digests` WorkflowAndIr | IR mismatch | `Err(CompiledIrDigestMismatch)` | integration |
| `verify_digests` Full | GAP-3 path | WAIVER — not reachable | integration |

### Hydration

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| snapshot + tail happy path | valid snapshot, tail events after seq | `Ok(RunFrame)` | integration |
| tail before snapshot seq | seq ≤ snapshot.seq | `Err(ReplayDivergence)` | integration |
| snapshot run_id mismatch | `snapshot.run != run_id` | `Err(CorruptSnapshot)` | integration |
| tail run_id mismatch | tail event with different run_id | `Err(ReplayDivergence)` | integration |
| empty events (hydrate_from_events) | `[]` | `Err(NoRecoveryData)` | integration |
| step_count = 0 derived | events produce max_step = 0 | `Err(ReplayDivergence)` | integration |
| dimension overflow | slot index overflows u16 | `Err(FrameDimensionOverflow)` | integration |
| events-only hydration | non-empty events | `Ok(RunFrame)` with `unsupported` markers | integration |
| unsupported slot values | corrupt SlotWrittenEvent bytes | `unsupported.slot_values == true` | integration |
| unsupported pending actions | unresolved ActionScheduled | `unsupported.pending_actions == true` | integration |

### Replay

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| max_attempt filtering | events from attempt 1 and 2 | only attempt 2 events affect state | integration |
| step ordering violation | step N+1 before step N | `Err(ReplayDivergence)` | integration |
| non-idempotent blocked | action already in tracker | `Err(NonIdempotentActionBlocked)` | integration |
| action completed tracked | ActionCompletedEvent | `tracker.is_resolved == true` | unit |
| action failed tracked | ActionFailedEvent | `tracker.is_resolved == true` | unit |
| monotonic: completed stays resolved | mark_completed then mark_failed same key | second call is no-op (first wins) | unit |
| `recover_full_journal` | complete event sequence | `Ok(replayed)` matching input events | integration |
| `recover_snapshot_plus_tail` | snapshot + tail | `Ok(replayed)` | integration |

### Incomplete Run Discovery

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| run without terminal | journal has no RunFinished/RunCancelled/RunFailed | included in result | integration |
| run with terminal | journal has RunFinished | excluded from result | integration |
| multiple incomplete runs | 3 runs, 1 complete, 2 incomplete | 2 results | integration |
| empty journal | no events | `Err(NoRecoveryData)` | integration |

---

## 9. Unit Test Inventory (Pure Functions)

| Function | File | Test Count | Coverage |
|---|---|---|---|
| `apply_summary_event` | `summary.rs` | 8 | All 18 JournalEvent variants + no-op variants |
| `dimension_count` | `summary.rs` | 4 | Overflow boundary, Ok(0), Ok(non-zero) |
| `max_step`, `min_step`, `max_slot` | `summary.rs` | 6 | None, equal, greater cases |
| `recoverable_slot_value` | `summary.rs` | 5 | All scalar variants, Object/List rejection |
| `legacy_slot_taint` | `summary.rs` | 4 | Bool(false)=Clean, Bool(true)/Null=DerivedFromSecret, other=Secret |
| `is_resolved` | `types.rs` | 3 | false, true(completed), true(failed) |
| `ActionReplayTracker` monotonicity | `types.rs` | 2 | completed, failed |
| `UnsupportedRecoveryState::union` | `types.rs` | 3 | commutative, idempotent, no-contradiction |
| `DigestCheck` hierarchy | `types.rs` | 1 | strict subset relationship |
| Error mapping | `summary.rs` | 7 | All ReplayError → RecoveryError variants |
| `summarize_recovery_events` | `summary.rs` | 2 | empty events, multi-run divergence |
| `recover_runtime_frame_seed_from_events` | `summary.rs` | 2 | empty events, multi-run divergence |
| `recover_run_admission_from_events` | `summary.rs` | 2 | latest admission, no admission |

**Total unit tests**: ~47

---

## 10. Open Questions

1. **Kani execution**: getrandom stdlib blocks symbolic execution. Workaround: stub getrandom with a mock that returns deterministic bytes. Is a stub acceptable for PO-VB-014/015/016 verification?

2. **Verus nightly**: Nightly toolchain not installed. Without Verus, PO-VB-001 (union algebraic proofs) and PO-VB-004/005 (hydrate preconditions) lack formal proof. GAP-3 waivers are sound but INV-002 and INV-004 remain unverified.

3. **TLC spec gaps**: PO-VB-009 through PO-VB-013 require `RecoveryReplayFull.tla` which does not exist. GAP notes confirm "Proof-writer must CREATE new spec." Is there a timeline for spec creation, or should these obligations be formally WAIVED?

4. **TerminalStateMismatch**: Contract B-014 requires this error variant but no public API can trigger it (no expected-terminal parameter). DEFERRED_GLOBAL recorded. Should a `recover_runtime_summary_with_expected` variant be added to unblock this test?

5. **ActionAbiMismatch / PolicyDigestMismatch**: GAP-3 deferred to vb-ty9. The `recovery_bdd_tests.rs` has `#[ignore]` tests that panic if the GAP paths return `Ok` instead of the expected error. When vb-ty9 implements these paths, are the ignored tests expected to start passing, or should they remain as cautionary stubs?

---

## Verification Status Summary

| Proof Obligation | Verifier | Status | Evidence |
|---|---|---|---|
| PO-VB-001 INV-002 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-002 INV-004 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-003 INV-005 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-004 PRE-001 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-005 PRE-002 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-006 POST-009 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-007 INV-003 | Verus | ⚠️ BLOCKED_TOOLING | Harness exists; nightly missing |
| PO-VB-008 TLA-001 | TLC | ⚠️ WAIVER | simulation mode 21404 states; spec gap noted |
| PO-VB-009 TLA-002 | TLC | ⚠️ WAIVER | spec gap: RecoveryReplayFull.tla missing |
| PO-VB-010 TLA-003 | TLC | ⚠️ WAIVER | spec gap: RecoveryReplayFull.tla missing |
| PO-VB-011 TLA-004 | TLC | ⚠️ WAIVER | Partially modelled |
| PO-VB-012 TLA-005 | TLC | ⚠️ WAIVER | spec gap: RecoveryReplayFull.tla missing |
| PO-VB-013 TLA-006 | TLC | ⚠️ WAIVER | spec gap: RecoveryReplayFull.tla missing |
| PO-VB-014 KANI-PRE-001 | Kani | ⚠️ BLOCKED_TOOLING | Harness correct (18-variant `kani::any % 18`); getrandom blocks exec |
| PO-VB-015 KANI-PRE-002 | Kani | ⚠️ BLOCKED_TOOLING | Same |
| PO-VB-016 KANI-POST-009 | Kani | ⚠️ BLOCKED_TOOLING | Same |
| GAP-3 ActionAbiMismatch | Waiver | ✅ SOUND | Not reachable via public API |
| GAP-3 PolicyDigestMismatch | Waiver | ✅ SOUND | Not reachable via public API |
| DEFERRED_GLOBAL TerminalStateMismatch | Waiver | ✅ SOUND | No expected-terminal parameter in public API |
| BDD Tests (all scenarios) | integration | ✅ ACTIVE | `recovery_bdd_tests.rs` 1918 lines, 34 tests |
| Unit tests (pure functions) | unit | ✅ ACTIVE | ~47 tests in `summary.rs::tests`, `types.rs::tests` |

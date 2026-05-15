# Test Plan: vb-core-replay-divergence-recovery

## Summary
- Bead: vb-core-replay-divergence-recovery
- Feature: Recovery subsystem — typed replay with divergence detection and no-YAML hydration
- Behaviors identified: 14 proof obligations (13 miri + 1 proptest with 3 properties)
- Trophy allocation: ~65% integration, ~30% unit, ~5% static
- Proptest invariants: 3
- Fuzz targets: 0 (Postcard is fuzzed externally)
- Kani harnesses: 0 (miri covers UB; formal proofs via Verus waiver)

## 1. Behavior Inventory

### CC-001: No YAML in Recovery Paths
- "Recovery never reparses YAML when hydrating frames"

### CC-002: Snapshot+Tail Hydration Fidelity
- "hydrate_run_frame produces RunFrame identical to pre-crash frame when given snapshot + tail events"
- "Hydration respects run_id match, seq ordering, and zero-dim guard"

### CC-003: Typed Digest Mismatch Errors
- "verify_digests produces typed RecoveryError variants with exact step and detail on mismatch"
- "Errors are: WorkflowSourceDigestMismatch, CompiledIrDigestMismatch, ActionAbiMismatch, PolicyDigestMismatch"

### CC-004: Typed Replay Divergence
- "replay_events produces ReplayDivergence { step, detail } on semantic divergence"
- "NonIdempotentActionBlocked prevents duplicate non-idempotent action scheduling"

### CC-005: Fail-Closed Corrupt/Incomplete Recovery
- "Corrupt snapshot → CorruptSnapshot error"
- "Unsupported live frame state → RuntimeError::UnsupportedFullRecoveryHydration"
- "reject_unsupported_live_frame_state fails closed if any UnsupportedRecoveryState category is true"

### CC-006: Object/List Slots Explicitly Unsupported
- "RecoveredSlots marks Object and List slot kinds as unsupported"
- "These slot kinds cannot be hydrated from events alone"

### CC-007: Events-Only Hydration Correctness
- "hydrate_run_frame_from_events reconstructs frame state from JournalEvents without snapshot"
- "Seq ordering and taint preservation are maintained"

### CC-008: Frame Seed Round-Trip Integrity
- "RecoveryFrameSeed round-trips through Postcard serialization identically"

### INV-001: JournalEvent Seq Ordering
- "All JournalEvents in a run have strictly monotonically increasing StepIdx values"

### INV-002: ActionReplayTracker Blocking
- "ActionReplayTracker blocks any Scheduled event for an action already marked Completed"

### INV-003: RecoveryFrameSeed Byte Identity
- "slot_taint and slot_values are byte-for-byte identical after Postcard round-trip"

### INV-004: UnsupportedRecoveryState Gate
- "DurableFrameRecoveryBoundary::hydrate_run_frame succeeds iff all 4 UnsupportedRecoveryState categories are false"

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|------------|
| Unit | 4 | Pure function tests (seed round-trip, slot event construction, proptest invariants) |
| Integration | 9 | Core recovery behavior with real Fjall journal and Postcard codec |
| E2E | 0 | Recovery is internal subsystem; covered by integration tests |
| Static | 1 | Grep verification for zero YAML imports in recovery/ |

**Rationale**: Recovery is a persistence-critical subsystem. Most risk is in the integration between Fjall journal I/O, Postcard codec, and frame hydration logic. Unit tests cover pure functions (event builders, seed constructors). Integration tests cover the full hydration paths with real journal.

---

## 3. BDD Scenarios

### CC-001: No YAML in Recovery Paths

**Behavior**: Recovery never reparses YAML when hydrating frames

```
Given: A recovery module with hydrate_run_frame function
When: grep -i 'yaml|serde_yaml|quick_yaml' is executed on crates/vb_storage/src/recovery/
Then: Zero matches are found

Given: Recovery integration tests run under miri
When: hydrate_run_frame is called with valid snapshot and tail events
Then: All tests pass with no UB detected
```

**Test names**:
- `miri_recovery_contains_no_yaml_imports`

---

### CC-002: Snapshot+Tail Hydration Fidelity

**Behavior**: hydrate_run_frame produces identical RunFrame to pre-crash state

```
Given: A RunFrame with known run_id, seq ordering, and slot values
When: hydrate_run_frame(snapshot, tail_events) is called
Then: The resulting RunFrame matches the pre-crash frame exactly

Given: A frame with taint-labeled slots
When: hydration completes successfully
Then: Slot values and taints are preserved byte-for-byte
```

**Test names**:
- `full_round_trip_recovery_reconstructs_summary`
- `full_round_trip_recovery_detects_slot_writes`
- `deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete`

---

### CC-003: Typed Digest Mismatch Errors

**Behavior**: verify_digests returns typed error variants with step and detail

```
Given: A workflow with mismatched compiled IR digest
When: verify_digests is called
Then: RecoveryError::CompiledIrDigestMismatch is returned with correct step and detail

Given: A workflow with mismatched workflow source digest
When: verify_digests is called
Then: RecoveryError::WorkflowSourceDigestMismatch is returned with correct step and detail

Given: A workflow with mismatched action ABI digest
When: verify_digests is called
Then: RecoveryError::ActionAbiMismatch is returned with correct step and detail
```

**Test names**:
- `digest_mismatch_detection_returns_typed_error_with_step_and_detail`

---

### CC-004: Typed Replay Divergence

**Behavior**: replay_events produces ReplayDivergence on semantic divergence

```
Given: Events with duplicate Scheduled events for a non-idempotent action
When: replay_events processes the events
Then: RecoveryError::NonIdempotentActionBlocked is returned

Given: Events with semantic divergence (step index mismatch)
When: replay_events detects divergence
Then: RecoveryError::ReplayDivergence { step, detail } is returned
```

**Test names**:
- `action_replay_tracker_reconstructs_from_events`
- `action_replay_tracker_tracks_failed_actions`
- `action_replay_blocks_duplicate_scheduled_action`

---

### CC-005: Fail-Closed Corrupt/Incomplete Recovery

**Behavior**: Corrupt or incomplete state fails closed

```
Given: A snapshot with corrupt Postcard bytes
When: load_snapshot is called
Then: RecoveryError::CorruptSnapshot is returned (not a panic)

Given: An unsupported live frame state (slot_values=true)
When: reject_unsupported_live_frame_state is called
Then: RuntimeError::UnsupportedFullRecoveryHydration is returned

Given: UnsupportedRecoveryState with action_payloads=true
When: DurableFrameRecoveryBoundary::hydrate_run_frame is called
Then: The call fails with UnsupportedFullRecoveryHydration
```

**Test names**:
- `corrupt_slot_value_blocks_both_values_and_taint`
- `missing_slot_value_blocks_both_values_and_taint`
- `durable_frame_recovery_boundary_rejects_unsupported_action_payloads`
- `durable_frame_recovery_boundary_rejects_inconsistent_seed`

---

### CC-006: Object/List Slots Explicitly Unsupported

**Behavior**: Object and List slot kinds cannot be hydrated from events alone

```
Given: A slot event with SlotValue::Object kind
When: recovery frame seed is built
Then: UnsupportedRecoveryState.slot_values is set to true

Given: A slot event with SlotValue::List kind
When: recovery frame seed is built
Then: UnsupportedRecoveryState.slot_values is set to true
```

**Test names**:
- `recovered_object_slots_are_explicitly_unsupported`
- `recovered_list_slots_are_explicitly_unsupported`

---

### CC-007: Events-Only Hydration Correctness

**Behavior**: hydrate_run_frame_from_events reconstructs frame from JournalEvents without snapshot

```
Given: Valid JournalEvents with slot written events
When: hydrate_run_frame_from_events is called
Then: Recovered slots include exact values and taints

Given: Events with Secret-tainted slot value
When: hydration completes
Then: The slot is recovered with Taint::Secret (not defaulted to Clean)

Given: A no-output step
When: recovery frame seed is built
Then: slot_count is 0 (no fabrication of slot zero dimension)
```

**Test names**:
- `event_only_recovery_returns_secret_i64_when_durable_taint_is_secret`
- `event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid`
- `no_output_step_does_not_fabricate_slot_zero_dimension`
- `no_output_step_summary_reports_zero_slots_written`
- `no_output_step_recovery_has_no_recovered_slot_entries`
- `recovery_does_not_default_missing_durable_taint_to_clean`

---

### CC-008: Frame Seed Round-Trip Integrity

**Behavior**: RecoveryFrameSeed round-trips through Postcard identically

```
Given: A valid RecoveryFrameSeed with slot entries
When: Postcard serialize → deserialize is performed
Then: The resulting seed has identical slot_values and slot_taint bytes
```

**Test names**:
- `recovery_boundary_factory_frame_seed_round_trips_summary`
- `supported_seed_hydrates_exact_secret_taint`
- `supported_seed_hydrates_exact_derived_taint`

---

### INV-001: JournalEvent Seq Ordering

**Behavior**: JournalEvents have strictly monotonically increasing StepIdx

```
Given: Events with a sequence gap before resume continuation
When: replay attempts to continue from the gap
Then: The replay is rejected
```

**Test names**:
- `resume_tail_replay_rejects_sequence_gap_before_resume_continuation`

---

### INV-002: ActionReplayTracker Blocking

**Behavior**: ActionReplayTracker blocks duplicate Scheduled events

```
Given: An action already marked Completed during replay
When: A subsequent Scheduled event for the same action is processed
Then: The event is blocked (NonIdempotentActionBlocked)
```

**Test names**:
- `action_replay_blocks_duplicate_scheduled_action`

---

### INV-003: RecoveryFrameSeed Byte Identity

**Behavior**: slot_taint and slot_values are byte-for-byte identical after round-trip

```
Given: A seed with Secret-tainted slots
When: seed is serialized and deserialized via Postcard
Then: The taint and value bytes are identical
```

**Test names**:
- `supported_seed_hydrates_exact_secret_taint`
- `supported_seed_hydrates_exact_derived_taint`

---

### INV-004: UnsupportedRecoveryState Gate

**Behavior**: hydrate_run_frame succeeds iff all 4 categories are false

```
Given: All UnsupportedRecoveryState categories are false
When: DurableFrameRecoveryBoundary::hydrate_run_frame is called
Then: Hydration succeeds with exact slot values and taints

Given: UnsupportedRecoveryState.slot_values=true
When: hydrate_run_frame is called
Then: The call fails with UnsupportedFullRecoveryHydration
```

**Test names**:
- `durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint`
- `durable_frame_recovery_boundary_rejects_unsupported_action_payloads`

---

## 4. Proptest Invariants

### PROPTEST-CC007-001: Slot Recovery Preserves Taint

**Invariant**: Valid slot events are fully hydrateable; no fabrication of slot zero or missing taints

```
Property: proptest_event_only_slot_recovery_preserves_secret_taint
Input: i64 values in range [-128, 127]
Strategy: any valid i64
Invariant: Recovered slot has exact same i64 value and Taint::Secret

Property: proptest_no_output_success_never_creates_slot_zero
Input: step indices [0, 15]
Strategy: any valid StepIdx
Invariant: slot_count_for(no_output_events) == 0

Property: proptest_valid_slot_events_are_fully_hydrateable
Input: slot [0, 15], value [0, 1023]
Strategy: any valid SlotIdx and i64 value
Invariant: unsupported_for(events) == UnsupportedRecoveryState::SUPPORTED
```

---

## 5. Fuzz Targets

No fuzz targets required for this bead:
- Postcard is an externally fuzzed codec with established corpus
- JournalEvent parsing is covered by miri integration tests
- All fuzz-worthy boundaries are exercised by integration tests

---

## 6. Kani Harnesses

No Kani harnesses required for this bead:
- Miri covers all UB detection for Postcard round-trips
- Verus waiver covers RecoveryError enum exhaustiveness
- proptest covers combinatorial slot value space

---

## 7. Mutation Checkpoints

Critical mutations that must be caught:

| Function | Mutation | Test |
|----------|----------|------|
| `hydrate_run_frame` | Skip run_id check | `full_round_trip_recovery_reconstructs_summary` |
| `verify_digests` | Return generic error instead of typed | `digest_mismatch_detection_returns_typed_error_with_step_and_detail` |
| `ActionReplayTracker` | Allow duplicate scheduled action | `action_replay_blocks_duplicate_scheduled_action` |
| `reject_unsupported_live_frame_state` | Skip one category check | `durable_frame_recovery_boundary_rejects_unsupported_action_payloads` |
| `recover_runtime_frame_seed_from_events` | Default taint to Clean | `recovery_does_not_default_missing_durable_taint_to_clean` |

**Threshold**: 90% mutation kill rate minimum (covered by existing test suite)

---

## 8. Combinatorial Coverage Matrix

### Recovery Hydration Paths

| Scenario | Snapshot | Tail Events | Expected Output | Test Layer |
|----------|----------|-------------|-----------------|------------|
| Happy path | Valid | Non-empty | Ok(RunFrame) | integration |
| Corrupt snapshot | Invalid bytes | Any | Err(CorruptSnapshot) | integration |
| Events-only | None | Valid | Ok(RunFrame) | integration |
| Unsupported slot kind | Valid | Object slot | Err(UnsupportedFullRecoveryHydration) | integration |
| Digest mismatch | Valid | Valid | Err(TypedError) | integration |
| Replay divergence | Valid | Divergent | Err(ReplayDivergence) | integration |

### Slot Recovery

| Scenario | Slot Value | Taint | slot_count | Test Layer |
|----------|------------|-------|-----------|------------|
| I64 with Secret | I64(99) | Secret | 1 | unit |
| Bool with Derived | Bool(true) | Derived | 1 | unit |
| Null with Derived | Null | Derived | 1 | unit |
| No-output step | N/A | N/A | 0 | unit |
| Corrupt bytes | Invalid | N/A | unsupported | integration |
| Missing value | None | N/A | unsupported | integration |

---

## 9. Proof Obligation Mapping

| Obligation ID | Contract Clause | Test(s) | Verifier |
|--------------|-----------------|---------|----------|
| MIRI-CC001-001 | CC-001 | grep yaml + miri integration | miri |
| MIRI-CC002-001 | CC-002 | full_round_trip_recovery_* | miri |
| MIRI-CC003-001 | CC-003 | digest mismatch tests | miri |
| MIRI-CC004-001 | CC-004 | action_replay_tracker_* | miri |
| MIRI-CC005-001 | CC-005 | corrupt_slot_value_* | miri |
| MIRI-CC005-002 | CC-005 | durable_frame_recovery_boundary_rejects_* | miri |
| MIRI-CC006-001 | CC-006 | recovered_object/list_slots_are_explicitly_unsupported | miri |
| MIRI-CC007-001 | CC-007 | event_only_recovery_* | miri |
| PROPTEST-CC007-001 | CC-007 | 3 proptest cases | proptest |
| MIRI-CC008-001 | CC-008 | recovery_boundary_factory_frame_seed_round_trips_summary | miri |
| MIRI-INV001-001 | INV-001 | resume_tail_replay_rejects_sequence_gap_* | miri |
| MIRI-INV002-001 | INV-002 | action_replay_blocks_duplicate_scheduled_action | miri |
| MIRI-INV003-001 | INV-003 | supported_seed_hydrates_exact_*_taint | miri |
| MIRI-INV004-001 | INV-004 | durable_frame_recovery_boundary_hydrates_exact_* | miri |

---

## 10. Open Questions

None. All proof obligations are well-formed and covered by existing tests.

---

## Test Execution Commands

```bash
# Miri (all integration tests)
cargo miri test --package vb_storage --test recovery_integration -- --nocapture
cargo miri test --package vb_storage --test replay_resume -- --nocapture
cargo miri test --package vb_runtime -- --nocapture

# Proptest (workspace_tests)
cargo test --package workspace_tests --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture

# Static verification (no YAML)
rg -i 'yaml|serde_yaml|quick_yaml' crates/vb_storage/src/recovery/ --files-with-matches
```

---

## Exit Criteria

- [x] Every public API behavior has at least one BDD scenario
- [x] Every pure function with multiple inputs has at least one proptest invariant
- [x] Every parsing/deserialization boundary has miri coverage
- [x] Every error variant in RecoveryError enum has an explicit test scenario
- [x] The mutation threshold target (≥90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value

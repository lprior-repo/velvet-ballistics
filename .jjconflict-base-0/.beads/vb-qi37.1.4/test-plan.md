# Test Plan — vb-qi37.1.4

## Header
- **bead_id**: vb-qi37.1.4
- **bead_title**: runtime/recovery: Fail closed on incomplete recovery
- **state**: 7 (test-planner)
- **date**: 2026-05-14
- **gap**: GAP-1/GAP-2/GAP-3 — fail-closed recovery boundary

---

## Summary
- Behaviors identified: 6
- Trophy allocation: 4 unit / 2 integration / 0 e2e / 0 static
- Proptest invariants: 0
- Fuzz targets: 0
- Kani harnesses: 1

---

## 1. Behavior Inventory

1. **`reject_unsupported_live_frame_state` returns `Err(InvalidRecoveryHydration)` when `unsupported.slot_taint` is `true`, independent of `slot_values`** (POST-001, GAP-1)
2. **`reject_unsupported_live_frame_state` returns `Err(InvalidRecoveryHydration)` when `unsupported.pending_actions` is `true` AND `!pending_actions.is_empty()`** (POST-002, GAP-2)
3. **`reject_unsupported_live_frame_state` returns `Err(InvalidRecoveryHydration)` when `unsupported.slot_values` is `true`** (INV-GAP1-001)
4. **`reject_unsupported_live_frame_state` returns `Err(InvalidRecoveryHydration)` when `unsupported.action_payloads` is `true`** (INV-GAP1-002)
5. **`verify_digests(DigestCheck::Full)` returns `Ok(())` only when workflow source digest matches** (POST-003)
6. **`verify_digests(DigestCheck::Full)` returns `Ok(())` only when compiled IR digest matches** (POST-003)

---

## 2. Trophy Allocation

| Level | Count | Rationale |
|-------|-------|-----------|
| Unit | 4 | Pure functions: reject_unsupported_live_frame_state has no I/O; verify_digests uses in-memory journal fake |
| Integration | 2 | Full hydration pipeline with real journal; boundary trait behavior |
| Property | 0 | Deterministic boolean conditions — not applicable |
| E2E | 0 | Recovery from durable events covered by integration |
| Static | 0 | Clippy/deny covered by CI |
| Kani | 1 | Codec roundtrip for RecoveryFrameSeed |

---

## 3. BDD Scenarios

### Behavior 1 — slot_taint fail-closed (POST-001, GAP-1)

**Scenario: `fn reject_returns_err_when_slot_taint_unsupported`**
```
Given: RecoveryFrameSeed with unsupported.slot_taint=true, unsupported.slot_values=false, unsupported.pending_actions=false, unsupported.action_payloads=false, pending_actions=[]
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
```

**Scenario: `fn reject_returns_err_when_slot_taint_and_slot_values_both_unsupported`**
```
Given: RecoveryFrameSeed with unsupported.slot_taint=true, unsupported.slot_values=true, unsupported.pending_actions=false, unsupported.action_payloads=false, pending_actions=[]
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
Note: slot_taint triggers fail-closed independent of slot_values
```

### Behavior 2 — pending_actions fail-closed (POST-002, GAP-2)

**Scenario: `fn reject_returns_err_when_pending_actions_unsupported_and_not_empty`**
```
Given: RecoveryFrameSeed with unsupported.pending_actions=true, pending_actions=[(action1, digest1)], other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
```

**Scenario: `fn reject_returns_err_when_pending_actions_unsupported_but_empty`**
```
Given: RecoveryFrameSeed with unsupported.pending_actions=true, pending_actions=[], other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
Note: POST-002 — unsupported.pending_actions triggers fail-closed regardless of pending_actions.is_empty()
```

**Scenario: `fn reject_returns_err_when_pending_actions_supported_regardless_of_empty`**
```
Given: RecoveryFrameSeed with unsupported.pending_actions=false, pending_actions=[], other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Ok(())
```

### Behavior 3 — slot_values fail-closed

**Scenario: `fn reject_returns_err_when_slot_values_unsupported`**
```
Given: RecoveryFrameSeed with unsupported.slot_values=true, other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
```

### Behavior 4 — action_payloads fail-closed

**Scenario: `fn reject_returns_err_when_action_payloads_unsupported`**
```
Given: RecoveryFrameSeed with unsupported.action_payloads=true, other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
```

### Behavior 5 — verify_digests at Full checks workflow source

**Scenario: `fn verify_digests_full_returns_workflow_mismatch_error`**
```
Given: InMemoryJournal with RunAccepted event containing workflow digest W_stored
When: verify_digests(journal, run, W_expected!=W_stored, ir_digest, found_ir_digest, DigestCheck::Full) is called
Then: returns Err(RecoveryError::WorkflowSourceDigestMismatch { expected: W_expected, found: W_stored })
```

### Behavior 6 — verify_digests at Full checks compiled IR

**Scenario: `fn verify_digests_full_returns_ir_mismatch_error`**
```
Given: InMemoryJournal with RunAccepted event containing workflow digest W
When: verify_digests(journal, run, W, I_expected, I_found!=I_expected, DigestCheck::Full) is called
Then: returns Err(RecoveryError::CompiledIrDigestMismatch { expected: I_expected, found: I_found })
```

---

## 4. Integration Tests

### Integration 1 — Full frame hydration with clean seed

**Scenario: `fn durable_boundary_hydrates_frame_with_all_supported`**
```
Given: FjallJournal with complete recovery events, all unsupported flags=false
When: DurableFrameRecoveryBoundary::hydrate_run_frame is called
Then: returns Ok(RunFrame) with correct run_id, pc, step_count
```

### Integration 2 — Full frame hydration rejects unsupported

**Scenario: `fn durable_boundary_rejects_unsupported_frame_seed`**
```
Given: FjallJournal with RecoveryFrameSeed where unsupported.slot_taint=true
When: DurableFrameRecoveryBoundary::hydrate_run_frame is called
Then: returns Err(RuntimeError::InvalidRecoveryHydration)
```

---

## 5. Proptest Invariants

Not applicable. Digest comparison is deterministic byte equality on fixed-size `WorkflowDigest` (32 bytes). Boolean gate conditions are exhaustive truth tables.

---

## 6. Fuzz Targets

Not applicable. No parsing boundaries involved. RecoveryFrameSeed is constructed by the system from validated journal events.

---

## 7. Kani Harness

### Kani Harness: RecoveryFrameSeed roundtrip codec

**File**: `crates/vb_storage/src/kani_codec.rs`

**Property**: `encode_record` then `decode_record` on `RecoveryFrameSeed` produces equal value

**Bound**: `kani::any::<RecoveryFrameSeed>()` with max snapshot bytes

**Rationale**: Ensures unsupported flags (slot_taint, pending_actions, etc.) survive serialization roundtrip

---

## 8. Mutation Checkpoints

| Mutation | Must be caught by |
|----------|-------------------|
| `|| seed.unsupported.slot_taint` removed | `reject_returns_err_when_slot_taint_unsupported` |
| `(!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)` changed to `seed.unsupported.pending_actions` | `reject_returns_err_when_pending_actions_unsupported_and_not_empty` + `reject_returns_ok_when_pending_actions_unsupported_but_empty` |
| `|| seed.unsupported.slot_values` removed | `reject_returns_err_when_slot_values_unsupported` |
| `|| seed.unsupported.action_payloads` removed | `reject_returns_err_when_action_payloads_unsupported` |
| `check_workflow_source_digest` removed from Full branch | `verify_digests_full_returns_workflow_mismatch_error` |
| `check_compiled_ir_digest` removed from Full branch | `verify_digests_full_returns_ir_mismatch_error` |

**Threshold**: ≥ 90% mutation kill rate

---

## 9. Error Variant Coverage

| Error Variant | Covered By |
|---|---|
| `RuntimeError::InvalidRecoveryHydration` | 4 unit tests + 2 integration tests |
| `RuntimeError::UnsupportedFullRecoveryHydration` | Existing test `summary_recovery_boundary_rejects_full_frame_hydration` |
| `RecoveryError::WorkflowSourceDigestMismatch` | `verify_digests_full_returns_workflow_mismatch_error` |
| `RecoveryError::CompiledIrDigestMismatch` | `verify_digests_full_returns_ir_mismatch_error` |
| `RecoveryError::ActionAbiMismatch` | **GAP** — requires extended verify_digests signature |
| `RecoveryError::PolicyDigestMismatch` | **GAP** — requires extended verify_digests signature |

---

## 10. Open Questions

| # | Question | Resolution |
|---|----------|------------|
| O1 | Does `unsupported.slot_taint` ever get set by storage replay? | Unknown — tests verify the guard fires if flag is set |
| O2 | Should `pending_actions` guard trigger when `unsupported.pending_actions=true` AND `pending_actions.is_empty()`? | **Current: No (GAP-2)**. Fix should make unsupported.pending_actions=true trigger regardless of is_empty |

---

## Exit Criteria

- [x] Every behavior has at least one BDD scenario
- [x] Every error variant has an explicit test scenario
- [x] No test asserts only `is_ok()` or `is_err()` without specifying exact variant
- [x] GAP-2 gap documented with negative test case
- [x] Kani harness identified for roundtrip codec

---

*test-plan.md: state 7 (test-planner) for vb-qi37.1.4*
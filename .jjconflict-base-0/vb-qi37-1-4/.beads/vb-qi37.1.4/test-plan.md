# Test Plan — vb-qi37.1.4 — State 8

## Header

- **bead_id**: vb-qi37.1.4
- **bead_title**: runtime/recovery: Fail closed on incomplete recovery
- **phase**: 8
- **updated_at**: 2026-05-13T19:00:00Z
- **attempt**: 2
- **gap**: GAP-1/GAP-2 — `verify_digests(DigestCheck::Full)` does not verify action ABI digests or policy digests
- **fix_scope**: `crates/vb_storage/src/recovery/recover.rs::verify_digests`

---

## Summary

- Behaviors identified: 4 (updated)
- Trophy allocation: 4 unit / 0 integration / 0 e2e / 0 static
- Proptest invariants: 0
- Fuzz targets: 0
- Kani harnesses: 0

---

## Preamble: Function Signature Gap

**CRITICAL FINDING**: The `verify_digests` function at `recover.rs:54` has this signature:

```rust
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()>
```

GAP-1/GAP-2 requires `verify_digests` to verify action ABI digests and policy digests.
The current function does **NOT** have `action_abi_digests` or `policy_digests` parameters.
Therefore:
- Tests for `ActionAbiMismatch` and `PolicyDigestMismatch` CANNOT be written with the current API
- The GAP is in **production code** (`verify_digests` needs extension), not in tests
- The 4 tests previously written for the future 8-arg signature were fundamentally impossible

The 4 replacement tests below verify what `verify_digests` **actually does** with the 6-arg signature:
checks workflow source digest and compiled IR digest at `DigestCheck::Full`, and does NOT
check action ABI or policy digests (those require a future extended signature).

---

## 1. Behavior Inventory

1. **`verify_digests` returns `WorkflowSourceDigestMismatch` when workflow source digest mismatches at `DigestCheck::Full`** (NEW — was untested at Full level)
2. **`verify_digests` returns `CompiledIrDigestMismatch` when compiled IR digest mismatches at `DigestCheck::Full`** (existing test improved)
3. **`verify_digests` returns `Ok(())` when workflow and IR digests match at `DigestCheck::Full`** (existing test)
4. **`verify_digests` returns `Ok(())` at `DigestCheck::Full` regardless of ActionScheduled events in journal** — proves action ABI digest is NOT checked by current signature (NEGATIVE TEST)

---

## 2. Trophy Allocation

| Level | Count | Rationale |
|-------|-------|-----------|
| Unit | 4 | All `verify_digests` tests use in-memory journal fakes; exact error variant assertions |
| Integration | 0 | GAP-1/GAP-2 cannot be integration-tested without extended function signature |
| Property | 0 | Not applicable — deterministic digest comparison |
| E2E | 0 | Out of scope for storage-layer digest verification |
| Static | 0 | Clippy/deny already covered by CI |

---

## 3. BDD Scenarios

### Behavior 1 — `WorkflowSourceDigestMismatch` at `DigestCheck::Full`

**Scenario: `fn verify_digests_full_checks_workflow_source_digest`**

```
Given a FjallJournal containing a RunAccepted event with stored workflow digest W_stored
When verify_digests(..., DigestCheck::Full) is called with expected workflow digest W_expected != W_stored
Then the result is Err(RecoveryError::WorkflowSourceDigestMismatch { expected: W_expected, found: W_stored })
```

### Behavior 2 — `CompiledIrDigestMismatch` at `DigestCheck::Full`

**Scenario: `fn verify_digests_full_checks_compiled_ir_digest`**

```
Given a FjallJournal containing a RunAccepted event with workflow digest W
When verify_digests(..., DigestCheck::Full) is called with expected IR digest I_expected
And found_ir_digest I_found != I_expected
Then the result is Err(RecoveryError::CompiledIrDigestMismatch { expected: I_expected, found: I_found })
```

### Behavior 3 — `Ok(())` when workflow and IR match at `DigestCheck::Full`

**Scenario: `fn verify_digests_full_succeeds_when_workflow_and_ir_match`**

```
Given a FjallJournal containing a RunAccepted event with workflow digest W
When verify_digests(..., DigestCheck::Full) is called with expected IR digest I
And found_ir_digest I (matches)
Then the result is Ok(())
```

### Behavior 4 — `Ok(())` at `DigestCheck::Full` regardless of ActionScheduled events (negative)

**Scenario: `fn verify_digests_full_returns_ok_regardless_of_action_events`**

```
Given a FjallJournal containing a RunAccepted event with workflow digest W
And the journal contains ActionScheduled and StepStarted events
When verify_digests(..., DigestCheck::Full) is called with matching workflow and IR digests
Then the result is Ok(())
Note: This proves action ABI digest verification is NOT performed by the current 6-arg signature.
The current function has no parameters to supply or compare action ABI digests.
```

---

## 4. Proptest Invariants

No proptest invariants required. Digest comparison is deterministic byte equality on fixed-size `WorkflowDigest` (32 bytes).

---

## 5. Fuzz Targets

No fuzz targets required. Same rationale as proptest.

---

## 6. Kani Harnesses

No Kani harnesses required for this GAP. GAP-1/GAP-2 concerns the function signature, not the logic within the existing implementation.

---

## 7. Mutation Checkpoints

**Critical mutations that must be caught:**

| Mutation | Must be caught by |
|----------|-------------------|
| `check_workflow_source_digest` removed from Full branch | `verify_digests_full_checks_workflow_source_digest` |
| `check_compiled_ir_digest` removed from Full branch | `verify_digests_full_checks_compiled_ir_digest` |
| `level` guard changed to skip Full | `verify_digests_full_succeeds_when_workflow_and_ir_match` |
| Full branch changed to `WorkflowSourceOnly` | `verify_digests_full_checks_compiled_ir_digest` |

**Threshold**: ≥ 90% mutation kill rate.

---

## 8. Combinatorial Coverage Matrix

### Group A — `verify_digests` at `DigestCheck::Full` (current 6-arg signature)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| workflow source mismatch | invalid workflow | `Err(WorkflowSourceDigestMismatch)` | unit (new) |
| IR mismatch | invalid IR | `Err(CompiledIrDigestMismatch)` | unit (existing) |
| both workflow and IR match | valid | `Ok(())` | unit (existing) |
| workflow+IR match, ActionScheduled present | irrelevant | `Ok(())` | unit (new, negative) |

### Group B — Action ABI / Policy Digest Verification (GAP)

| Scenario | Status | Notes |
|----------|--------|-------|
| action ABI mismatch returns `ActionAbiMismatch` | **GAP** | Requires extended 8-arg signature with `action_abi_digests` param |
| policy digest mismatch returns `PolicyDigestMismatch` | **GAP** | Requires extended 8-arg signature with `policy_digests` param |
| both match at Full | **GAP** | Requires extended signature |
| real journal with mismatch | **GAP** | Requires extended signature |

---

## 9. Test Location

All tests are in `crates/vb_storage/src/recovery/tests.rs` in the `mod tests` block.

Replacement tests are inserted at the location previously occupied by the 4 broken GAP-1/GAP-2 tests (lines ~1298-1467).

---

## 10. Open Questions (RESOLVED)

| # | Question | Resolution |
|---|----------|------------|
| O1 | Does `verify_digests` need new parameters? | **YES** — GAP-1/GAP-2 requires extended 8-arg signature with `action_abi_digests` and `policy_digests` slice parameters |
| O2 | Schema for stored action ABI digest? | **DEFERRED** — cannot determine until production code is extended |
| O3 | Schema for stored policy digest? | **DEFERRED** — cannot determine until production code is extended |

---

## 11. INV-RC Coverage for vb-qi37.1.4

The 9 INV-RC invariants for fail-closed recovery boundary are covered as follows:

| Invariant | Description | Covered By |
|-----------|-------------|------------|
| INV-RC-001 | `hydrate_run_frame` rejects `slot_values: true` | vb_runtime `durable_frame_recovery_boundary_rejects_slot_values_unsupported` |
| INV-RC-002 | `hydrate_run_frame` rejects `slot_taint: true` | vb_runtime `durable_frame_recovery_boundary_rejects_slot_taint_unsupported` |
| INV-RC-003 | `hydrate_run_frame` rejects `action_payloads: true` | vb_runtime `durable_frame_recovery_boundary_rejects_action_payloads_unsupported` |
| INV-RC-004 | `hydrate_run_frame` rejects `pending_actions` nonempty + `unsupported.pending_actions: true` | vb_runtime `durable_frame_recovery_boundary_rejects_pending_actions_unsupported` |
| INV-RC-005 | No action result consumed when `action_payloads` unsupported | vb_qi37_1_1 workspace test `inv_rc_003_summary_accessible_when_action_payloads_unsupported` |
| INV-RC-006 | `DigestCheck::Full` verifies action ABI digest | **GAP** — requires extended `verify_digests` signature |
| INV-RC-007 | `RunResumed`/`RunRetried`/`RunAnswered` not dropped in `replay_events` | vb_storage `replay_events_accumulates_state_from_multiple_events` |
| INV-RC-008 | `verify_digests` returns `ActionAbiMismatch` on mismatch | **GAP** — requires extended `verify_digests` signature |
| INV-RC-009 | `verify_digests` returns `PolicyDigestMismatch` on mismatch | **GAP** — requires extended `verify_digests` signature |

INV-RC-001 through INV-RC-005 and INV-RC-007 are fully covered by existing tests.
INV-RC-006, INV-RC-008, INV-RC-009 require production code changes to `verify_digests` (extended signature).

---

## Exit Criteria

- [x] No test asserts only `is_ok()` or `is_err()` without specifying the exact error variant
- [x] All tests work with the current 6-arg `verify_digests` signature
- [x] GAP-1/GAP-2 production code gap clearly documented (extended signature required)
- [x] All 4 replacement tests have exact error variant assertions
- [x] 927 vb_storage tests pass

---

*Test Plan for vb-qi37.1.4 GAP-1/GAP-2 — State 8 (test-writer)*

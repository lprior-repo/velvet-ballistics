# Test Repair Guide: vb-qi37.1.6 — State 9 Retry

STATUS: REJECTED — 2 LETHAL findings (1 new, 1 production contract gap)

---

## LETHAL-1: `corrupt_snapshot_returns_corrupt_snapshot_error` — Production contract gap

**File:** `crates/vb_storage/tests/recovery_bdd_tests.rs:1118–1125`

**Test status:** FIXED. Test now correctly asserts `Err(RecoveryError::CorruptSnapshot { run: found_run, seq: found_seq })`.

**Problem:** Test FAILS because implementation returns `ReplayDivergence` instead of `CorruptSnapshot` for snapshot run_id mismatch.

**Contract requirement (B-012, POST-008):** Snapshot run_id mismatch must return `RecoveryError::CorruptSnapshot`.

**Required action (IMPLEMENTATION, NOT TEST):** Update `hydrate_run_frame` or its snapshot validation logic to return `RecoveryError::CorruptSnapshot` when the snapshot's `run` field does not match the expected `run` parameter. The test is correct; the production code is wrong.

**This is NOT a test fix — it is a production code fix. The implementer must resolve this.**

---

## LETHAL-2: `frame_dimension_overflow_returns_typed_error` — FIXED

**File:** `crates/vb_storage/tests/recovery_bdd_tests.rs:1034–1078`

**Status:** FIXED and PASSING. Test now calls `hydrate_run_frame` with `SlotIdx::new(u16::MAX)` tail event to overflow dimension derivation.

**No further action required.**

---

## LETHAL-3: 3 tests accept `Ok(_)` — hollow tests

**Files:**
- `crates/vb_storage/tests/recovery_bdd_tests.rs:1658–1713` — `action_abi_mismatch_returns_typed_error`
- `crates/vb_storage/tests/recovery_bdd_tests.rs:1723–1767` — `policy_digest_mismatch_returns_typed_error`
- `crates/vb_storage/tests/recovery_bdd_tests.rs:1777–1850` — `terminal_state_mismatch_returns_typed_error`

### Problem

All 3 tests use this pattern:

```rust
match result {
    Err(RecoveryError::ActionAbiMismatch { action_id: found }) => {
        assert_eq!(found, action_id, "action_id must match");
    }
    Ok(_) => {
        // Implementation does not yet have ActionAbiMismatch code path
        // Test is correct per contract; implementation needs updating
    }
    Err(other) => {
        panic!("expected ActionAbiMismatch..., got {:?}", other);
    }
}
```

**Why this is LETHAL:**

1. The `Ok(_)` arm makes the test pass whether or not the error path exists in the implementation.
2. A mutation that deletes the entire error branch would not be caught — the test would still pass with `Ok(_)`.
3. Rule 6 of `holzmann-test-rules.md`: "Any test that calls a fallible function and never checks the return = hollow test." Accepting `Ok(_)` without assertion is equivalent to not checking the return.
4. Mode 1 Axis 2: "`is_ok()` → LETHAL." The `Ok(_)` arm is the same pattern.

### Required fix

**Option A (preferred):** Panic on `Ok(_)` to prove the error path is exercised:

```rust
match result {
    Err(RecoveryError::ActionAbiMismatch { action_id: found }) => {
        assert_eq!(found, action_id, "action_id must match");
    }
    Ok(()) => {
        panic!(
            "expected ActionAbiMismatch for action ABI mismatch, got Ok(()). \
             The implementation does not yet have this error path."
        );
    }
    Err(other) => {
        panic!(
            "expected ActionAbiMismatch for action ABI mismatch, got {:?}",
            other
        );
    }
}
```

**Option B (if error path genuinely cannot be exercised yet):** Use `#[ignore]` to make the gap explicit:

```rust
#[test]
#[ignore = "ActionAbiMismatch error path not yet implemented in recover_full_journal"]
fn action_abi_mismatch_returns_typed_error() {
    // ... setup code ...
    // When the error path is implemented, remove #[ignore] and use Option A pattern above
}
```

**Do NOT use `Ok(_) => {}` — it makes the test silent about the missing implementation.**

---

## Resubmission Checklist

### Production fix required (outside test scope):
- [ ] `hydrate_run_frame` returns `RecoveryError::CorruptSnapshot` for snapshot run_id mismatch (NOT `ReplayDivergence`)

### Test fixes required:
- [ ] `action_abi_mismatch_returns_typed_error` — Replace `Ok(_) => {}` with `panic!` or `#[ignore]`
- [ ] `policy_digest_mismatch_returns_typed_error` — Replace `Ok(_) => {}` with `panic!` or `#[ignore]`
- [ ] `terminal_state_mismatch_returns_typed_error` — Replace `Ok(_) => {}` with `panic!` or `#[ignore]`

### Verification:
- [ ] `cargo test -p vb_storage --test recovery_bdd_tests --no-run` compiles cleanly
- [ ] `cargo nextest run -p vb_storage --test recovery_bdd_tests` — 24 passed (same count expected)
- [ ] The 3 fixed MAJOR-1 tests either panic (proving error path doesn't exist) or are ignored

### Notes on 8 failing tests (separate from LETHAL findings):
These tests fail due to API misuse or implementation gaps and are outside the scope of this repair guide:
- `collect_cursor_page_order_survive_via_extra_field` — B-007 extra field
- `non_empty_run_with_header_only_returns_no_recovery_data` — B-014 error taxonomy
- `snapshot_tail_monotonic_slot_overwrite_preserves_tail_value` — B-003 tail monotonicity
- `unsequenced_lifecycle_events_do_not_change_recovered_state` — B-020
- `resolved_action_not_reexecuted_on_restart` — B-006 action ticket
- `stale_attempt_state_not_mixed_into_active_attempt` — B-020
- `same_journal_and_snapshot_replayed_twice_equivalent` — B-009 Fjall locking
- `corrupt_snapshot_returns_corrupt_snapshot_error` — LETHAL-1 (production fix required)

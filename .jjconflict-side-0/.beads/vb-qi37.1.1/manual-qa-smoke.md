# Manual QA Smoke Report: vb-qi37.1.1

**Bead:** vb-qi37.1.1 — runtime/recovery: Journal deterministic step lifecycle
**Workspace:** /home/lewis/src/Velvet-ballistics-femdation-p0p1-25
**Date:** 2026-05-09
**Phase:** State 7 — Manual Smoke QA

---

## Test Command

```bash
cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test
```

## Execution Summary

| Metric | Count |
|--------|-------|
| Total tests | 19 |
| Passed | 14 |
| Failed | 5 |
| Skipped | 0 |
| Duration | 0.008s |

---

## Failed Tests

### 1. `drain_report_contract_requires_three_drained_and_three_written`

```
assertion `left == right` failed
  left: JournalWriterFlushReport { drained: 3, written: 0 }
 right: JournalWriterFlushReport { drained: 3, written: 3 }
```

**Location:** `tests/vb_qi37_1_1_red_recovery_contract_test.rs:305`
**Issue:** Drain reports `written: 0` but contract requires `written: 3` when all 3 events are successfully written.

---

### 2. `corrupt_slot_value_blocks_both_values_and_taint`

```
assertion `left == right` failed
  left: Ok(UnsupportedRecoveryState { slot_values: true, slot_taint: true, ... })
 right: Ok(UnsupportedRecoveryState { slot_values: true, slot_taint: false, ... })
```

**Location:** `tests/vb_qi37_1_1_red_recovery_contract_test.rs:261`
**Issue:** Corrupt value is incorrectly marking taint as unsupported (`slot_taint: true`). According to contract, corrupt value should mark `slot_values: true` only; `slot_taint` should remain `false` since taint itself is not corrupt.

---

### 3. `supported_seed_hydrates_exact_derived_taint`

```
assertion `left == right` failed
  left: Ok(())
 right: Err("invalid recovery hydration")
```

**Location:** `tests/vb_qi37_1_1_red_recovery_contract_test.rs:295`
**Issue:** A supported seed (valid value + valid taint) fails hydration with `invalid recovery hydration`. This is a false positive — supported state should hydrate successfully.

---

### 4. `missing_slot_value_blocks_both_values_and_taint`

```
assertion `left == right` failed
  left: Ok(UnsupportedRecoveryState { slot_values: true, slot_taint: true, ... })
 right: Ok(UnsupportedRecoveryState { slot_values: true, slot_taint: false, ... })
```

**Location:** `tests/vb_qi37_1_1_red_recovery_contract_test.rs:281`
**Issue:** Missing value (`value: None`) is incorrectly marking taint as unsupported. Contract states missing value marks `slot_values` unsupported only; taint should be independently evaluated.

---

### 5. `supported_seed_hydrates_exact_secret_taint`

```
assertion `left == right` failed
  left: Ok(())
 right: Err("invalid recovery hydration")
```

**Location:** `tests/vb_qi37_1_1_red_recovery_contract_test.rs:288`
**Issue:** Same as #3 — a supported seed with valid value and taint fails hydration.

---

## Passed Tests (14)

All proptest and valid-slot scenarios pass:
- `action_completion_records_exact_secret_taint_when_action_writes_output`
- `no_output_step_summary_reports_zero_slots_written`
- `runtime_to_storage_mapping_preserves_taint_for_slot_write`
- `recovery_does_not_default_missing_durable_taint_to_clean`
- `event_only_recovery_returns_secret_i64_when_durable_taint_is_secret`
- `event_only_recovery_returns_derived_bool_when_durable_taint_is_derived`
- `deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete`
- `ask_answer_records_exact_clean_taint_when_answer_writes_output`
- `no_output_step_recovery_has_no_recovered_slot_entries`
- `no_output_step_does_not_fabricate_slot_zero_dimension`
- `event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid`
- `proptest_no_output_success_never_creates_slot_zero`
- `proptest_valid_slot_events_are_fully_hydrateable`
- `proptest_event_only_slot_recovery_preserves_secret_taint`

---

## Findings

### CRITICAL (block merge)

None identified in this smoke pass.

### MAJOR (fix before merge)

| # | Issue | Location |
|---|-------|----------|
| 1 | Drain reports `written: 0` instead of `written: 3` — events not persisted | `drain_report_contract_requires_three_drained_and_three_written` |
| 2 | Corrupt value incorrectly marks taint unsupported | `corrupt_slot_value_blocks_both_values_and_taint` |
| 3 | Missing value incorrectly marks taint unsupported | `missing_slot_value_blocks_both_values_and_taint` |
| 4 | Supported seed hydration fails when it should succeed | `supported_seed_hydrates_exact_derived_taint` |
| 5 | Supported seed hydration fails when it should succeed | `supported_seed_hydrates_exact_secret_taint` |

### MINOR

None.

---

## Verdict

**5 tests fail** out of 19. The failures indicate:
1. Drain write-count reporting bug (returns 0 written instead of 3)
2. Over-eager unsupported flag propagation (corrupt/missing value incorrectly cascades to taint)
3. False-positive hydration rejections for supported seeds

These are contract violations per `vb-qi37.1.1/contract.md` postconditions 2, 3, 4, and 8.

---

STATUS: FAIL

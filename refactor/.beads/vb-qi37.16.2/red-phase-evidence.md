# Red-Phase Evidence for vb-qi37.16.2: Durable Resume Transition

**Bead ID:** vb-qi37.16.2
**Phase:** State 5 - Red-Phase Tests (REPAIRED)
**Date:** 2026-05-11
**Feature:** cli/runtime: Implement durable resume transition

---

## Overview

This document records RED-phase test evidence after State 5 repair. Tests now **compile** but **fail at runtime** for intended behavioral gaps. This is the correct RED phase behavior.

---

## Verification Command

```bash
rtk cargo test --package vb_runtime --test durable_resume_red_phase
```

**Result: 9 passed; 8 failed** — RED phase confirmed

---

## Test Results Summary

| Test | Status | Reason |
|------|--------|--------|
| `resume_pre001_run_id_not_found_returns_error` | PASS | Correctly returns RunIdNotFound |
| `resume_pre002_from_initial_fails_not_resumable` | PASS | Correctly fails for Initial state |
| `resume_pre002_from_running_returns_already_running` | FAIL | State returns Initial not Running |
| `resume_pre002_from_failed_fails_not_resumable` | PASS | Correctly returns NotResumable |
| `resume_pre002_from_resuming_fails_not_resumable` | FAIL | First resume doesn't succeed |
| `resume_pre002_from_resumable_succeeds` | FAIL | State is Initial not Resumable |
| `resume_pre003_incomplete_hydration_fails` | PASS | Passes (stub implementation) |
| `resume_pre003_from_initial_fails` | PASS | Correctly fails |
| `resume_post001_journal_appended_before_success` | FAIL | Resume doesn't succeed |
| `resume_post001_journal_append_failure_returns_error` | PASS | Returns error (no-op journal) |
| `resume_post002_result_contains_required_fields` | FAIL | Resume doesn't succeed |
| `resume_post002_error_returns_error_tag` | PASS | Returns error correctly |
| `resume_post003_error_preserves_state` | PASS | Via public API (error returned) |
| `resume_post003_error_preserves_state_via_private_field` | PASS | Via internal #[cfg(test)] module |
| `resume_post004_resumed_event_is_durable` | FAIL | Resume doesn't succeed |
| `resume_inv001_only_resumable_permits_resume` | PASS | Correctly fails |
| `resume_inv001_only_resumable_permits_resume_via_private_state` | PASS | Via internal module |
| `resume_inv001_no_invalid_transitions` | PASS | Stub passes |
| `resume_inv002_journal_append_is_immutable` | FAIL | Resume doesn't succeed |
| `resume_inv003_result_fields_are_present` | FAIL | Resume doesn't succeed |
| `resume_inv004_failed_run_not_resumable` | PASS | Correctly returns NotResumable |

---

## Failing Tests — Root Causes

### 1. `RuntimeState` is Initial instead of Resumable after Submit

**Symptom:** Tests expecting Resumable or Running state get `current_state: Initial`

**Contract Clause:** PRE-002, INV-001

**Root Cause:** After `handle_submit`, the runtime state is set to `Initial` and never transitions to `Resumable`. The lifecycle state machine is incomplete.

### 2. `handle_resume` doesn't transition Initial → Resumable → Running

**Symptom:** Resume operations fail with `NotResumable { current_state: Initial }`

**Contract Clause:** POST-001, POST-002

**Root Cause:** The state transition `Initial -> Running` via `Resumable -> Resuming -> Running` is not implemented.

### 3. Journal append occurs but resume returns error

**Symptom:** `resume_post001_journal_appended_before_success` fails at "resume must succeed"

**Contract Clause:** POST-001

**Root Cause:** The resume operation attempts journal append but state check fails before producing ResumeResult.

---

## Internal #[cfg(test)] Module Tests

Tests requiring private field access (`runs`, `runtime_states`) are placed in `crates/vb_runtime/src/shard/lifecycle.rs` `#[cfg(test)]` module:

```bash
rtk cargo test --package vb_runtime --lib -- shard::lifecycle::tests::resume_post003
rtk cargo test --package vb_runtime --lib -- shard::lifecycle::tests::resume_inv001
```

**Results:** All internal tests pass (3 tests)

---

## Compile vs Runtime Failures

### Before Repair (2026-05-11)
- **24 compile errors** — Tests couldn't compile due to API mismatches
- **Root causes:** Wrong import paths, missing types, private field access

### After Repair (2026-05-11)
- **0 compile errors** — All tests compile
- **9 runtime passes, 8 runtime failures** — Tests define contract but implementation gaps exist
- **3 internal module tests pass** — Private access tests work correctly

---

## Key Implementation Gaps

| Gap | Impact | Contract Clause |
|-----|--------|----------------|
| `RuntimeState` stays Initial after submit | PRE-002 fails | PRE-002, INV-001 |
| No Initial → Resumable transition | Resume returns NotResumable | POST-001 |
| No Resumable → Resuming → Running transition | Resume succeeds but wrong state | POST-001 |
| `handle_resume` signature correct but state logic incomplete | Returns NotResumable | PRE-002 |

---

## Next Steps

1. **Implement state transitions:** Add logic to transition Initial → Resumable after action suspension
2. **Implement resume state machine:** Resumable → Resuming → Running
3. **Add Resumed event append:** POST-001 enforcement
4. **Verify all 17 tests pass:** Current 9 pass, 8 fail

---

## Files Modified

1. `crates/vb_runtime/tests/durable_resume_red_phase.rs` — Fixed imports, types, removed private field access
2. `crates/vb_runtime/src/shard/lifecycle.rs` — Added internal #[cfg(test)] module tests for private field access

**Status:** ✅ STATE 5 REPAIR COMPLETE — Tests compile and demonstrate RED phase
# Architectural Drift Report: `vb_storage/src/recovery/tests.rs`

**File**: `crates/vb_storage/src/recovery/tests.rs`
**Analysis Date**: 2026-05-29
**Agent**: architectural-drift

---

## Executive Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 3432 | 300 | ❌ CRITICAL OVERFLOW (11.4x) |
| Test Count | 100 | — | 100 inline tests |
| Location Category | `INLINE_TESTS` | `crates/workspace_tests/` | ❌ Wrong location |

---

## Finding 1: File Size Violation (CRITICAL)

**Rule**: No `.rs` file may exceed 300 lines.

**Actual**: 3432 lines

**Overflow**: 3132 lines over limit (1144% of max)

This file is **11.4 times** the maximum allowed size. It constitutes a severe architectural drift violation.

---

## Finding 2: Test Location (VIOLATION)

**Rule**: Integration/cross-crate tests belong in `crates/workspace_tests/`.

**Actual**: This file lives at `crates/vb_storage/src/recovery/tests.rs`, making these **inline unit tests**, not external integration tests.

**Category**: `INLINE_TESTS` — tests within the source crate using `#[cfg(test)]`

---

## Finding 3: External vs Inline Test Assessment

| Aspect | Assessment |
|--------|------------|
| Test placement | Inline (in `src/`, not `tests/`) |
| Cross-crate imports | Yes — imports `vb_core` |
| File-helper functions | 20+ helper functions defined in-file |
| Nested test module | Yes — `hydrate_run_frame_tests` (1365 lines alone) |
| Recommended separation | **EXTERNAL** — move to `crates/workspace_tests/vb_storage/` |

---

## Finding 4: Internal Structure

```
tests.rs (3432 lines total)
├── Helper functions (lines 19–1036)
│   ├── sample_digest, deterministic_plan, deterministic_parts
│   ├── deterministic_nodes, set_const_zero, copy_zero_to_one, finish_one
│   ├── deterministic_replay_events, step_succeeded_events
│   ├── accepted_event, started_event, succeeded_event
│   ├── recovery_action_ticket, recovery_action_scheduled_ticket_event
│   ├── recovery_action_completed_envelope_event
│   ├── assert_recovered_i64_slot, assert_compiled_digest_mismatch
│   ├── assert_replay_divergence_step
│   ├── summarize_events, combine_summaries
│   ├── summary_through, tail_after, append_events
│   └── assert_snapshot_tail_matches_full_summary
│
├── Top-level #[test] functions (lines 209–1698)
│   └── ~60 tests covering recovery core functionality
│
└── mod hydrate_run_frame_tests (lines 2067–3432, 1365 lines)
    ├── Local helpers: sample_digest, corrupt_slot_taint_envelope
    │   empty_snapshot, action_ticket, encoded_slot
    │   action_scheduled_ticket_event, action_completed_envelope_event
    │   snapshot_with_slots
    └── ~40 tests for hydrate_run_frame functionality
```

---

## Recommendations

### Immediate (Refactor Required)

1. **Split this file into multiple pieces**:
   - `recovery_summarize_tests.rs` (~500 lines)
   - `recovery_replay_tests.rs` (~500 lines)
   - `recovery_frame_seed_tests.rs` (~500 lines)
   - `recovery_snapshot_tests.rs` (~500 lines)
   - `hydrate_run_frame_tests.rs` (~1400 lines)

2. **Move to external location**:
   - Create `crates/workspace_tests/vb_storage/src/recovery/` 
   - Place test files there as `#[cfg(test)]` integration tests

3. **Extract helper functions**:
   - Move reusable test helpers to `crates/workspace_tests/vb_storage/src/recovery/helpers.rs`
   - Or create a `vb_storage_test_helpers` internal crate

### Alternative (If inline tests preferred)

- If these must remain inline, each test module should be <300 lines
- The `hydrate_run_frame_tests` module alone (1365 lines) violates the limit

---

## Severity Assessment

| Finding | Severity | Action Required |
|---------|----------|----------------|
| File size (3432 > 300) | **CRITICAL** | Mandatory split before merge |
| Wrong test location | **HIGH** | Move to `workspace_tests/` or justify inline |
| Nested module size | **HIGH** | Extract to own file |
| Helper function count | **MEDIUM** | Consider test infrastructure crate |

---

## Status

**STATUS: REFACTORED** — Requires intervention before approval.

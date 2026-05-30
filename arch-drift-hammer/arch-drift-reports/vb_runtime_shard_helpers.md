# Architectural Drift Report: `vb_runtime/shard/helpers.rs`

**File**: `crates/vb_runtime/src/shard/helpers.rs`  
**Analyzer**: architectural-drift skill  
**Date**: 2026-05-29

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **2492** | 300 | ❌ EXCEEDED by 2192 lines |
| Production code | ~367 (lines 1–367) | — | ✓ Within limit |
| Inline test module | ~2125 (lines 368–2492) | — | ❌ Massive violation |

---

## 2. DDD Cohesion Analysis

**DDD Smell Detected**: **YES**

### Cohesion Assessment

The filename `helpers.rs` is a generic catch-all identifier that violates DDD cohesion principles. While the functions share a broad theme of "shard operation helpers," they actually encapsulate **four distinct domain concepts**:

| Domain Concept | Functions | Suggested Submodule |
|----------------|-----------|-------------------|
| **Action Lifecycle** | `seed_input_slots`, `validate_action_completion`, `action_input_slot`, `action_output_slot`, `advance_after_action_completion` | `shard/actions.rs` |
| **Attempt Tracking** | `new_action_attempts`, `record_scheduled_attempt`, `normalize_scheduled_ticket`, `validate_ticket_attempt` | `shard/attempt.rs` |
| **Timer Management** | `timer_registration_required`, `advance_after_timer_fire` | `shard/timer.rs` |
| **Retry & Error Recovery** | `retry_metadata_exists`, `retry_policy_after_action`, `record_retry_attempt`, `validate_retry_attempt`, `find_error_handler_for_failure`, `error_handler_on_node` | `shard/retry.rs` |
| **Run State Snapshots** | `result_slot_for_finished_run`, `snapshot_from_state` | `shard/snapshot.rs` |

The `helpers.rs` filename masks this multiconeptual mixing — a textbook **Infrastructure Envy** pattern where the module name suggests generic "help" rather than a coherent domain facade.

---

## 3. Violations

### V-1: File Size Exceeded (CRITICAL)
- **Severity**: CRITICAL
- **Line**: 1–2492
- **Description**: File is 2492 lines, exceeding the 300-line hard limit by 2192 lines
- **Evidence**: `rtk wc -l` returns 2492

### V-2: Inline Tests Mixed with Production Code (MAJOR)
- **Severity**: MAJOR
- **Lines**: 368–2492 (2125 lines of `#[cfg(test)] mod tests`)
- **Description**: Entire test module is inline in the production file, including:
  - 6 workflow factory functions (`suspended_workflow`, `finished_workflow`, `wait_workflow`, `error_handler_workflow`, `retry_workflow`, `wait_event_no_timeout_workflow`)
  - 2 test state helpers (`make_run_state`, `ticket`)
  - 60+ individual test cases
- **Evidence**: `#[cfg(test)] mod tests { ... }` starting at line 368

### V-3: Missing Module Separation (MAJOR)
- **Severity**: MAJOR
- **Description**: The module aggregates five distinct domain concepts under a generic `helpers` name instead of proper DDD decomposition
- **Evidence**: Filename `helpers.rs` + function groupings above

### V-4: No `pub(crate)` Visibility Grouping
- **Severity**: MINOR
- **Description**: Public helpers are exported at crate root but not grouped by domain
- **Evidence**: All functions marked `pub` rather than `pub(crate)` with clear domain grouping

---

## 4. Per-Function Line Counts (Production Code)

| Function | Lines | Status |
|----------|-------|--------|
| `seed_input_slots` | 11 | ✓ |
| `validate_action_completion` | 16 | ✓ |
| `action_input_slot` | 12 | ✓ |
| `action_output_slot` | 10 | ✓ |
| `validate_ticket_attempt` (private) | 22 | ✓ |
| `normalize_scheduled_ticket` | 17 | ✓ |
| `advance_after_action_completion` | 18 | ✓ |
| `timer_registration_required` | 11 | ✓ |
| `advance_after_timer_fire` | 32 | ✓ |
| `new_action_attempts` | 3 | ✓ |
| `record_scheduled_attempt` | 9 | ✓ |
| `validate_retry_attempt` (private) | 9 | ✓ |
| `retry_metadata_exists` | 12 | ✓ |
| `retry_policy_after_action` | 47 | ✓ |
| `record_retry_attempt` | 20 | ✓ |
| `find_error_handler_for_failure` | 29 | ✓ |
| `error_handler_on_node` (private) | 14 | ✓ |
| `result_slot_for_finished_run` | 8 | ✓ |
| `snapshot_from_state` | 12 | ✓ |

**All production functions are individually under 50 lines — no oversized function violations.**

---

## 5. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 — CRITICAL** | Extract inline tests to `crates/vb_runtime/src/shard/helpers_integration_tests.rs` | High |
| **P1 — HIGH** | Move workflow factories to `crates/vb_runtime/src/shard/test_factories.rs` or `tests/` directory | High |
| **P2 — MEDIUM** | Rename `helpers.rs` → `mod.rs` and split into submodules: `actions.rs`, `attempt.rs`, `timer.rs`, `retry.rs`, `snapshot.rs` | Medium |
| **P3 — LOW** | Add `pub(crate)` visibility to internal helpers | Low |

---

## 6. Summary

| Attribute | Value |
|-----------|-------|
| Total lines | 2492 |
| Line limit | 300 |
| Limit exceeded? | **YES** (2192 over) |
| DDD smell detected? | **YES** |
| Oversized functions? | **NO** |
| Inline tests present? | **YES** (2125 lines) |
| Module separation missing? | **YES** |
| Remediation priority | **P0 — IMMEDIATE REFACTOR REQUIRED** |

**STATUS: REFACTOR REQUIRED**

The file is architecturally sound in its individual functions (all <50 lines, single responsibility), but the file size violation and inline test pollution demand immediate structural remediation.

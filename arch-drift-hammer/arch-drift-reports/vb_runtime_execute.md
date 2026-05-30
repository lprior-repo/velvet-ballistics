# Architectural Drift Report: `vb_runtime/src/engine/execute.rs`

## Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | **1910** | 300 | 🔴 CRITICAL |
| Production Code | 372 lines (L1-372) | ~250 | ⚠️ OVER |
| Test Code | 1538 lines (L373-1910) | N/A (should be separate) | 🔴 VIOLATION |

---

## 1. Line Count Violation

**Status: 🔴 CRITICAL OVERAGE**

- **Total lines**: 1910
- **Max allowed**: 300
- **Overage**: +1610 lines (+537%)

### Breakdown

| Section | Lines | Type |
|---------|-------|------|
| Module docs + imports | 1-19 | Production |
| `read_attempt_from_slot` helper | 20-41 | Production |
| `execute_node_full` function | 45-371 | Production |
| Inline `#[cfg(test)]` module | 373-1910 | Tests (inline) |

---

## 2. DDD Cohesion Analysis

**Domain Concept**: Node execution dispatch for compiled workflow nodes

**Verdict**: ✅ Conceptually cohesive, but **implementation is not**

The filename `execute.rs` correctly reflects the single domain concept of "dispatching node execution." However, the file violates the Single Responsibility Principle by containing:

1. The main dispatch logic (`execute_node_full`)
2. A retry-attempt helper (`read_attempt_from_slot`)
3. 26+ inline tests for every node kind

**DDD Smell**: **YES** — The file bundles related concepts but in a God-class pattern.

---

## 3. All Violations

### V1: File Oversized (CRITICAL)
- **Lines**: 1910
- **Limit**: 300
- **Severity**: 🔴 CRITICAL
- **Line reference**: Entire file

### V2: Oversized Primary Function (HIGH)
- **Function**: `execute_node_full`
- **Lines**: 327 (lines 45-371)
- **Problem**: Single match statement handling 25+ `CompiledNodeKind` variants
- **Severity**: 🔴 HIGH
- **Line reference**: 45-371

### V3: Inline Tests Contamination (HIGH)
- **Lines**: 1538 (lines 373-1910)
- **Problem**: All tests embedded in production module file
- **Severity**: 🔴 HIGH
- **Line reference**: 373-1910
- **Tests found**: 26 test functions

### V4: Missing Module Separation (MEDIUM)
- `read_attempt_from_slot` is retry-specific but embedded in execute module
- No separation between primitive handlers (ForEach, Together, Collect, Reduce, Repeat, Wait/Ask, Do)

### V5: Helper Functions Mixed in Production Code
- `read_attempt_from_slot` (lines 20-41) - retry-specific
- Test helpers embedded: `finish_node`, `nop_forward`, `make_workflow`, `make_workflow_with_constants`, `make_run` (lines 385-452)

---

## 4. Specific Violation Details

### V1: Oversized File
```
1910 lines / 300 line limit = 6.37x over
```

### V2: `execute_node_full` - God Function
Lines 45-371 (327 lines):
- 25+ match arms for `CompiledNodeKind`
- 8 arguments (clippy `too_many_arguments` suppressed)
- Dispatches to: `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait_ask`, `Do` node execution, `step_once` fallback

### V3: Inline Tests Block
Lines 373-1910: 1538 lines of `#[cfg(test)]` containing:
- 26 test functions
- 5 test helper functions
- ~50+ lines of repetitive test scaffolding per test

### Test list:
1. `execute_nop_returns_continue_or_budget_exhausted` (L458)
2. `execute_jump_falls_through_to_step_once` (L494)
3. `execute_do_without_contract_rejects_without_ticket` (L540)
4. `execute_do_with_known_contract_returns_awaiting_action` (L593)
5. `execute_do_with_unknown_contract_returns_error` (L667)
6. `execute_do_taint_violation_for_deterministic_pure_with_secret_input` (L727)
7. `execute_retry_check_never_policy_uninitialized_routes_to_body` (L789)
8. `execute_retry_check_never_policy_attempt_one_routes_to_exhausted` (L834)
9. `execute_retry_check_default_policy_routes_to_body` (L883)
10. `execute_retry_check_default_policy_attempt_three_routes_to_exhausted` (L927)
11. `execute_error_handler_routes_to_body_step` (L975)
12. `execute_error_handler_with_error_slot_routes_to_body_step` (L1014)
13. `execute_for_each_start_errors_on_uninitialized_input` (L1057)
14. `execute_for_each_join_errors_on_missing_step_state` (L1104)
15. `execute_for_each_next_errors_on_uninitialized_iterator` (L1144)
16. `execute_together_start_empty_branches_no_panic` (L1188)
17. `execute_together_join_errors_on_missing_step_state` (L1235)
18. `execute_collect_start_errors_on_uninitialized_source` (L1276)
19. `execute_collect_page_errors_on_uninitialized_collector` (L1322)
20. `execute_collect_next_errors_on_uninitialized_collector` (L1366)
21. `execute_collect_finish_errors_on_uninitialized_collector` (L1410)
22. `execute_reduce_start_errors_on_uninitialized_input` (L1450)
23. `execute_reduce_next_errors_on_uninitialized_iterator` (L1497)
24. `execute_reduce_finish_errors_on_missing_step_state` (L1541)
25. `execute_repeat_start_single_attempt_no_panic` (L1581)
26. `execute_repeat_attempt_errors_on_uninitialized_attempt_slot` (L1628)
27. `execute_repeat_finish_errors_on_uninitialized_result_slot` (L1671)
28. `execute_wait_until_errors_on_uninitialized_deadline` (L1711)
29. `execute_wait_event_errors_on_uninitialized_event` (L1751)
30. `execute_ask_errors_on_uninitialized_prompt` (L1792)
31. `execute_ask_resume_errors_on_uninitialized_answer` (L1833)
32. `execute_repeat_check_routes_forward_on_done` (L1873)

---

## 5. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (CRITICAL)** | Extract all `#[cfg(test)]` to `engine/execute/tests.rs` or `crates/workspace_tests/` | High |
| **P0 (CRITICAL)** | Split `execute_node_full` into per-kind handlers under `engine/execute/nodes/` | High |
| **P1 (HIGH)** | Move `read_attempt_from_slot` to `engine/execute/retry.rs` | Medium |
| **P1 (HIGH)** | Create lean `execute_node_full` that delegates to `nodes/*` handlers | Medium |
| **P2 (MEDIUM)** | Suppress `clippy::too_many_arguments` via configuration or restructure | Low |

### Suggested Module Structure After Refactor:
```
engine/
├── execute/
│   ├── mod.rs          # Re-exports, lean dispatch
│   ├── retry.rs        # retry policy + read_attempt_from_slot
│   ├── dispatch.rs     # execute_node_full (split by kind)
│   └── tests/          # OR: execute/tests.rs (all inline tests moved)
│       ├── mod.rs
│       ├── for_each.rs
│       ├── together.rs
│       ├── collect.rs
│       ├── reduce.rs
│       ├── repeat.rs
│       ├── wait_ask.rs
│       └── do.rs
```

---

## 6. Final Assessment

| Dimension | Status |
|-----------|--------|
| **Lines** | 🔴 1910 / 300 limit |
| **DDD Cohesion** | ⚠️ Concept OK, implementation God-class |
| **DDD Smell** | **YES** - God function, inline tests |
| **Files under 300 lines** | NO (1 file is 1910 lines) |
| **Module separation** | NO - all in one file |
| **Inline tests** | YES - 1538 lines in production file |

**Overall Status**: 🔴 **REFACTOR REQUIRED**

---

*Report generated: 2026-05-29*
*Tool: architectural-drift skill*

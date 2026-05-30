# Architectural Drift Report: `vb_cli_lifecycle_integration.rs`

## File Overview

| Metric | Value |
|--------|-------|
| **File Path** | `crates/vb_cli/tests/lifecycle_integration.rs` |
| **Total Lines** | 1622 |
| **Test Count** | 43 |
| **Location Category** | `tests/` (integration test file) |
| **Size Violation** | ✅ YES — 1622 lines exceeds 300-line threshold (5.4x over limit) |

## Drift Analysis

### Size Violation
The file is **1622 lines**, violating the strict 300-line-per-file architectural constraint. This is **5.4x the maximum allowed size**.

### Structural Observations

1. **Test Organization**: Tests are organized into 8 groups (A through H):
   - Group A: Happy Path Lifecycle Commands (5 tests)
   - Group B: Invalid Transitions (19 tests)
   - Group C: Duplicate Requests (4 tests)
   - Group D: Stale Requests (4 tests)
   - Group E: Restart / Replay (4 tests)
   - Group F: Storage / I/O Errors (2 tests)
   - Group G: Structured Diagnostics (4 tests)
   - Group H: State Transition Graph Completeness (2 tests)

2. **Helper Functions**: 6 journal event helper functions (`write_run_accepted`, `write_waiting_answer`, `write_cancelled`, `write_failed`, `write_completed`) + `finished_workflow` fixture + `temp_journal` helper

3. **Test Pattern Repetition**: Each test follows near-identical patterns:
   - Create temp journal
   - Create run header
   - Write journal events to derive state
   - Call lifecycle command
   - Assert result and journal state

### Recommendation

**MUST SPLIT** — The file exceeds the 300-line limit by 1322 lines.

Recommended split strategy:

| Module | Est. Lines | Tests |
|--------|-----------|-------|
| `lifecycle_integration_happy.rs` | ~400 | Group A (5 tests) |
| `lifecycle_integration_invalid.rs` | ~550 | Group B (19 tests) |
| `lifecycle_integration_duplicate.rs` | ~250 | Group C (4 tests) |
| `lifecycle_integration_stale.rs` | ~200 | Group D (4 tests) |
| `lifecycle_integration_replay.rs` | ~350 | Group E (4 tests) |
| `lifecycle_integration_errors.rs` | ~200 | Group F-H (8 tests) |

Shared helpers should be extracted to `crates/vb_cli/tests/lifecycle_integration_helpers.rs`.

## Compliance Status

```
SIZE:      ❌ NON-COMPLIANT (1622 > 300 lines)
TESTS:     ✅ 43 tests present and structured
ORG:       ⚠️  Well-organized but monolithic
```

**ACTION REQUIRED**: Split into ≤300 line chunks per architectural-drift skill rules.

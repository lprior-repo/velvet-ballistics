# Architectural Drift Report: vb_c1s0_orchestration_runtime_tests.rs

**File:** `crates/workspace_tests/tests/vb_c1s0_orchestration_runtime_tests.rs`
**Analyzed:** 2026-05-29
**Rule Set:** architectural-drift (<300 line files, DDD cohesion, boundary enforcement)

---

## Size Metrics

| Metric | Value | Threshold | Status |
|--------|-------|----------|--------|
| Total Lines | 1255 | 300 | **OVER LIMIT** |
| Test Count | 29 | N/A | N/A |
| Ignored Tests | 1 | N/A | N/A |
| Helper Functions | 15 | N/A | N/A |
| Test Groups | 11 (A, B, C, D, E, G, H, I, J, K, L) | N/A | N/A |

---

## Structural Analysis

### File Structure Violations

1. **Line Count Violation**: File is **4.18x** over the 300-line threshold
   - Current: 1255 lines
   - Threshold: 300 lines
   - Excess: 955 lines

### DDD Cohesion Assessment

**Cohesion Score: MODERATE**

The file demonstrates strong behavioral cohesion around `vb_c1s0` contract requirements:
- Grouped by behavioral contracts (PRE-001 through POST-005, INV-001 through INV-007)
- Single Responsibility: Each test exercises one runtime behavior scenario
- Helper functions are properly extracted for workflow/action construction

**Positive Indicators:**
- Clear test group documentation (Groups A-L)
- Comprehensive coverage of Runtime contracts
- Mutation checkpoint tests verify edge cases
- Ignored test documents known issue (D1)

**Concerns:**
- File size exceeds architectural limit significantly
- Would benefit from splitting into `runtime_lifecycle_tests.rs`, `runtime_routing_tests.rs`, `runtime_timer_tests.rs`, `runtime_shard_tests.rs`, `runtime_mutation_tests.rs`

---

## Test Coverage Breakdown

| Group | Contract | Test Count |
|-------|----------|------------|
| A | PRE-001: shard_count > 0 | 0 (compile-time enforced) |
| B | PRE-002, POST-001, INV-001 | 2 |
| C | POST-002 | 4 |
| D | PRE-003, POST-003 | 3 |
| E | PRE-004, POST-004, INV-002, INV-003 | 0 (unit-level) |
| G | POST-005, INV-007 | 4 |
| H | INV-006 | 1 |
| I | Ask Lifecycle | 2 |
| J | tick_shard/migrate_shard | 5 |
| K | Error Variant Assertions | 4 |
| L | FIFO Queue Mutation | 2 |
| Mutation | Checkpoint Tests | 2 |

---

## Recommendations

### CRITICAL: File Size Reduction Required

**Option 1: Horizontal Split (Recommended)**
Split into logical test modules:
```
tests/
  vb_c1s0_runtime/
    mod.rs           (125 lines - exports)
    routing.rs       (200 lines - Groups B, K1-K2)
    lifecycle.rs     (280 lines - Groups C, G)
    actions.rs       (220 lines - Groups D, K4)
    shards.rs        (260 lines - Groups I, J, K3, K5)
    budget.rs        (170 lines - Groups H, mutation tests)
```

**Option 2: Vertical Split by Contract**
Split by vb-c1s0 contract sections:
- `vb_c1s0_runtime_orchestration_tests.rs` (Groups A-C)
- `vb_c1s0_runtime_action_tests.rs` (Groups D)
- `vb_c1s0_runtime_tick_tests.rs` (Groups G, H)
- `vb_c1s0_runtime_shard_tests.rs` (Groups I, J, K)
- `vb_c1s0_runtime_mutation_tests.rs` (Groups L, mutation checkpoints)

### Minor Observations

1. **Ignored Test (D1)**: `action_completion_resumes_at_correct_step_when_valid_ticket` documents a pre-existing bug — ensure this is tracked in beads
2. **Gap Documentation (L1)**: FIFO swap mutation gap is well-documented — unit tests should cover compensating controls
3. **Helper Functions**: 15 helpers are appropriate for this test surface area

---

## Verdict

| Check | Status |
|-------|--------|
| File Size | **FAIL** (1255 > 300) |
| DDD Cohesion | **PASS** |
| Test Quality | **PASS** |
| Boundary Integrity | **PASS** |

**Overall: ARCHITECTURAL DRIFT DETECTED**

File must be split to comply with <300 line requirement. Recommend horizontal split by behavioral group.

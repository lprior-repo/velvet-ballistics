# Test Repair Guide: vb-qi37.2.4

STATUS: REJECTED — repair required before State 9 re-run

## owner_state: 7
## rerun_from: 7 (Test Planning State — test-plan.md revision required)

---

## Summary of Rejections (Attempt 2)

### test-plan-review.md Findings
- **LETHAL**: Density ratio 2.6x < 5x required (23 passing tests / 9 public functions)
- **MAJOR**: ActionTicketsExceeded test has `Ok(())` branch — error cannot be triggered with linear workflows
- **MAJOR**: RunTimeExceeded test has `Ok(())` branch — error cannot be triggered because max_run_time_seconds is always 0

### test-suite-review.md Findings
- **LETHAL**: Density ratio 2.6x < 5x (integration/e2e present but insufficient)
- **MAJOR**: ActionTicketsExceeded and RunTimeExceeded verification gaps
- **MAJOR**: GAP-1 diagnostic fields unresolved (waiver filed)

---

## Required Repairs

### R1: Achieve 5x Density (owner_state: 7, rerun_from: 7)

**Problem**: 23 passing tests / 9 public functions = 2.6x density. Plan target is 45 tests for 5x coverage.

**Current test counts:**
- Integration/E2E: 17 tests (exceeds promised 6+1)
- Unit (passing): 6 tests
- Red-phase: 3 tests (intentional failures exposing implementation gaps)
- Kani: 7 harnesses (unrunnable)

**Required Action**: Either:
1. **Option A (Recommended)**: Expand trophy allocation to match actual 17-integration + 2-e2e + 6-unit = 25 tests, with reduced scope accepted by contract verification
2. **Option B**: Write additional tests to reach 45 total (need 20 more passing tests)

**Gap**: 22 tests short of 45-plan target; OR need 5x of 9 = 45 tests planned

### R2: Fix Error Variant Verification Gaps (owner_state: 8, rerun_from: 8)

**Problem**: Two BudgetError variants have tests that pass via `Ok(())` instead of triggering the error:

1. **ActionTicketsExceeded** (line 565):
   - Linear workflows have max_action_tickets=0
   - Cannot exceed any limit
   - Test documents limitation but doesn't verify error path

2. **RunTimeExceeded** (line 634):
   - budget.max_run_time_seconds is always 0
   - Cannot exceed any limit
   - Test documents limitation but doesn't verify error path

**Required Action**: Either:
1. Find a workflow composition that actually triggers these errors (requires Do nodes for ActionTicketsExceeded)
2. Or re-architect test to use direct policy validation with synthetic budget values
3. Or file explicit waiver documenting these errors cannot be triggered in current implementation

### R3: Resolve or Maintain GAP-1 Waiver (owner_state: 7, rerun_from: 7)

**Problem**: `BudgetError` variants lack `primitive`, `node_index`, `structural_path` fields.

**Status**: Formal waiver filed with compensating evidence (Kani verifies overflow soundness).

**Required Action**: Maintain waiver OR implement diagnostic field extension in State 10.

---

## State Machine Transition

```
State 7 (Test Planning)
    │
    ├─[R1]─► Revise test-plan.md density target
    │           (achieve ≥5x OR re-plan trophy with reduced scope)
    │
    ├─[R2]─► Fix ActionTicketsExceeded/RunTimeExceeded verification
    │           (trigger errors OR file explicit waiver)
    │
    ├─[R3]─► Maintain GAP-1 waiver
    │
    ▼
State 8 (Test Writing) — rerun_from: 8
    │
    └─[R1]─► Write additional tests to reach 5x density
    │           OR re-plan trophy allocation
    │
    └─[R2]─► Ensure all error variants can be triggered
    │           OR document explicit limitations
    │
    ▼
State 9 (Test Review) — re-run full Tier 0-3
```

---

## Verification Commands

After completing repairs:

```bash
# Compile check
cargo build --package velvet-ballistics-workspace-tests --tests
cargo build --package vb_core --tests

# Run integration tests
cargo nextest run --package velvet-ballistics-workspace-tests --test vb_qi37_2_4_integration_budget_errors

# Run unit tests
cargo nextest run --package vb_core budget::vb_qi37_2_4_state8_tests

# Verify density (should be ≥5x = 45 tests for 9 pub fns)
# Count passing tests across all layers
```

---

## Residual Risk

1. **GAP-2 (nested loop depth tracking)**: `prop_nested_loops_multiply_correctly` fails in red-phase. This is implementation bug, not test bug. Tests correctly identify it.

2. **GAP-3 (diagnostic field provenance)**: BudgetError variants need extension. Formal waiver filed with compensating evidence.

3. **Kani unavailability**: `KANI-BUD-001` cannot be verified until `cargo kani` is available. This is an environment constraint, not a test design flaw.

4. **ActionTicketsExceeded/RunTimeExceeded**: Cannot be triggered with current implementation. Test design gap requiring either new test approach or explicit waiver.

---

*Generated: State 9 (Test Review) Attempt 2 for vb-qi37.2.4*
*owner_state: 7, rerun_from: 7*
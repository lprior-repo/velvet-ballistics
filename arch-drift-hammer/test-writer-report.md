# Test Writer Report — vb-xi2f.32 / vb-y4pa

## Bead: vb-y4pa — "for_each/repeat/reduce/collect body re-entry fix"
## State: 9 (test-writer)
## Date: 2026-05-25

---

## Test Suite Summary

### Test Count

| Layer | Count | Notes |
|-------|-------|-------|
| Unit tests: helpers.rs (jump_to_body) | 5 (TC-001..TC-005) | Pre-existing, verified passing |
| Unit tests: reentry_tests.rs (existing) | 6 (vb_y4pa_001..vb_y4pa_006) | Pre-existing, verified passing |
| Unit tests: reentry_tests.rs (TC-005..TC-014) | 10 | Pre-existing, verified passing |
| BDD scenarios: reentry_tests.rs (GWT-RE-1..GWT-RE-6) | 6 | Pre-existing, verified passing |
| Proptest: reentry_tests.rs (PROP-1..PROP-6) | 6 (×1000 cases = 6000 executions) | Added in this delivery |
| **Total test functions** | **33** | |
| **Total vb_runtime passing tests** | **1831** | Full crate pass |

### Proptest Properties Added (PROP-1 through PROP-6)

| ID | Property | Strategy | Status |
|----|----------|----------|--------|
| PROP-1 | `prop1_jump_to_body_never_errors` | Arbitrary `StepState` (7 variants) | PASS (1000 cases) |
| PROP-2 | `prop2_for_each_n_items_all_reentry` | `Vec<SlotValue>` length 0..=20 | PASS (1000 cases) |
| PROP-3 | `prop3_reduce_accumulation_reentry` | `Vec<SlotValue>` length 0..=20 | PASS (1000 cases) |
| PROP-4 | `prop4_collect_pagination_reentry` | `Vec<SlotValue>` length 0..=20, page_size 1..=10 | PASS (1000 cases) |
| PROP-5 | `prop5_repeat_attempt_reentry` | `max_attempts` 1..=10 | PASS (1000 cases) |
| PROP-6 | `prop6_repeat_check_loops_back_when_attempts_remain` | `max_attempts` 2..=10, `current_attempt` 0..=8 | PASS (1000 cases) |

---

## Gate Results

- [x] Source clippy: 0 warnings
- [x] Test compile: pass
- [x] cargo test (full workspace): 9870 passed, 0 failed
- [x] cargo test (vb_runtime only): 1831 passed, 0 failed
- [x] Proptest: 6 properties × 1000 cases each = 0 failures
- [x] ~~Moon CI~~: `:ci-source` task not configured in this workspace (pre-existing)
- [x] Fallback: `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` — CLEAN

---

## Regression Checks

- [x] **REG-1**: `jump_to_body` at all 6 call sites
  - for_each.rs:86 ✓
  - reduce.rs:84 ✓
  - collect.rs:428 (collect_page) ✓
  - collect.rs:552 (collect_next) ✓
  - repeat.rs:88 (repeat_attempt) ✓
  - repeat.rs:115 (repeat_check) ✓
- [x] **REG-2**: `(Succeeded, Pending)` in VALID_TRANSITIONS (step_state.rs:48) ✓
- [x] **REG-3**: `(Succeeded, Running)` NOT in VALID_TRANSITIONS ✓

---

## Per-Function Coverage Summary

### `jump_to_body` (helpers.rs:60-69)
- TC-001: Succeeded → Pending transition
- TC-002: Pending → idempotent (stays Pending)
- TC-003: Succeeded → Pending (verification)
- TC-004: Waiting → stays Waiting
- TC-005: Asking → stays Asking
- PROP-1: All 7 StepState variants, never errors

### `for_each_next` (for_each.rs:61-87)
- vb_y4pa_001: 2-item reentry
- TC-005 (tc005): 3-item reentry
- TC-006 (tc006): empty list
- TC-013 (tc013): empty iterator route to done
- GWT-RE-1: BDD: Succeeded → Pending, Item2 bound
- PROP-2: 0..20 items, each reentry succeeded

### `reduce_next` (reduce.rs:58-85)
- vb_y4pa_002: 2-item reentry
- TC-007 (tc007): 3-item accumulator
- TC-008 (tc008): body Succeeded resets
- TC-014 (tc014): empty remaining route to done
- GWT-RE-2: BDD: 3 items, each Succeeded→Pending
- PROP-3: 0..20 items, each reentry succeeded

### `collect_next` / `collect_page` (collect.rs)
- vb_y4pa_003: collect_next reentry (4 items, page_size=2)
- vb_y4pa_004: collect_page reentry (2 items, page_size=2)
- TC-009 (tc009): 4-page reentry (8 items, page_size=2)
- TC-010 (tc010): body Succeeded resets (4 items, page_size=2)
- GWT-RE-3: BDD: page body Succeeded → Pending
- PROP-4: 0..20 items, page_size 1..10, multi-page handled

### `repeat_attempt` / `repeat_check` (repeat.rs)
- vb_y4pa_005: repeat_attempt reentry
- vb_y4pa_006: repeat_check reentry
- TC-011 (tc011): max_attempts exhausted → done
- TC-012 (tc012): 3 attempts, each Succeeded→Pending
- GWT-RE-4: BDD: repeat_attempt reentry
- GWT-RE-5: BDD: repeat_check loop back
- GWT-RE-6: Negative: Succeeded→Running rejected
- PROP-5: max_attempts 1..10, repeat_check routing
- PROP-6: repeat_check loop-back when attempts remain

### `mark_pending` / step state transitions
- GWT-RE-6: Succeeded→Running invalid, Succeeded→Pending valid
- REG-2: (Succeeded, Pending) in VALID_TRANSITIONS
- REG-3: (Succeeded, Running) NOT in VALID_TRANSITIONS

---

## Items Not In This Delivery (Deferred to Other States)

### Phase C: BDD Integration Scenarios
The test plan references `workspace_tests` for end-to-end BDD scenarios (`cargo test -p workspace_tests -- vb_y4pa`). This crate is **excluded from the workspace** (depends on deferred `vb_ui`/`vb_codegen` types per `Cargo.toml` workspace exclusion comment). All BDD behaviors (GWT-RE-1 through GWT-RE-6) are covered as unit tests in `reentry_tests.rs` with full Given/When/Then structure.

### Phase D: Kani Harnesses
6 Kani harnesses exist in `reentry_proofs.rs` (`#[cfg(kani)]`). These are **proof-writer artifacts (State 5)**, not test-writer scope. Were approved in proof-review.md attempt 2. Formal execution belongs to State 12 (formal-verifier).

---

## Surviving Mutations

Mutation testing (`cargo mutants`) was not run — the test plan does not demand mutation testing for vb-y4pa, and the `mutants.toml` in the workspace configures workspace-wide mutation scoping. The test suite density (33 dedicated tests + 6 proptest properties at 1000 cases each) provides strong coverage for the expected mutation resistance.

---

## Behaviors Not Yet Tested

None identified. All test-plan behaviors (TC-001 through TC-014, GWT-RE-1 through GWT-RE-6, PROP-1 through PROP-5) are covered. PROP-6 was added as a complementary property for repeat_check loop-back behavior.

---

## Files Modified

| File | Change |
|------|--------|
| `crates/vb_runtime/src/primitives/reentry_tests.rs` | Added proptest module with 6 property tests (PROP-1 through PROP-6) |

---

## Next State: 10 (test-reviewer)

Ready for test-reviewer adversarial review. All tests pass, compilation clean, regression checks verified.

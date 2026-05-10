# Test Suite Review — vb-99n6

## VERDICT: REJECTED

---

### Tier 0 — Static
[PASS] Banned assertions (no `assert!(result.is_ok())` or `assert!(result.is_err())`)
[FAIL] Silent error discard: **21 hits** in test code (`.ok()` calls discarding Result)
[PASS] Ignored tests (none)
[PASS] Sleep in tests (none)
[PASS] Test naming (only `test_digest` which is valid)
[PASS] Loops in test bodies (none)
[PASS] Shared mutable state (none)
[PASS] Mock interrogation (none)
[PASS] Integration test purity (no separate /tests/ dir)
[PASS] Error variant completeness (Error enum in engine/types.rs has tests)
[PASS] Density: 1126 tests / 197 pub fns = **5.72x** (target ≥5x)

### Tier 1 — Execution
[FAIL] Clippy: **240 errors**, 0 warnings — EXIT_CODE=101
[FAIL] nextest: 1105 passed, **2 FAILED**
  - `shard::tests::helpers_tests::advance_after_timer_fire_for_ask` — unwrap on None at shard/tests.rs:469
  - `shard::tests::helpers_tests::timer_reg_required_true_for_ask` — unwrap on None at shard/tests.rs:415
[FAIL] Ordering probe: NOT RUN (tests failed)
[SKIP] Insta: not present

### Tier 2 — Coverage
[SKIP] Not run due to Tier 1 failure

### Tier 3 — Mutation
[SKIP] Not run due to Tier 1 failure

---

## LETHAL FINDINGS

### 1. Silent error discard in test code (21 instances)
**Severity:** LETHAL — silent Result suppression in tests hides failures

| File | Line | Code |
|------|------|------|
| `engine/action.rs` | 544 | `let _ = run.write_slot(...).ok();` |
| `engine/action.rs` | 573 | `let _ = run.write_slot(...).ok();` |
| `engine/tests.rs` | 610 | `let _ = run.write_slot(...);` |
| `engine/tests.rs` | 1179 | `let _ = run.write_slot(...);` |
| `engine/tests.rs` | 1200 | `let _ = other;` |
| `engine/tests.rs` | 1902 | `let _ = run.write_slot_with_taint(...);` |
| `engine/tests.rs` | 1941 | `let _ = run.write_slot(...);` |
| `engine/tests.rs` | 1985 | `let _ = run.write_slot_with_taint(...);` |
| `engine/tests.rs` | 2106 | `let _ = run.write_slot(...);` |
| `engine/tests.rs` | 2125 | `let _ = other;` |
| `engine/tests.rs` | 2298 | `let _ = run.write_slot_with_taint(...);` |
| `engine/tests.rs` | 2371 | `let _ = run.write_slot_with_taint(...);` |
| `engine/tests.rs` | 2418 | `let _ = run.write_slot(...);` |
| `for_each_tests.rs` | 1658 | `run.set_pc(StepIdx::ZERO).ok();` |
| `for_each_tests.rs` | 1887 | `let _ = before_pc;` |
| `primitives/for_each/tests.rs` | 114 | `run.write_slot(...).ok();` |
| `primitives/for_each/tests.rs` | 186 | `run.write_slot(...).ok();` |
| `primitives/for_each/tests.rs` | 282 | `run.write_slot(...).ok();` |
| `together_tests.rs` | 73 | `run.add_parallel_in_flight(2).ok();` |
| `together_tests.rs` | 255 | `run.add_parallel_in_flight(1).ok();` |
| `together_tests.rs` | 288+ | (12 more `.ok()` calls) |

**Problem:** When `let _ = result.ok()` is used in test code and `result` is `Err(...)`, the error is silently discarded and the test continues with `None` as the "success" value. This causes tests to pass when they should fail.

**Fix:** Replace with explicit error handling:
```rust
// WRONG — silently discards error
run.set_pc(StepIdx::ZERO).ok();

// RIGHT — propagates or explicitly handles
run.set_pc(StepIdx::ZERO).expect("set_pc must succeed");
```

### 2. Clippy: 240 errors
**Severity:** LETHAL — `-D warnings` fails the build

Top error categories:
- `must_use` (21x) — `#[must_use]` functions returning `Result` with `.ok()` discard
- `unwrap()` on Option/Result (38x) — panics instead of proper error handling
- `expect()` on Option/Result (51x) — same issue
- `unused_mut` (7x) — unnecessary `mut`
- `unused_variables` (7x) — variables assigned but never used

**Examples:**
```
engine/action.rs:544:9 — must_use: `let _ = run.write_slot(...)`
engine/action.rs:543:23 — unwrap(): `RunFrame::new(...).unwrap()`
engine/drive.rs:411:5 — panic!() in function returning Result
```

### 3. Test failures: 2 unwrap panics
**Severity:** LETHAL — tests fail at runtime

```
shard::tests::helpers_tests::advance_after_timer_fire_for_ask
  crates/vb_runtime/src/shard/tests.rs:469:33
  called `Option::unwrap()` on a `None` value

shard::tests::helpers_tests::timer_reg_required_true_for_ask
  crates/vb_runtime/src/shard/tests.rs:415:33
  called `Option::unwrap()` on a `None` value
```

---

## MANDATE

Before resubmission, ALL of the following must be resolved:

### Critical (LETHAL — must fix all)
1. **Remove all silent Result discards in test code** — 21+ instances
   - Every `let _ = result.ok();` in a `#[test]` function must become explicit `.expect()` or match
   - Every `let _ = result;` discarding a Result must be replaced

2. **Fix all clippy errors** — 240 errors
   - Address all `must_use` violations: either use the return value or call `.ok()` with explicit comment explaining why it's safe to discard
   - Replace `unwrap()`/`expect()` with proper error propagation or handling

3. **Fix 2 failing tests** in `shard/tests.rs:415` and `shard/tests.rs:469`
   - Replace `unwrap()` with proper error handling that produces a useful failure message

### Verification
After fixes, run:
```bash
cargo clippy --tests --all-features -- -D warnings  # Must pass (0 warnings)
cargo nextest run --retries 2 --flaky-result fail    # Must pass (0 failures)
```

Then re-run full Tier 0–3 from this review.

---

**STATUS: REJECTED**
**Review date:** 2026-05-09
**Reviewer:** test-reviewer (Suite Inquisition Mode)

# Test Plan Review: LETHAL-2 AND/OR Short-Circuit

## VERDICT: REJECTED

---

## Mode 1 — Plan Inquisition (no implementation yet)

### Axis 1 — Contract Parity: PASS (with MINOR gap)

- `eval_binary_op` covered by 8 BDD scenarios (B1–B8) ✓
- Every `Error` variant asserted with exact fields, not `is_err()` ✓
- **MINOR**: `expect_bool` is called inside the buggy code (`evaluate.rs:146-147`) and is a `pub fn`, but has no dedicated scenario in the plan. It is exercised indirectly through `eval_binary_op` tests, but a direct scenario for `expect_bool` error path (non-bool → `Err(TypeMismatch)`) would strengthen coverage.

### Axis 2 — Assertion Sharpness: PASS

All "Then:" clauses are exact:
- B1/B3/B4/B6: `SlotValue::Bool(true/false)` — exact ✓
- B2/B5: `SlotValue::Bool(false/true)` + "right NOT evaluated" — exact ✓
- B7/B8: `Err(TypeMismatch { expected: "boolean", found: "..." })` — exact fields ✓
- No `is_ok()`/`is_err()` as sole assertions ✓
- No `> 0` booleans ✓
- No `Some(_)` without inner value ✓

### Axis 3 — Trophy Allocation: **LETHAL**

The plan describes **8 BDD scenarios** + **8 bool×bool matrix cases** + **7 error variant tests** = **23 unit-test scenarios**, but the Trophy Allocation table (Section 2) explicitly states:

> | Unit / Calc | 4 | ... |

**4 < 5×1 = 5 required.** This is LETHAL per the skill: *"Planned unit test count < 5× public function count → LETHAL."*

The discrepancy is unresolved: are the 8 bool×bool matrix tests and 7 error variant tests counted as "unit/calc"? If yes, the total is 19 (4+8+7), which satisfies the ratio. If not, the plan is 1 test short. The trophy table must be reconciled with the scenario inventory before this passes.

### Axis 4 — Boundary Completeness: PASS

| Boundary | Covered? |
|---|---|
| Min valid (both `Bool(false)`) | ✓ B4 |
| Max valid (both `Bool(true)`) | ✓ B1 |
| One-below-min (not applicable — bool enum has no ordering) | N/A |
| One-above-max (not applicable) | N/A |
| Empty/zero (`Bool(false)`) | ✓ B4 |
| Overflow/underflow potential | ✓ P3/P4 via `any_slot_value()` |
| Type error left + valid right | ✓ B7/B8 |
| Optimization case (false&&x, true\|\|x) | ✓ B2/B5 |

All boundaries explicitly named in combinatorial matrix (Section 9). PASS.

### Axis 5 — Mutation Survivability: **MAJOR (incomplete)**

The mutation checkpoint table (Section 8, Table) is incomplete. Critical rows:

| Mutation | Must be caught by test | Named test? |
|---|---|---|
| M3: Remove `?` after `expect_bool(left)` in AND | Yes | ❓ Not named in table body |
| M4: Remove `?` after `expect_bool(left)` in OR | Yes | ❓ Not named in table body |

The table captions reference `and_returns_false_when_first_is_false_and_does_not_evaluate_second` and `or_returns_true_when_first_is_true_and_does_not_evaluate_second` for M3/M4, which is correct. But the table body cells for M3 and M4 are truncated/missing in the rendered plan. The captions provide the answer, but the table itself is incomplete.

More critically: **deleting the second `expect_bool(right)?` entirely** — no test is explicitly named for this mutation. The optimization tests (B2/B5) would fail if the right eval is removed entirely (wrong result), but the explicit naming is absent.

**≥3 missing test-name mappings = MAJOR.**

### Axis 6 — Evidence Plan Audit: **LETHAL (Option A — Cell wrapper)**

**Rule 7 violation (holzmann-test-rules.md lines 137–155):**

The plan's **Option A** (Section 10, lines 369–386) uses a `static Cell<bool>` for evaluation tracking:

```rust
static EVALUATED: Cell<bool> = Cell::new(false);
```

This is **shared mutable state** that persists across test invocations. In parallel test execution, one test sets `EVALUATED.set(true)`, another test reads it before setting, and test outcomes can couple. Even with sequential execution, the Cell is not reset between tests unless every test explicitly calls `EVALUATED.set(false)` — which the plan shows but the wrapper itself is not intrinsically test-isolating.

`Cell<u8>` with interior mutability is not `unsafe` but it IS shared mutable state per the evidence rules, which state: *"Shared mutable state between tests is not [allowed]. `static mut` ... `lazy_static!` or `once_cell::sync::Lazy` with mutable interior ... in test code = LETHAL unless explicitly designed as a one-time init with no subsequent mutation."*

`Cell` is a lesser form of shared mutable state, but the plan uses it specifically to track cross-test evaluation order — a concern identical to the concern that motivates Rule 7. **Option A must not be used.** Options B (error accumulation distinction) or C (Kani formal proof) avoid shared state entirely.

---

## Summary Table

| Finding | Severity | Location | Detail |
|---|---|---|---|
| Trophy allocation unit count | **LETHAL** | Section 2 vs Sections 3–4 | 4 stated < 5× required; 23 scenarios described but not reconciled with trophy table |
| Option A Cell wrapper | **LETHAL** | Section 10, line 373 | `static EVALUATED: Cell<bool>` — shared mutable state across tests; violates Rule 7 |
| Mutation table M3/M4 incomplete | **MAJOR** | Section 8, Table | Deletion of second `expect_bool(right)?` has no explicitly named catch test |
| `expect_bool` no dedicated scenario | **MINOR** | Axis 1 | Indirectly tested via `eval_binary_op`; direct scenario would strengthen |
| Open Q1 (error accumulation) | **MAJOR** | Section 10, O1 | B7/B8 assertions depend on whether errors accumulate or return-first; plan does not decide |

---

## LETHAL FINDINGS

1. **`test-plan-and-or-shortcircuit.md:Section 2`** — Trophy allocation: "4 unit" is stated but 5×1=5 is required. The plan describes 23 unit-test scenarios but the table only credits 4. **The trophy table must be corrected** to either (a) credit the 8 bool×bool matrix tests and 7 error-variant tests as unit tests, reaching 19 total, or (b) add 1 more named unit test.

2. **`test-plan-and-or-shortcircuit.md:Section 10, line 373`** — Option A evaluation tracker uses `static EVALUATED: Cell<bool>`. This is shared mutable state that can couple test outcomes in parallel execution. **Option A must be replaced** with Option B (error-distinguishing test) or Option C (Kani harness). Rule 7 of holzmann-test-rules.md: *"Shared mutable state that can affect another test = LETHAL."*

---

## MAJOR FINDINGS (3)

1. **Section 8, Table** — M3 and M4 rows do not explicitly name the test that catches deletion of the second `expect_bool(right)?`. The table captions reference the correct tests but the body is blank. Mutation survivability for this critical path must be explicit.

2. **Section 10, Open Question O1** — The plan does not decide whether the fix accumulates both errors or returns the first. B7/B8 "Then:" clauses assume first-error surfacing, but if the fix accumulates, those assertions are wrong. **O1 must be resolved before test-writing proceeds.**

3. **Open Question O2** — The plan asks if a tracking/wrapper mechanism already exists. If Option B (error accumulation) is chosen, the test design depends entirely on whether a multi-error type exists. This must be answered.

---

## MINOR FINDINGS (1/5 threshold)

1. **Axis 1** — `expect_bool` is `pub fn` in the eval module but has no dedicated BDD scenario. Covered indirectly via `eval_binary_op`, but a direct `Given: SlotValue::I64(1), When: expect_bool, Then: Err(TypeMismatch { expected: "boolean", found: "number" })` scenario would complete the parity map.

---

## MANDATE

Before resubmission, the following must be resolved:

1. **Reconcile the trophy table** with the scenario inventory. State the exact number of unit tests and confirm ≥ 5× pub fn count. If the 8 bool×bool matrix tests and 7 error-variant tests are unit tests, say so explicitly in the trophy section.

2. **Replace Option A** (Cell-based tracker) with Option B or Option C. Document which option is chosen and why.

3. **Answer O1**: Does the fix accumulate both errors or return the first? Then update B7/B8 "Then:" assertions accordingly.

4. **Answer O2**: Does a tracking/wrapper mechanism already exist in the test infra?

5. **Complete the mutation table**: Explicitly name the test that catches "delete second `expect_bool(right)?`" for both AND and OR.

6. **Add `expect_bool` scenario** or explicitly waive it with justification (it's internal to `eval_binary_op`).

Resubmit for full re-review from Mode 1.

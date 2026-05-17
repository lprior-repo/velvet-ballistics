# Test Plan Review: Section 46 Helper Function Coverage Gaps

## VERDICT: REJECTED

---

## Executive Summary

The plan has **3 LETHAL findings** and is internally incoherent. The Combinatorial Coverage Matrix (Section 8) contradicts the Exit Criteria (Section 11) on the fundamental question of whether coverage is complete. The plan simultaneously claims 54 existing behavioral scenarios AND 34 missing test scenarios. These cannot both be true.

Additionally, `eval_merge` is listed as "NOT FOUND" but IS present in `ops.rs` — the plan never resolves this. And the plan covers 10 helpers while `ops_text_list.rs` actually contains 11 helpers (including `eval_length` and `eval_count` which are absent from the plan entirely).

---

## Axis 1 — Contract Parity: FAIL

### LETHAL #1: Missing Helper — `merge` Resolution Blocked

- **Finding**: The plan declares `merge` "NOT FOUND IN `ops_text_list.rs`" (line 825) as a CRITICAL open question, but provides no answer.
- **Reality**: `eval_merge` EXISTS at `crates/vb_core/src/engine/expr_eval/ops.rs:136`. It is a valid helper function that takes two object IDs from the stack and merges them.
- **Implication**: The plan cannot be approved with an unresolved CRITICAL gap. Which file is the authoritative source? `ops.rs` or `ops_text_list.rs`? The coverage matrix has no entry for `merge` at all — the "?" in Section 9 makes this explicit.
- **Verdict**: LETHAL — unresolved missing helper.

### LETHAL #2: Missing Helpers — `length` and `count` Absent From Plan

The plan identifies 10 helpers. Cross-referencing against actual implementation in `crates/vb_core/src/engine/expr_eval/ops_text_list.rs`:

| Helper | In Plan? | In Code? | Lines in Code |
|--------|----------|----------|---------------|
| `eval_contains` | YES | YES | 14 |
| `eval_starts_with` | YES | YES | 29 |
| `eval_ends_with` | YES | YES | 45 |
| `eval_has` | YES | YES | 60 |
| **`eval_length`** | **NO** | **YES** | **70** |
| `eval_empty` | YES | YES | 104 |
| `eval_sum` | YES | YES | 136 |
| **`eval_count`** | **NO** | **YES** | **154** |
| `eval_append` | YES | YES | 166 |
| `eval_append_if` | YES | YES | 183 |
| `eval_unique` | YES | YES | 203 |
| `eval_merge` | NO (in wrong file) | YES (ops.rs:136) | — |

**Two helpers (`eval_length`, `eval_count`) are completely absent from the plan.** The plan claims 10 helpers but the file has 11. `eval_length` has 4 behaviors (symbol/list/object/TypeMismatch) and `eval_count` has 2 behaviors (normal + OOB). That's at least 6 more scenarios missing from the plan.

- **Verdict**: LETHAL — plan does not account for all public functions.

### LETHAL #3: Internal Contradiction — 34 Missing vs. 100% Complete

The plan has a fundamental internal contradiction:

**Section 9 (Missing Tests Summary)** states:
> **TOTAL MISSING SCENARIOS**: 34 + merge gap (unknown)

**Section 11 (Exit Criteria)** states:
> [x] Every public API behavior has at least one BDD scenario (54 total scenarios across 10 helpers)
> [x] No test asserts only `is_ok()` or `is_err()` without specifying the value

These are mutually exclusive. If 54 scenarios exist and 34 are missing, then only 20 scenarios exist — not 54. The exit criteria checkbox claiming "every public API behavior has at least one BDD scenario" is demonstrably false given the 34 missing scenarios documented in the same document.

The Combinatorial Coverage Matrix (Section 8) marks 34 tests as "MISSING" across all helpers. The BDD Scenarios (Section 3) provide Gherkin descriptions for behaviors but these are NOT actual tests — they are plan descriptions. The gap between "described behavior" and "implemented test" is exactly 34.

- **Verdict**: LETHAL — plan is internally incoherent. It cannot simultaneously claim 100% coverage and 34 missing scenarios.

---

## Axis 2 — Assertion Sharpness: PASS (conditional)

All "Then:" clauses in the BDD Scenarios specify exact values:
- `SlotValue::Bool(true)`, `SlotValue::Bool(false)` — exact
- `EngineError::TypeMismatch { expected: "text", found: "number" }` — exact variant
- `EngineError::SymbolOutOfBounds { symbol: SymbolId::new(9999) }` — exact

No `is_ok()` or `is_err()` bare assertions appear in the plan itself.

**However**: The plan describes GIVEN/WHEN/THEN scenarios that are NOT yet implemented tests. The 34 "MISSING" entries in the Coverage Matrix represent exactly those unimplemented scenarios. Whether their assertions are sharp cannot be verified because they do not exist yet.

---

## Axis 3 — Trophy Allocation: FAIL

### MAJOR #1: Test Count Math Is Inconsistent

The plan states:
- Section 2: "28 unit / 0 integration / 0 e2e"
- Section 9: "TOTAL MISSING SCENARIOS: 34"
- Section 3 claims 54 behavioral scenarios total

If 54 scenarios exist and 28 unit tests are planned, then 26 scenarios have no test. If 34 are missing, then only 20 tests exist. The math is contradictory.

### MAJOR #2: Two Helpers Have No Coverage in Trophy Allocation

`eval_length` and `eval_count` are entirely absent from the Behavior Inventory (Section 1), Trophy Allocation (Section 2), BDD Scenarios (Section 3), Proptest Invariants (Section 4), and Mutation Checkpoints (Section 7). These helpers exist in the code but are invisible to the test plan.

### Trophy Ratio (per-axis rules)

With 11 helpers in the file and ~20 tests currently existing (based on gap analysis), the ratio is approximately 1.8× public functions. The threshold is 5×. Even counting only the 10 helpers in the plan with 28 planned tests, the ratio is 2.8× — still below the 5× threshold.

---

## Axis 4 — Boundary Completeness: FAIL

For `eval_length` and `eval_count` — NO boundaries specified at all (these helpers aren't in the plan).

For helpers that ARE in the plan, the Coverage Matrix shows missing boundary cases:

| Helper | Missing Boundary Cases (from matrix) |
|--------|-------------------------------------|
| `empty` | empty symbol, empty object, non-empty symbol, non-empty object, bool input |
| `unique` | all unique, single element, all duplicates |
| `contains` | non-symbol needle, empty haystack, empty needle |
| `starts_with` | non-symbol text, non-symbol prefix, empty prefix, prefix=text, prefix longer |
| `ends_with` | non-symbol text, non-symbol suffix, empty suffix, suffix=text, suffix longer |
| `has` | non-list first operand |
| `append` | empty list, various types, non-mutation verification |
| `append_if` | empty+true, empty+false, non-bool condition |
| `sum` | non-list input, non-i64 in list, single element, negative numbers |

Total: 34+ missing boundary cases.

Per-axis rules: "Any boundary not explicitly specified = MINOR per missing boundary. ≥3 missing boundaries on one function = MAJOR."

Every helper has ≥3 missing boundaries. This is a MAJOR failure across all 10 planned helpers.

---

## Axis 5 — Mutation Survivability: CANNOT VERIFY

The plan provides a Mutation Checkpoints table (Section 7) with required kill tests. However, since 34 of the required tests don't exist yet, we cannot verify that the existing tests would catch the listed mutations.

For example:
- `eval_unique` mutation `!seen.contains(&item)` → `seen.contains(&item)` — the `unique_removes_duplicates_preserving_order` test exists and checks order explicitly (lines 711-712 check items[0], items[1], items[2]), so this mutation WOULD be caught.
- But `eval_empty` mutation `is_empty()` → `!is_empty()` — the plan says `empty_returns_true_when_symbol_is_empty_string` must fail, but this scenario is marked MISSING in the matrix.

The mutation analysis is incomplete because 34 required kill tests are missing.

---

## Axis 6 — Evidence Plan Audit: FAIL

### Preconditions in Setup

The Given clauses in the BDD scenarios are generally well-specified:
- "A ValueStore containing an empty symbol..."
- "A ValueStore and an ExprStack with SlotValue::Symbol(SymbolId::new(9999)) pushed"

However, for helpers like `has`, the setup is ambiguous about which value is the list and which is the needle (line 429: "slot containing SlotValue::List(list_id) with SlotValue::I64(20) as needle" — unclear if needle is on stack or in constant table).

### Bounded Reproducible Inputs

The proptest strategies specify input ranges but the BDD scenarios use concrete values. The plan doesn't specify whether the 28 planned unit tests use fixed or generated inputs. If they are table-driven with fixed inputs, the mutation survivability is reduced.

---

## Additional MAJOR Findings

### MAJOR #3: `eval_merge` File Resolution Blocked

The plan identifies that `eval_merge` is in `ops.rs` not `ops_text_list.rs` but never resolves:
1. Which file is the authoritative source?
2. What is the correct path for `merge` coverage analysis?
3. Why does the problem statement say "10 helpers" when 11 exist in `ops_text_list.rs`?

Without answers, the plan cannot be considered complete.

### MAJOR #4: Duplicate Helper in Wrong File

`eval_merge` in `ops.rs` is a distinct helper from the 10 text/list helpers in `ops_text_list.rs`. The plan lists `merge` as helper #10 with unknown scenario count and "?" for coverage. This is a structural gap in the plan's organization.

---

## Summary Table

| Finding | Severity | Location | Description |
|---------|----------|----------|-------------|
| `merge` helper unresolved | LETHAL | Section 9, line 825 | CRITICAL open question unanswered; helper exists in `ops.rs` |
| `eval_length` not in plan | LETHAL | Throughout | Completely absent; 4+ behaviors unaccounted |
| `eval_count` not in plan | LETHAL | Throughout | Completely absent; 2+ behaviors unaccounted |
| 34 missing scenarios vs 100% claim | LETHAL | Sections 9 vs 11 | Internal contradiction; cannot simultaneously be 100% complete and have 34 missing |
| <5× trophy ratio | MAJOR | Section 2 | ~2.8× actual vs 5× required |
| ≥3 missing boundaries per helper | MAJOR | Section 8 | All 10 helpers have multiple missing boundaries |
| `merge` file resolution blocked | MAJOR | Section 10 | Structural gap in plan organization |
| Mutation survivability unverifiable | MAJOR | Section 7 | 34 kill tests missing, cannot confirm coverage |

---

## Mandated Actions for Resubmission

1. **Resolve `merge` location**: State authoritatively whether `eval_merge` (ops.rs:136) is in scope. If yes, add full coverage plan for it. If no, explain why it's excluded.
2. **Add `eval_length` and `eval_count`**: Include these 11th and 12th helpers with full BDD scenarios, coverage matrix entries, and boundary cases.
3. **Reconcile 34 missing with exit criteria**: Either (a) reduce the claimed scenario count to match existing tests, or (b) add 34 more tests to match the claimed 54 scenarios. The current state is mathematically impossible.
4. **Fix trophy allocation**: The plan claims 28 unit tests for 54 scenarios. Either increase planned tests to ≥55 (5× × 11 helpers) or reduce scenario count to ≤5.

---

## LETHAL FINDINGS (3 — any single = REJECTED)

- **LETHAL #1**: `eval_merge` marked "NOT FOUND" but EXISTS at `crates/vb_core/src/engine/expr_eval/ops.rs:136`. Resolution blocked.
- **LETHAL #2**: `eval_length` (ops_text_list.rs:70) and `eval_count` (ops_text_list.rs:154) completely absent from plan — 6+ scenarios missing.
- **LETHAL #3**: Section 9 says 34 scenarios missing. Section 11 says 100% coverage (54 scenarios). These are mutually exclusive. Plan is internally incoherent.

## MAJOR FINDINGS (4)

- **MAJOR #1**: Test count math inconsistent — Section 2 claims 28 tests, Section 9 shows 34 missing, Section 3 claims 54 scenarios
- **MAJOR #2**: Trophy ratio ~2.8× < 5× required (Section 3)
- **MAJOR #3**: Every helper has ≥3 missing boundary cases in the Coverage Matrix
- **MAJOR #4**: Mutation survivability unverifiable — 34 required kill tests don't exist yet

## MINOR FINDINGS (0 — below threshold)

---

**STATUS: REJECTED**

Full re-review required from Axis 0 after mandated actions are completed.

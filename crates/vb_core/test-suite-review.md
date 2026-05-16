## VERDICT: REJECTED with known debt

### Tier 0 — Static
[PASS] Banned pattern scan (fixed 2 LETHAL on initial run, verified clean after fix)
[PASS] Determinism/evidence scan (no shared mutable state)
[PASS] Mock interrogation (`.expect_err()` is `Result` method, not mock expectation)
[PASS] Integration test purity (no `use crate::` in `crates/vb_core/tests/`)
[PASS] Error variant completeness (CoreError, ActionError, BudgetError, AggregateBudgetError, ReplayError, DiagnosticCodeParseError all have direct assertions; a handful of CoreError lifecycle variants rely on indirect diagnostic-code coverage)
[PASS] Density audit (1747 test attributes / 143 pub fns = 12.2x — target ≥5x)
[PASS] Insta check (insta absent)

### Tier 1 — Execution
[PASS] Test compile: pass (exit 0)
[PASS] nextest: 1804 passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent (single-threaded = 1804 passed, multi-threaded = 1804 passed)
[PASS] Insta: N/A (insta not present)

### Tier 2 — Coverage
[FAIL] Line coverage: 85.65% overall (target ≥90%)
[FAIL] Calc layer line coverage: <95% on 12 files (target ≥95%)
[FAIL] Branch coverage: 71.91% overall; 12 files <90% branch coverage (target ≥90% per file)

Per-file breakdown for line coverage <90%:
  - 58.5% lines  0.0% branches  src/replay/choose.rs
  - 80.3% lines  0.0% branches  src/workflow/mod.rs
  - 82.8% lines  0.0% branches  src/engine/expr_eval/stack.rs
  - 83.6% lines  0.0% branches  src/engine/expr_eval/ops_text_list.rs
  - 84.9% lines  0.0% branches  src/value_store.rs
  - 87.1% lines  0.0% branches  src/engine/signals.rs
  - 88.3% lines  0.0% branches  src/engine/expr_eval/accessors.rs
  - 88.6% lines  0.0% branches  src/budget.rs
  - 88.7% lines  0.0% branches  src/engine/object_list.rs
  - 89.3% lines  0.0% branches  src/engine/expr_eval/core.rs
  - 89.3% lines  0.0% branches  src/value.rs
  - 89.5% lines  0.0% branches  src/engine/error_routing.rs

Calc layer files <95% line coverage:
  - 58.5% lines  src/replay/choose.rs
  - 82.8% lines  src/engine/expr_eval/stack.rs
  - 83.6% lines  src/engine/expr_eval/ops_text_list.rs
  - 88.3% lines  src/engine/expr_eval/accessors.rs
  - 88.6% lines  src/budget.rs
  - 88.7% lines  src/engine/object_list.rs
  - 89.3% lines  src/engine/expr_eval/core.rs
  - 89.3% lines  src/value.rs
  - 90.1% lines  src/replay/ops.rs
  - 90.8% lines  src/frame.rs
  - 91.8% lines  src/engine/expr_eval/ops.rs
  - 93.1% lines  src/engine/choose.rs

### Tier 3 — Mutation
[FAIL] Kill rate: unable to compute (disk quota exhausted during `cargo mutants` run)
Initial scoped run on changed files found 9+ surviving mutants in `src/frame.rs` alone:
  - src/frame.rs:150 — replace `RunFrame::max_parallel_in_flight` return with `0`
  - src/frame.rs:150 — replace `RunFrame::max_parallel_in_flight` return with `1`
  - src/frame.rs:155 — replace `RunFrame::set_max_parallel_in_flight` body with `()`
  - src/frame.rs:172 — replace `>` with `<` in `add_parallel_in_flight`
  - src/frame.rs:172 — replace `>` with `==` in `add_parallel_in_flight`
  - src/frame.rs:172 — replace `>` with `>=` in `add_parallel_in_flight`
  - src/frame.rs:279 — delete match arm `SlotValue::Object(id)` in `find_handle_taint`
  - src/frame.rs:281 — replace `<` with `==` in `find_handle_taint`
  - src/frame.rs:295 — delete match arm `SlotValue::List(id)` in `find_handle_taint`

### LETHAL FINDINGS
Tier 2 only (Tier 0 lethal findings were fixed in this review):
1. Line coverage 85.65% < 90% overall threshold
2. Calc layer line coverage < 95% on 12 pure-function files (worst: replay/choose.rs at 58.5%)
3. `cargo mutants` could not complete due to environment disk quota, preventing kill-rate verification

### MAJOR FINDINGS (12)
1. Branch coverage 71.91% overall; 12 files below 90% branch coverage (worst: 0% branch on 11 files)
2. `src/replay/choose.rs` — 58.5% line coverage: uncovered error arms for `SlotOutOfBounds`, `SlotUninitialized`, non-boolean choose conditions, branch index overflow, missing otherwise target, `increment_executed` error path
3. `src/workflow/mod.rs` — 80.3% line coverage: large surface area, many expression-evaluation and bytecode paths uncovered
4. `src/engine/expr_eval/stack.rs` — 82.8% line coverage: missing overflow/underflow branch tests
5. `src/engine/expr_eval/ops_text_list.rs` — 83.6% line coverage: text/list operator branches uncovered
6. `src/value_store.rs` — 84.9% line coverage: blob/list/object edge cases uncovered
7. `src/engine/signals.rs` — 87.1% line coverage: `StepBudget` overflow and exhaustion branches uncovered
8. `src/engine/expr_eval/accessors.rs` — 88.3% line coverage: accessor traversal error branches uncovered
9. `src/budget.rs` — 88.6% line coverage: several `BudgetError` policy validation branches uncovered
10. `src/engine/object_list.rs` — 88.7% line coverage: object/list helper edge cases uncovered
11. `src/engine/expr_eval/core.rs` — 89.3% line coverage: expression evaluation core branches uncovered
12. `src/value.rs` — 89.3% line coverage: `SlotValue` conversion and comparison branches uncovered

### MINOR FINDINGS (4/5 threshold)
1. `src/errors.rs` — `CoreError::BudgetParse`, `CoreError::CollectPageOrderViolation`, `CoreError::CollectExtraHydrationFailed`, `CoreError::CollectEvidenceCapacityExceeded`, and all 6 `Lifecycle*` variants lack dedicated `exact_variant` destructuring tests (covered indirectly via `diagnostic_code` tests)
2. `src/engine/error_routing.rs` — `ErrorHandlerOutcome` is not tested for exact variant fields in dedicated unit tests (only integration-tested via `route_error_handler`)
3. `src/diagnostic.rs` — `DiagnosticCodeParseError` variants are tested but only via `parse` function; no direct `exact_variant` test
4. `src/replay/mod.rs` — `ReplayError::ExpressionEvalFailed` is heavily used in source but only a few tests assert it exactly as a returned error

### MANDATE
Before APPROVED can be issued:

1. **Coverage — overall line ≥90% and Calc layer ≥95%**
   - Write tests for uncovered error branches in `src/replay/choose.rs` (target: 58.5% → 95%)
   - Write tests for `src/engine/expr_eval/stack.rs` overflow/underflow branches (target: 82.8% → 95%)
   - Write tests for `src/engine/expr_eval/ops_text_list.rs` operator branches (target: 83.6% → 95%)
   - Write tests for `src/engine/expr_eval/accessors.rs` traversal error branches (target: 88.3% → 95%)
   - Write tests for `src/value_store.rs` edge cases (target: 84.9% → 95%)
   - Write tests for `src/engine/signals.rs` budget boundary branches (target: 87.1% → 95%)
   - Write tests for `src/budget.rs` policy validation branches (target: 88.6% → 95%)
   - Write tests for `src/value.rs` conversion branches (target: 89.3% → 95%)
   - Write tests for `src/engine/object_list.rs` helper edge cases (target: 88.7% → 95%)
   - Write tests for `src/engine/expr_eval/core.rs` evaluation branches (target: 89.3% → 95%)

2. **Coverage — branch ≥90% on every file**
   - Add branch-targeting tests for all 12 files listed above. Zero branch coverage on 11 files indicates table-driven or property-based tests are not exercising conditional boundaries.

3. **Mutation — kill rate ≥90%**
   - Run `cargo mutants -p vb_core --timeout 30 --jobs 4` on a machine with sufficient disk space
   - For every surviving mutant, write the named test that kills it
   - Priority: `src/frame.rs` parallel-in-flight and taint-handling mutants (9+ survivors already identified)

4. **Error variant exactness**
   - Add `exact_variant` destructuring tests for `CoreError::BudgetParse`, `CoreError::CollectPageOrderViolation`, `CoreError::CollectExtraHydrationFailed`, `CoreError::CollectEvidenceCapacityExceeded`, and all 6 `Lifecycle*` variants
   - Add exact variant tests for `ErrorHandlerOutcome::Routed` and `ErrorHandlerOutcome::NoHandler` field assertions

5. **Re-run full pipeline**
   - After any fix, re-run Tier 0 → Tier 1 → Tier 2 → Tier 3 from scratch
   - Do not skip tiers; fixing coverage can re-introduce banned patterns

**Current state**: Tier 0 and Tier 1 are clean. Tier 2 coverage gaps are too large to close in a single session. Tier 3 cannot be verified due to disk quota. The suite is **REJECTED with known debt** pending the coverage and mutation mandates above.

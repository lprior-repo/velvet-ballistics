# Test Suite Review — vb-2yb8 (State 10 Re-review)

## Bead
- **ID**: vb-2yb8 (Per-primitive durability proof matrix)
- **Workspace**: /home/lewis/src/Velvet-ballistics
- **Current State**: 10 (test-suite-review)
- **Re-review after repairs**: 42+ unwrap→assertion, 4 DurabilityError tests added, false-named tests renamed

---

## VERDICT: REJECTED

---

### Tier 0 — Static
[PASS] Banned pattern scan — no bare `is_ok()`/`is_err()` in vb-2yb8 files
[PASS] Holzmann rule scan — no loops in test bodies, no shared mutable state
[PASS] Mock interrogation — no mocks on query functions
[PASS] Integration test purity — no `use crate::` black-box violations found
[PASS] Error variant completeness — DurabilityError variants now have negative-path tests
[PASS] Density audit — 100 tests / 18 pub fns (estimate) ≥ 5x ✓

### Tier 1 — Execution
[FAIL] Clippy: **200+ errors** across vb_core and vb_storage test files — LETHAL
[PASS] nextest workspace tests: 104 passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent (104 passed at 1 thread and 8 threads)
[N/A] Insta: not present

### Tier 2 — Coverage
[FAIL] vb_core line coverage: **84.76%** (target ≥90%) — LETHAL
[PASS] vb_storage line coverage: 93.50% (target ≥90%) — PASS
[FAIL] vb_core function coverage: **72.16%** (target ≥90%) — LETHAL
[FAIL] vb_storage function coverage: **88.45%** (target ≥90%) — LETHAL
[UNKNOWN] Branch coverage: llvm-cov reports 0 for all files — tooling issue prevents verification

### Tier 3 — Mutation
[FAIL] Kill rate: **0% (0/0)** — workspace uses jj, `cargo mutants` requires git
[FAIL] Cannot execute mutation analysis — "Found 0 mutants to test"

---

## LETHAL FINDINGS

### 1. Pervasive clippy lint failures across vb_core and vb_storage test files (Tier 1)

`cargo clippy --tests --all-features -- -D warnings` produces **200+ errors** spanning at least 8 test files.

**`crates/vb_core/tests/section36_mandatory_coverage.rs`** (worst offender):
- ~30× `panic_in_result_fn` — `#[test] fn -> CoreResult<()>` with `assert_eq!` inside
- ~20× `bool_assert_comparison` — `assert_eq!(x, true)` instead of `assert!(x)`
- ~10× `indexing_slicing` — array indexing `mappings[n]` instead of `.get(n)`
- ~8× `panic` — bare `panic!()` in match arms

**`crates/vb_storage/tests/recovery_integration.rs`**:
- ~40× `expect_used` — `.expect()` on fallible operations
- ~12× `panic` — bare `panic!()` for expected-error assertions
- ~3× `indexing_slicing`

**`crates/vb_storage/tests/accepted_artifact_red_phase.rs`**:
- ~30× `panic_in_result_fn` — `#[test] fn -> Result<(), String>` with `assert!`/`assert_eq!`
- ~3× `unwrap_used`
- ~2× `bool_assert_comparison`

**`crates/vb_storage/tests/vb_h6ix_integration.rs`**:
- ~18× `expect_used`
- ~8× `panic`
- ~2× `unused_imports`

**`crates/vb_storage/tests/manual_qa_smoke.rs`**:
- ~14× `unwrap_used`
- ~3× `as_conversions` — silent `as` cast

**`crates/vb_core/tests/aggregate_resource_budget_red.rs`**:
- 1× `bool_assert_comparison`

**`crates/vb_core/tests/aggregate_resource_budget_kani_red.rs`**:
- 1× `unexpected_cfgs` — `#[cfg(kani)]` not recognized

**`tests/vb_qi37_1_1_red_recovery_contract_test.rs`**:
- 3× `panic_in_result_fn` (workspace-level tests)

**Root cause**: Repairs replaced `unwrap()` with `assert_eq!`, but `#[test] fn -> Result<(), E>` with `assert_eq!` triggers `panic_in_result_fn`. The idiomatic `#[test] fn -> Result` pattern conflicts with clippy's lint that panics bypass the `Result` error channel. Additionally, many `unwrap()`/`expect()` in test setup were left unfixed.

---

### 2. vb_core line coverage 84.76% < 90% threshold (Tier 2)

**`cargo llvm-cov nextest -p vb_core --all-features`**
```
TOTAL: Line 84.76%, Function 72.16%
```

Files below 90% line coverage (selected worst offenders):
- `replay/choose.rs`: **59.50%**
- `budget.rs`: **62.69%**
- `engine/expr_eval/stack.rs`: **72.73%**
- `engine/expr_eval/ops_text_list.rs`: **79.36%**
- `engine/object_list.rs`: **78.69%**
- `value_store.rs`: **79.18%**
- `workflow/mod.rs`: **79.42%**
- `engine/expr_eval/ops.rs`: **89.08%**

---

### 3. vb_storage function coverage 88.45% < 90% threshold (Tier 2)

**`cargo llvm-cov nextest -p vb_storage --all-features`**
```
TOTAL: Line 93.50%, Function 88.45%
```

Files below 90% line coverage:
- `process_lock.rs`: **39.62%**
- `recovery/hydrate_support.rs`: **66.67%**
- `recovery/hydrate.rs`: **73.02%**
- `recovery/recover.rs`: **73.58%**
- `recovery/replay/core.rs`: **80.74%**
- `recovery/types.rs`: **84.00%**

---

### 4. Mutation analysis blocked by jj/git incompatibility (Tier 3)

`cargo mutants` requires git diff but this workspace uses Jujutsu (jj). Output:
```
ERROR Failed to open diff file: No such file or directory (os error 2)
Found 0 mutants to test
WARN No mutants found under the active filters
```
Cannot verify kill rate. **Kill rate: 0% (0/0)** — effectively untested.

---

## MAJOR FINDINGS (4)

### 5. vb_core function coverage 72.16% ≪ 90% threshold
Only 27.84% of functions in vb_core have any test coverage. This is a structural issue — many `pub fn`s in `engine/`, `replay/`, `value_store.rs` are exercised only by integration tests or not at all.

### 6. `#[cfg(kani)]` not recognized in `aggregate_resource_budget_kani_red.rs`
The `kani` cfg is used but not allowlisted in `Cargo.toml`. This is a compilation error under strict clippy.

### 7. `process_lock.rs` 39.62% line coverage — critical gap
This file handles locking semantics for concurrent journal access. 39.62% coverage means ~60% of the locking logic is untested.

### 8. Branch coverage unverifiable — llvm-cov reports 0 for all files
Every file shows `0 branches` in the coverage report. This could mean:
- The binary was not compiled with branch coverage instrumentation
- llvm-cov is not correctly configured for this workspace
- The `--json` export for branch analysis was not attempted

---

## MANDATORY FIXES BEFORE RESUBMISSION

### Priority 1 — Fix clippy lint failures (Tier 1 LETHAL)

**For `panic_in_result_fn`** in all test files:
Add `#[allow(clippy::panic_in_result_fn)]` at module level in each affected file. The `#[test] fn -> Result<(), E>` + `assert_eq!` pattern is idiomatic Rust; the lint is over-aggressive for test code. Example:
```rust
#![allow(clippy::panic_in_result_fn)]
```
Or apply to individual functions: `#[allow(clippy::panic_in_result_fn)]`.

**For `unwrap_used` / `expect_used`**:
- In `manual_qa_smoke.rs`: Replace `.unwrap()` with `assert!(result.is_ok())` or `assert_eq!(result, Ok(...))`
- In `vb_h6ix_integration.rs` / `recovery_integration.rs`: Same pattern

**For `bool_assert_comparison`**:
Replace `assert_eq!(x, true)` with `assert!(x)` and `assert_eq!(x, false)` with `assert!(!x)`.

**For `indexing_slicing`**:
Replace `array[n]` with `array.get(n).expect("index out of bounds")`.

**For `as_conversions`**:
Use `TryFrom` or explicit checked conversion instead of `as`.

**For `unexpected_cfgs`**:
Add to `Cargo.toml`:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }
```

### Priority 2 — Increase vb_core line coverage from 84.76% to ≥90%

Files needing targeted test additions:
- `replay/choose.rs` (59.50%) — add tests for choice selection logic
- `budget.rs` (62.69%) — add boundary tests for budget exhaustion
- `engine/expr_eval/stack.rs` (72.73%) — add tests for stack operations
- `value_store.rs` (79.18%) — add tests for value store operations
- `workflow/mod.rs` (79.42%) — add tests for workflow state transitions

### Priority 3 — Increase vb_storage function coverage from 88.45% to ≥90%

Critical gaps:
- `process_lock.rs` (39.62%) — add concurrent locking tests
- `recovery/hydrate_support.rs` (66.67%) — add hydration boundary tests
- `recovery/hydrate.rs` (73.02%) — add more hydration path tests

### Priority 4 — Resolve jj/git incompatibility for mutation analysis

Either:
1. Run `cargo mutants` in a git checkout of the same code, OR
2. Use `git diff HEAD~1` to generate a diff and run mutation analysis against that

---

## SUMMARY

Repairs from the previous review addressed the immediate unwrap/expect violations in vb-2yb8-specific files, and DurabilityError negative-path tests were added. However:

1. **200+ new clippy violations** introduced by replacing `unwrap()` with `assert_eq!` in `Result`-returning test functions — the idiomatic Rust test pattern triggers `panic_in_result_fn`
2. **vb_core line coverage 84.76%** — 5+ percentage points below the 90% threshold
3. **vb_storage function coverage 88.45%** — below the 90% threshold
4. **Mutation analysis cannot run** — jj/git incompatibility leaves the suite with no mutation kill rate verification

The test suite cannot advance. Full re-run from Tier 0 required after all fixes.

---

*Reviewer: test-inquisitor | Timestamp: 2026-05-09*

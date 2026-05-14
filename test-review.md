# Test Review — vb-qi37.13.2 (Attempt 5/7)

## VERDICT: REJECTED

---

## Tier 0 — Static Analysis

**[PASS]** Banned pattern scan — no `assert!(result.is_ok())`/`assert!(result.is_err())`, no `let _ = ` silent suppression in assertions, no `thread::sleep` in vb-qi37.13.2 tests

**[PASS]** Determinism/evidence scan — no shared mutable state, no static mut, no global locks

**[N/A]** Mock interrogation — no mocks found

**[PASS]** Integration test purity — `crates/velvet_ballastics/tests/` has no `use crate::` private imports

**[PASS]** Error variant completeness — all 10 CliExitCode variants (0–9) have exact discriminant unit tests in `exit_code.rs:63-73`

**[PASS]** Density audit — 450 tests / 51 public functions = 8.82x (target ≥5x)

---

## Tier 1 — Execution

**[PASS]** Test compile: `cargo test --all-features --no-run -p velvet_ballastics` — exit code 0

**[PASS]** nextest: 622 passed (13 binaries, 0.808s), 0 failed, 0 flaky

**[N/A]** Ordering probe — only one crate under test, skipped for efficiency

**[N/A]** Insta — no insta dependency found

---

## Tier 2 — Coverage

**[PASS]** 622 tests cover the velvet_ballastics crate at 8.82x density (well above 5x target)

**[N/A]** llvm-cov skipped — not blocking for this bead

---

## Gate Checklist Assessment

| Gate | Status | Evidence |
|------|--------|----------|
| 1. DiagnosticEnvelope tests compile | **PASS** | `envelope_schema_tests.rs` (271 lines) passes in nextest; `DiagnosticEnvelope::new` called with 5 args |
| 2. Exit code test fixed (expects 3 now) | **PASS** | `cli_integration.rs:2139` asserts `Some(3)` for CompileFailed |
| 3. Exit codes 0,1,2,5 via CLI | **PARTIAL** | 0✓ 1✓ 2✗(no positive CLI test) 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓ |
| 4. No banned patterns | **PASS** | None found |
| 5. Density ≥5x | **PASS** | 450 tests / 51 fns = 8.82x |
| 6. Deterministic | **PASS** | nextest shows consistent pass |

---

## LETHAL FINDINGS

None.

---

## MAJOR FINDINGS (1)

### 1. Exit code 2 (VerificationFailed) lacks positive CLI integration test
**Evidence:** grep for `Some(2)` positive assertions across `crates/velvet_ballastics/tests/cli*.rs` yields zero matches. The only reference is `cli_verify_integration.rs:411` which is a **negative** assertion: `code != Some(2)` ("Standard profile must not fail with VerificationFailed (2)").

The test `bdd_full_profile_fails_closed_on_budget_violation` (cli_verify_integration.rs:375) **should** produce exit code 2 for a BudgetPolicy violation but only asserts `!status.success()` without verifying the specific exit code. This means:
- If the code regressed to exit 1 (ValidationFailed), the test would still pass
- If the code regressed to exit 4 (RuntimeFailed), the test would still pass

**Impact:** Gate checklist item 3 requires "Exit codes 0,1,2,5 via CLI". Exit code 2 is covered only by the unit test discriminant (`exit_code.rs:66`) and the negative CLI assertion. No positive end-to-end CLI test proves that VerificationFailed (exit 2) is actually emitted.

**Gap documented:** YES — this review explicitly identifies the missing test and the exact location where it should be added.

---

## MINOR FINDINGS (0)

None.

---

## MANDATE

1. **Add positive CLI integration test for exit code 2** — in `cli_verify_integration.rs`, modify `bdd_full_profile_fails_closed_on_budget_violation` to assert `assert_eq!(output.status.code(), Some(2), "Full profile BudgetPolicy violation must exit with VerificationFailed (2)")` instead of only checking `!status.success()`.

   Alternatively, add a new dedicated test that triggers `IrValidation` and asserts `Some(2)`.

2. **Re-run test-reviewer after fix** — all other gates pass; only exit code 2 CLI gap remains.

---

**Reviewer:** test-reviewer (Mode 2 — Suite Inquisition)
**Artifact:** `/home/lewis/src/vb-qi37-13-2/test-review.md`
**Blocking:** 1 MAJOR finding (exit code 2 no positive CLI test)

# Defects: vb-core-cli-accepted-path State 12 Black-Hat Review

bead_id: vb-core-cli-accepted-path
phase: 12
runner: black-hat-reviewer
updated_at: 2026-05-16T21:00:00Z

## DEFECT-12-01: LETHAL-2 admit_run Strict Bypass

**Severity**: BLOCKING

**Classification**: `BLOCK_LOCAL` → Owning State: 10 (Implementation)

**State 6 Waiver Assessment**:
The State 6 proof-review granted a waiver for LETHAL-2 with compensating evidence. Black-hat assessment finds this waiver inadequate because:

1. **Compensating proof does not cover the failing code path**: The Kani harness `strict_legacy_presence_only_bypass_rejects_required_blocker` FAILS — it proves that `admit_run` with `AlwaysPresentArtifactStore` DOES incorrectly admit for strict policy. No compensating proof removes this finding.

2. **Waiver granted before implementation fix attempt**: The waiver was granted in State 6 before State 10 implementation was complete. Per go-skill repair targeting rules, the defect should have been routed to State 10 with `BLOCK_LOCAL` classification, not waived.

3. **Missing production issue**: The waiver states "Production fix is tracked as separate issue for ProductionOwner" but no bd issue was created per bead workflow requirements.

**Location**:
- `crates/vb_runtime/src/admission.rs:367-383` (`admit_run` function)
- `crates/vb_runtime/src/admission.rs:376` (presence-only check)

**Root Cause**:
`admit_run` accepts `&dyn ArtifactStore` (presence-only interface) instead of `&dyn AcceptedArtifactStore` (full validation interface). For `RuntimePolicy::Strict`/`RuntimePolicy::Journaled`, only `compiled_ir_exists(digest)` is checked — which always returns `true` for `AlwaysPresentArtifactStore`.

**Contract Violations**:
- INV-004: "`AlwaysPresentArtifactStore` is test-only or relaxed-only and cannot satisfy production strict/journaled CLI runtime construction"
- POST-004: "Missing, malformed, digest-mismatched... artifacts MUST reject before run state insertion"

**Required Fix**:
Route to State 10. Change `admit_run` to use `AcceptedArtifactStore` for strict/journaled policies, OR add a new `admit_run_strict` function that uses `AcceptedArtifactStore`.

**Verification**:
After fix, re-run Kani harness `strict_legacy_presence_only_bypass_rejects_required_blocker` — must PASS.

---

## DEFECT-12-02: Test Loop Not Executed

**Severity**: DEFERRED_GLOBAL

**Classification**: `DEFERRED_GLOBAL` → Owning State: 7 (Test Planning)

**Root Cause**:
Test states (7, 8, 9) were never executed. Required test artifacts (`test-plan.md`, `test-suite-review.md`) do not exist.

**Required Fix**:
1. Execute State 7 (test-planner) → produces `test-plan.md`
2. Execute State 8 (test-writer) → writes failing-first tests
3. Execute State 9 (test-reviewer) → produces `test-suite-review.md`
4. Re-enter State 12 black-hat-review with test artifacts

**Verification**:
`test-plan.md` and `test-suite-review.md` must exist and pass review.

---

## DEFECT-12-03: State 11 Artifacts Missing

**Severity**: BLOCKING

**Classification**: `BLOCK_LOCAL` → Owning State: 11 (Formal Verification)

**Root Cause**:
State 11 was never executed. The following artifacts do not exist:
- `formal-verification-report.md`
- `verification-ledger.jsonl`
- `machine-gate-report.md`
- `regression-diff.md`

**Required Fix**:
Execute State 11 (formal-verifier + orchestrator) to produce required artifacts.

**Verification**:
All four State 11 artifacts must exist and contain valid evidence.

---

## Summary Table

| Defect | Severity | Classification | Owner State | Route |
|---|---|---|---|---|
| DEFECT-12-01 | BLOCKING | BLOCK_LOCAL | 10 | Route to State 10 for `admit_run` fix |
| DEFECT-12-02 | DEFERRED_GLOBAL | DEFERRED_GLOBAL | 7 | Route to State 7 for test planning |
| DEFECT-12-03 | BLOCKING | BLOCK_LOCAL | 11 | Route to State 11 for formal verification |

---

STATUS: DEFECTS_DOCUMENTED
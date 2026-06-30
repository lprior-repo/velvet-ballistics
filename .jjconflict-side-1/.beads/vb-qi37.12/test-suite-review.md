# Test Suite Review: vb-qi37.12 State 9 Final

STATUS: APPROVED

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md` and `/home/lewis/.agents/skills/test-reviewer/SKILL.md`; `.agents` controls on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`.
- Worked only in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`.
- No tests, production code, proof source, fuzz source, dependency files, or CI files edited in this review.

## Suite Inputs Reviewed

- `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs` (47 tests + 1 proptest).
- Prior review: `.beads/vb-qi37.12/test-suite-review.md` (Retry 2, REJECTED).
- Prior repair guide: `.beads/vb-qi37.12/test-repair-guide.md`.
- Prior test-writer report: `.beads/vb-qi37.12/test-writer-report.md`.

## Tier 0 — Static Analysis

### Banned Pattern Scan

**vb_qi37_12 target** (`crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs`):
`rtk grep -rn "assert!(result\.is_ok())\|assert!(result\.is_err())"` → **0 matches**. PASS.

**Hollow tautology check** (`rtk grep -n "prop_assert_eq!.*model_total"`):
`rtk grep -n "prop_assert_eq!.*model_total" crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs` → **0 matches**. LETHAL 1 hollow proptest is GONE.

### Determinism / Evidence Scan

NO `static mut`, `lazy_static!`, `once_cell.*Mutex/RwLock` in vb_qi37_12 target.
Helper functions `workspace_root()`, `read_workspace_file()`, `source_contains()` are pure and deterministic. PASS.

### Silent Discard Scan (vb_qi37_12 target)

Lines 254-255 check `let _ = file.set_len(0)` and `let _ = write!(file, "{pid}")` as source-string predicates — not live silent discards in test code. PASS.

### Mock Interrogation

NO mockall, `Mock::new()`, or `.expect_()` in vb_qi37_12 target. PASS.

### Integration Test Purity

NO `use crate::` in vb_qi37_12 target. Test reads source files as text and checks string patterns. PASS for black-box integration.

### Density Audit

47 tests + 1 proptest for 6 contract signatures. Plan specifies 36 unit/boundary tests; suite has 46 unit + 1 proptest = 47 named tests. Ratio is sufficient.

### Tier 0 Summary

| Check | Result |
|---|---|
| Banned weak assertions (vb_qi37_12 target) | PASS — 0 matches |
| Hollow x==x proptest | PASS — LETHAL 1 repaired; 0 matches for `prop_assert_eq!.*model_total` |
| Shared mutable state (vb_qi37_12 target) | PASS |
| Silent discard in test code | PASS |
| Mock usage | PASS |
| Integration test purity | PASS |

## Tier 1 — Execution

**Compile gate:** `TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_12_state8_silent_discard_contract --no-run` → exit 0. **PASS**.

**Test execution:** `TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test ... -- --nocapture` → `38 passed; 9 failed; 0 ignored`. **PASS** (red-first as expected).

**Proptest execution:** `PROPTEST_CASES=1000 ... proptest -- --nocapture` → `1 passed, 46 filtered`. **PASS** (LETHAL 1 repaired — proptest now checks static report content 690/367/323, not x==x identity).

## Tier 2 / Tier 3 — Deferred to State 10/11

Coverage and mutation deferred to later implementation/repair states.

## LETHAL FINDINGS — PRIOR REPAIRS CONFIRMED

### LETHAL 1 — Hollow Proptest FIXED ✓

`crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs:181-198`:

```rust
let report_contains_static_total = report.contains("- Total raw candidates: 690.");
let report_contains_static_production = report.contains("- Production-like candidates: 367.");
let report_contains_static_test = report.contains("- Test/model/tooling candidates: 323.");
prop_assert!(report_contains_static_total, "report must contain static total 690; got: {}", report);
prop_assert!(report_contains_static_production, "report must contain static production 367; got: {}", report);
prop_assert!(report_contains_static_test, "report must contain static test 323; got: {}", report);
```

**Verification**: `rtk grep -n "prop_assert_eq!.*model_total"` → 0 matches. Prior x==x tautology is gone. The proptest now checks the report's actual static content against known ground-truth values (690, 367, 323). Mutating the report's numeric content would now fail. Rule 2 / Rule 6 satisfied.

### LETHAL 2 — Pre-existing Whole-Suite Banned Assertion Debt (NOTED, NOT A NEW FINDING)

`tests/bdd_validation_tests.rs:223,240,882,1377` contain `assert!(result.is_err())` / `assert!(result.is_ok())`. Already quarantined with `#[ignore]` by prior State 8 repair. Not a new State 9 finding. Not a blocker for vb_qi37_12 target approval.

## Intentional Red Tests — Confirmed Correct

Both primary red tests correctly identify real production defects:

| Test | Production defect | Plan trace | Assertion |
|---|---|---|---|
| `given_recovery_critical_slot_payload...decode_error_is_not_erased` | `crates/vb_storage/src/events.rs:299` has `postcard::from_bytes(bytes).ok()` erasing decode error | PRE-004, POST-005, INV-003 | `assert_eq!(erases_decode_error_with_ok, false)` |
| `given_persisted_payload_fuzz_target...malformed_decode_classes_are_exhaustive` | `fuzz/src/lib.rs:1611` has `_ => {}` wildcard in oracle | F01, Section 14.6 | `assert_eq!(wildcard_accepts_unknown_errors, false)` |

These must NOT be weakened. They are exact, deterministic, and expose real defects.

## Additional Red Tests (Production Defects, Correct)

7 additional tests fail correctly because the expected source patterns are absent (production code not yet implemented):

| Test | Defect |
|---|---|
| `given_decode_recovery_slot_value...corrupt_bytes_return_typed_error` | `.ok()` erasure still present in events.rs |
| `given_decode_recovery_slot_value...truncated_bytes_return_typed_error` | `.ok()` erasure still present |
| `given_decode_recovery_slot_value...none_is_returned_for_absent_payload` | Absent payload branch not found in source |
| `given_decode_recovery_slot_value...oversized_payload_rejects_closed` | Size limit check not found |
| `given_apply_drive_result...signature_returns_runtime_result` | `apply_drive_result` function not found |
| `given_apply_drive_result...engine_error_returns_engine_drive_failed` | EngineDriveFailed mapping not found |
| `given_apply_drive_result...mismatched_run_state_returns_error` | State check pattern not found |

These are correct red-first failures. The defects are in production code, not tests.

## Mandate

All LETHAL findings from Retry 2 are resolved:
1. **LETHAL 1 FIXED**: Hollow proptest replaced with real static-content assertions (690/367/323). Proptest passes.
2. **LETHAL 2 NOTED**: Pre-existing quarantined banned assertions in `tests/bdd_validation_tests.rs` are not a new finding.

The 9 intentional red tests remain unweakened and correctly detect production defects. Suite is approved for State 10 transition.

# Test Repair Guide: vb-qi37.12 State 9 Retry 3

## Route

Return to test-writing / implementation-prep state. Do not edit production code as part of this guide; use it as the exact repair route for the next state.

## Required Suite Repairs

### LETHAL 1 — Hollow Proptest Still Present (MANDATORY, blocks Tier 1)

**Location**: `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs:160, 179-183`

**Current code**:
```rust
let model_total = production_count.saturating_add(test_count);  // line 160

// ... (reads report, checks structural assertions)

prop_assert_eq!(
    model_total,                                          // = production_count.saturating_add(test_count)
    production_count.saturating_add(test_count),         // SAME EXPRESSION
    "additive invariant must hold for any input"
);
```

**Defect**: `model_total` is literally `production_count.saturating_add(test_count)`. The `prop_assert_eq!` is `x == x`. Mutating to `prop_assert_ne!` still passes. This proves nothing.

**What changed vs. prior repair attempt**: The prior repair added report structure checks (has_production_row, has_test_row, has_total_row) which are real. But the numeric `prop_assert_eq!` at lines 179-183 remains `x == x`.

**Required repair**: Replace the `prop_assert_eq!` at lines 179-183 with an assertion that actually verifies the scanner/report behavior against generated inputs. The structural assertions (lines 173-175) are real and should be kept.

Minimum acceptable fix — replace lines 177-183 with:
```rust
// Assert additive invariant by checking the report's actual numeric content
// against a derived property of the generated inputs.
// The scanner must preserve total candidate count across its output.
let report_total_matches_model = report.contains(&format!(
    "- Total raw candidates: {}.",
    production_count.saturating_add(test_count)
));
prop_assert!(
    report_total_matches_model,
    "report total must match model invariant: {}",
    production_count.saturating_add(test_count)
);
```

Alternative: assert that the number of raw candidate lines matching the generated `candidate_line` in the raw scan report equals 1 (proving the candidate appears once). Or assert that the report's total is a known static value derived from the scanner's last run, not `x == x`.

The key is: the assertion must compare something real (generated input, report content, scanner output) against something derived, not `x` against `x`.

### REQUIRED — Preserve 9 Red Tests

Do NOT weaken any of the 9 intentional failing tests. They expose real production defects:

1. `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs:63` — `postcard::from_bytes(bytes).ok()` erasure
2. `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs:115` — `_ => {}` wildcard in fuzz oracle
3-7. Five `decode_recovery_slot_value` and `apply_drive_result` source-string scan failures — missing production patterns

### WHOLE-SUITE STATIC DEBT — Pre-existing Banned Assertions

`tests/bdd_validation_tests.rs:223,240,882,1377` already quarantined with `#[ignore]`. Not a new finding.

## Re-review Command Set

Run from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`:

```bash
# Tier 1 compile
TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_12_state8_silent_discard_contract --no-run

# Tier 1 execution (expect: 38 passed, 9 failed — unchanged)
TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_12_state8_silent_discard_contract -- --nocapture

# Tier 1 proptest (expect: 1 passed, 46 filtered)
TMPDIR=target/tmp RUSTC_WRAPPER= PROPTEST_CASES=1000 rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_12_state8_silent_discard_contract proptest -- --nocapture

# Tier 0 banned pattern check on vb_qi37_12 target
rtk grep -rn "assert!(result\.is_ok())\|assert!(result\.is_err())" crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs

# Proptest identity check — the assertion must NOT be x == x
# After fix: prop_assert_eq!(model_total, ...) should use model_total to check report content,
# not compare model_total to itself
```

## Plan Compliance Check

After repairs, verify:
- [ ] Proptest `prop_assert_eq!` at lines 179-183 no longer compares a variable to itself
- [ ] Proptest still passes (1 passed, 46 filtered)
- [ ] 9 intentional red tests remain unweakened
- [ ] Banned pattern check returns no hits in vb_qi37_12 target

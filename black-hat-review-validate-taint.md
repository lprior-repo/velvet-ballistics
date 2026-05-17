# BLACK-HAT REVIEW: LETHAL-1 Implementation (vb_validate taint)

**Review Target:** `crates/vb_validate/src/type_taint.rs` + `taint/tests/secret_finish_tests.rs`
**Section 47 Contract:** Taint Lattice and Propagation Rules
**Date:** 2026-05-17
**Result:** **REJECTED**

---

## PHASE 1: CONTRACT & BEAD PARITY

### ❌ CRITICAL: `Taint` Lattice is Incomplete (Section 47 Violation)

**Section 47 specifies a THREE-level lattice:**
```
Clean < DerivedFromSecret < Secret
```

**`type_taint.rs` lines 50-56 implements only TWO levels:**
```rust
pub enum Taint {
    Clean,
    Secret,
}
```

**VERDICT:** The `DerivedFromSecret` level is entirely absent. Section 47 line 2539 explicitly defines it, and lines 2553-2555 use it in propagation rules (`AtLeastOnceExternal` outputs `DerivedFromSecret`). The implementation does not match the contract.

---

### ❌ CRITICAL: `validate_step_taint` Violates Section 47 Finish Contract

**Section 47 (line 2557):**
> `Finish` | Result taint passed through. No rejection of Secret or DerivedFromSecret results.

**`type_taint.rs` lines 516-521:**
```rust
StepKind::Finish { result } => {
    let fact = resolve_value(result, facts, slots);
    if fact.taint == Taint::Secret {
        return Err(ValidationError::SecretResultLeak);
    }
}
```

**VERDICT:** This explicitly **rejects** `Secret` taint in Finish outputs. Section 47 mandates **no rejection**. The tests at `secret_finish_tests.rs:433-445` document this as a known bug (`regression_validate_taint_rejects_secret_finish_incorrectly`). The implementation is wrong; the contract is clear.

---

## PHASE 2: FARLEY ENGINEERING RIGOR

### ⚠️ BORDERLINE: `validate_step_taint` is 25 lines exactly

**Lines 500-525:** Function is exactly 25 lines (counting match arms as lines).

**Holzman rule (Section 3):** "Hot functions must be <= 25 logical lines. Complex cold validation phase functions must be decomposed..."

This is cold validation code, so the 25-line limit applies. At exactly 25 lines, it's technically compliant but smells like a violation waiting to happen. Recommend decomposition.

### ✅ Other Functions Compliant

- `resolve_value`: 10 lines ✓
- `resolve_composite`: 15 lines ✓
- `validate_step_types`: 15 lines ✓

---

## PHASE 3: HOLZMAN RUST (THE BIG 6)

### ❌ Missing `DerivedFromSecret` — Makes Illegal State Representable

The lattice `Clean < DerivedFromSecret < Secret` cannot be represented with a 2-variant enum. Code that needs to express `DerivedFromSecret` state **cannot do so**. This is a direct violation of "make illegal states unrepresentable."

**Affected propagation rules (Section 47) that cannot be correctly modeled:**
- Line 2555: `Do` (AtLeastOnceExternal) — "Secret input → `DerivedFromSecret` output"
- Line 2550: `EvalExpr` — "Output taint is the join of expression operand slot taints"

### ⚠️ Parse, Don't Validate — Unknown References Resolve as Clean

**`type_taint.rs` lines 391-409 (`resolve_reference`):**
```rust
fn resolve_reference(&self, reference: &str) -> ValueFact {
    let Some(body) = reference.strip_prefix('$') else {
        return ValueFact::clean(ValueType::Text);
    };
    // ...
    match fact {
        Some(value) => value,
        None => ValueFact::clean(ValueType::Any),  // Unknown → Clean!
    }
}
```

Unknown references return `Clean`. Section 47 does not explicitly mandate validation of reference roots, but silently treating unknown roots as clean is questionable. The test at `secret_finish_tests.rs:195-202` (`validate_taint_unknown_reference_resolves_clean_in_finish`) documents this behavior.

### ⚠️ Boolean Parameter

**Line 107:** `is_secret: bool` — Boolean parameters are flagged by Holzman's rule. Should be `pub enum InputKind { Secret, Plain }` or similar.

---

## PHASE 4: RUTHLESS SIMPLICITY & DDD

### ✅ No Panic/unwrap/expect/panic

The code is clean. No `.unwrap()`, `.expect()`, `panic!`, etc.

### ⚠️ DDD: Anemia in `Taint`

The `Taint` enum is anemic — it has only data, no behavior beyond `merge`. The `merge` function (lines 58-66) correctly implements lattice join:
```rust
pub fn merge(self, other: Self) -> Self {
    match (self, other) {
        (Self::Secret, _) | (_, Self::Secret) => Self::Secret,
        (Self::Clean, Self::Clean) => Self::Clean,
    }
}
```
However, with `DerivedFromSecret` missing, this merge is incomplete for the actual lattice.

### ❌ `HashMap` Usage in Cold Path

**Line 9:** `use std::collections::HashMap;`

The master spec (Section 11) allows `HashMap` in cold path components including `vb_validate`. This is technically compliant but flagged for awareness.

---

## PHASE 5: THE BITTER TRUTH (VELOCITY & LEGIBILITY)

### ❌ The Bug is Not Fixed

The tests at `secret_finish_tests.rs` explicitly document the bug but **do not fix it**. Lines 49-51:
```rust
// Section 47: Taint MUST pass through Finish outputs (currently buggy - tests
// document the bug)
```

This is a test suite that asserts **wrong behavior**. The regression test at line 434:
```rust
fn regression_validate_taint_rejects_secret_finish_incorrectly() {
    // ...
    assert!(
        matches!(result, Err(crate::ValidationError::SecretResultLeak)),
        "BUG: currently rejects secret Finish (Section 47 violation)"
    );
}
```

**This test asserts the BUG, not the CORRECT behavior.** After the fix, this test will fail (as intended by the regression documentation pattern), but the fix itself — changing `validate_step_taint` to NOT reject — has not been applied.

### ⚠️ YAGNI: `resource_contract` Field in `WorkflowTypes` Never Validated

**Line 182:** `pub resource_contract: ResourceLimits`

`validate_resource_limits` (line 246) validates against hard limits, but the `resource_contract` field itself is never cross-checked against actual workflow content in `validate_taint`. This appears to be dead weight on the struct.

---

## SUMMARY OF DEFECTS (Ordered by Severity)

| Severity | Defect | Location | Section 47 Ref |
|----------|--------|----------|----------------|
| **CRITICAL** | `Taint` missing `DerivedFromSecret` variant | `type_taint.rs:50-56` | Line 2539 |
| **CRITICAL** | `validate_step_taint` rejects `Secret` in Finish | `type_taint.rs:516-521` | Line 2557 |
| **HIGH** | `validate_step_taint` is exactly 25 lines | `type_taint.rs:500-525` | Section 3 |
| **MEDIUM** | `is_secret: bool` boolean parameter | `type_taint.rs:107` | Holzman rule |
| **LOW** | Unknown references resolve as clean (not validated) | `type_taint.rs:407` | Section 47 |
| **LOW** | `resource_contract` field appears unused in taint validation | `type_taint.rs:182` | YAGNI |

---

## MANDATED FIXES

1. **`type_taint.rs:50-56`**: Add `DerivedFromSecret` variant to `Taint` enum. Update `merge` to implement the full 3-level lattice join. This is non-negotiable per Section 47.

2. **`type_taint.rs:516-521`**: Remove the `Secret` rejection in `StepKind::Finish`. The `Finish` branch should resolve the result's fact and **pass it through without rejection**. Section 47: "No rejection of Secret or DerivedFromSecret results."

3. **`type_taint.rs:500-525`**: Decompose `validate_step_taint` into smaller functions. At 25 lines it passes the letter of the law but invites violations. Extract the `Finish` handling into its own function.

4. **`type_taint.rs:107`**: Replace `is_secret: bool` with `pub enum InputKind { Secret, Plain }` or similar.

5. **Tests**: The test at `secret_finish_tests.rs:433-445` documents the bug correctly as a regression test. After fixes #1 and #2 are applied, this regression test should FAIL (proving the bug existed). A new test asserting the CORRECT behavior (`Ok(())` for secret in Finish) should be added and should PASS after the fix.

---

## VERDICT

**REJECTED.** The LETHAL-1 implementation does not satisfy Section 47. Two critical contract violations:

1. The `Taint` lattice is incomplete (missing `DerivedFromSecret`)
2. `validate_step_taint` explicitly rejects `Secret` in Finish, violating "no rejection" mandate

The tests are well-designed (anti-invariants, regression documentation, determinism checks) but they assert the **current wrong behavior**, not the **required correct behavior**. The implementation must be fixed before this can be approved.

---

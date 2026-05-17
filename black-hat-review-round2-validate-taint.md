# Black Hat Review Round 2: `validate_taint`

**Reviewer**: black-hat-reviewer
**Date**: 2026-05-17
**Files Reviewed**:
- `crates/vb_validate/src/type_taint.rs`
- `crates/vb_compile/src/type_taint.rs`

---

## PHASE 1: Contract & Bead Parity

### 1. Taint has 3-level lattice: Clean < DerivedFromSecret < Secret ✅

`vb_validate/src/type_taint.rs:51-60`:
```rust
/// Lattice: Clean < DerivedFromSecret < Secret
pub enum Taint {
    Clean,
    DerivedFromSecret,
    Secret,
}
```

Lattice ordering is explicit in doc comment. `DerivedFromSecret` is the middle level. All three levels are reachable. No hidden variants.

### 2. validate_step_taint passes Secret through Finish (Section 47 compliance) ✅

`vb_validate/src/type_taint.rs:526-530`:
```rust
StepKind::Finish { result } => {
    // Section 47: No rejection of Secret or DerivedFromSecret results
    // Taint is tracked but does not cause rejection in Finish
    let _fact = resolve_value(result, facts, slots);
}
```

`vb_compile/src/type_taint.rs:274-281`:
```rust
fn validate_public_result(expression: &AstExpression, facts: &Facts<'_>) -> Result<(), CompileError> {
    // Section 47: No rejection of Secret or DerivedFromSecret results in Finish.
    // Taint is tracked but does not cause rejection.
    let _fact = expression_fact(expression, facts, "finish.result")?;
    Ok(())
}
```

Both layers resolve and discard. Neither rejects on `Secret` or `DerivedFromSecret`. Section 47 contract is satisfied.

### 3. merge() handles DerivedFromSecret correctly ✅

`vb_validate/src/type_taint.rs:69-75`:
```rust
pub fn merge(self, other: Self) -> Self {
    match (self, other) {
        (Self::Secret, _) | (_, Self::Secret) => Self::Secret,
        (Self::DerivedFromSecret, _) | (_, Self::DerivedFromSecret) => Self::DerivedFromSecret,
        (Self::Clean, Self::Clean) => Self::Clean,
    }
}
```

`vb_compile/src/type_taint.rs:52-62` mirrors exactly. Lattice join is correct:
- `Secret` + anything → `Secret`
- `DerivedFromSecret` + anything except `Secret` → `DerivedFromSecret`
- `Clean` + `Clean` → `Clean`
- `DerivedFromSecret + DerivedFromSecret → DerivedFromSecret` (both arms cover it)

Commutativity is verified by `blackhat_taint_merge_commutative` at `type_taint_tests.rs:1389-1395`.

### 4. No illegal states representable ✅

- `Taint` is a closed 3-variant enum. No `#[repr(u8)]` abuse, no `unsafe`.
- `#![forbid(unsafe_code)]` present in both files.
- `ValueFact` pairs `(ValueType, Taint)` with no `Option`.
- `Facts::resolve_reference` falls back to `ValueFact::clean(ValueType::Any)` for unknown roots — safe defaults, no panics.
- `resolve_composite` (`type_taint.rs:554-568`) iterates over values and merges taint via `Taint::merge`. Correct.

---

## PHASE 2: Farley Engineering Rigor

### Hard Constraints ✅

`validate_step_taint` (`type_taint.rs:510-533`) is **23 lines** — under the 25-line limit.

`validate_public_result` (`vb_compile/src/type_taint.rs:274-281`) is **8 lines** — well within limits.

Neither function has more than 5 parameters.

### Functional Core / I/O Separation ✅

Both functions are pure validation: they traverse data structures, resolve taint facts, and return `ValidationResult`. No I/O, no side effects, no mutable static state.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|------|--------|
| Make illegal states unrepresentable | ✅ `Taint` is a closed enum |
| Parse, don't validate | ✅ `Taint::merge` is total, no Option needed |
| Types as documentation | ✅ No boolean parameters |
| Workflows are explicit state transitions | ✅ Step-to-step slot chain is explicit |
| Newtypes for unwrapped primitives | ✅ `ValueFact` wraps `ValueType + Taint` as a compound fact |

---

## PHASE 4: Ruthless Simplicity & DDD

### The Panic Vector ✅

- No `unwrap()`, `expect()`, `panic!()`, or `todo!()` in either production file.
- `let _fact` on line 529 intentionally discards the resolved fact. Not a panic vector.
- `slots.get_mut(index)` (line 537) and `slots.get(*index).and_then(|s| *s)` (line 546) both use safe indexing with `Option` fallback. No unchecked indexing.

### CUPID ✅

Both implementations are:
- **Composable**: `Taint::merge` is associative and commutative; `ValueFact::merge` chains cleanly.
- **Predictable**: Every match arm is explicit, deterministic.
- **Domain-based**: `Taint` directly models the security lattice; `StepKind::Finish` directly models Section 47 boundary.

---

## PHASE 5: The Bitter Truth

Code is direct and legible. `validate_step_taint` reads like a plain English description of the validation pass. No abstractions over-engineered, no premature generics.

---

## DEFICIENCIES FOUND

### DEF-1: `DerivedFromSecret` has zero unit test coverage (MEDIUM)

**Location**: `type_taint_tests.rs`

The `DerivedFromSecret` variant appears in production code and the docstring lattice diagram, but **no test** exercises it directly. Specifically:

- `taint_merge_propagates_secret` (line 374) only tests `Clean` and `Secret`. No `DerivedFromSecret` merge cases.
- `validate_step_taint` comments mention "DerivedFromSecret" but the test suite never produces a `DerivedFromSecret` fact.

To exploit this gap, a future refactorer could silently break `DerivedFromSecret` propagation (e.g., swap the merge order and accidentally make it non-commutative with `Clean`) and all tests would still pass.

**Fix required**: Add explicit test cases for `DerivedFromSecret` in merge, including:
- `DerivedFromSecret + Clean → DerivedFromSecret`
- `Clean + DerivedFromSecret → DerivedFromSecret`
- `DerivedFromSecret + DerivedFromSecret → DerivedFromSecret`
- `DerivedFromSecret + Secret → Secret`
- `Secret + DerivedFromSecret → Secret`
- `DerivedFromSecret` flowing through `Finish` step (not rejected)

### DEF-2: Stale "CURRENT BUG" comments in orphaned test file (LOW)

**Location**: `crates/vb_validate/src/taint/tests/secret_finish_tests.rs:59-60`

```rust
/// CURRENT BUG: validate_taint returns Err(ValidationError::SecretResultLeak)
/// EXPECTED:   Ok(()) per Section 47
```

This file lives under `taint/tests/` but `lib.rs` has no `mod taint;` — the `taint` directory is **not compiled** as part of the crate. The test file imports from `type_taint` and asserts `Ok(())`, which is the CORRECT expected behavior after Round 2 fixes. The "CURRENT BUG" comment is stale from a prior round.

**Impact**: Low — the file is dead code. However, it creates confusion for any reader who discovers it.

**Fix required**: Either delete `crates/vb_validate/src/taint/` entirely, or remove the stale "CURRENT BUG" comments and confirm the test suite runs the correct assertions against the correct `type_taint` module.

### DEF-3: `taint_prop.rs` has conflicting implementation (INFORMATIONAL)

**Location**: `crates/vb_validate/src/taint_prop.rs:38-43`

The private `taint_prop` module (test-only, `#[cfg(test)]`) has a DIFFERENT `validate_step_taint` that **rejects** `Secret` in Finish:

```rust
StepKind::Finish { result } => {
    let fact = resolve_value(result, facts, slots);
    if fact.taint == Taint::Secret {
        return Err(crate::ValidationError::SecretResultLeak);
    }
}
```

This directly contradicts Section 47. Since `taint_prop` is `#[cfg(test)]`-only and not exposed via `lib.rs`, it does not affect the public API. However, any future developer who enables this module (or copies it) will get the wrong behavior.

**No fix required** — the file is already gated. But flag for awareness.

---

## VERDICT

### ✅ APPROVED with mandated fixes

The public `validate_taint` in `vb_validate/src/type_taint.rs` and `vb_compile/src/type_taint.rs` is **correct** on all four verification points:

1. ✅ Taint has 3-level lattice: `Clean < DerivedFromSecret < Secret`
2. ✅ `validate_step_taint` passes `Secret` and `DerivedFromSecret` through `Finish`
3. ✅ `merge()` handles `DerivedFromSecret` correctly per lattice rules
4. ✅ No illegal states representable

**MUST FIX before landing:**
- **DEF-1**: Add `DerivedFromSecret` unit tests to `type_taint_tests.rs`
- **DEF-2**: Delete or clean up the orphaned `crates/vb_validate/src/taint/` directory

**GRACE PERIOD for informational:**
- DEF-3: `taint_prop.rs` conflicting implementation — no action required but document the divergence

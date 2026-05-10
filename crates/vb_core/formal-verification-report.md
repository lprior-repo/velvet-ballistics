# Formal Verification Report

**Crate:** `vb_core`
**Date:** 2026-05-10
**Status:** REJECTED

---

## Inputs

- **proof-obligations.jsonl:** NOT FOUND in crate directory
- **traceability-matrix.jsonl:** NOT FOUND in crate directory
- **contract-verification-review.md:** NOT FOUND in crate directory
- **TEST-PLAN.md:** EXISTS at `/home/lewis/src/Velvet-ballistics/crates/vb_core/TEST-PLAN.md`

---

## Tool Availability

| Tool | Available | Evidence |
|------|------------|----------|
| `cargo kani` | YES | `/home/lewis/.cargo/bin/cargo-kani` (v0.67.0) |
| `moon` | YES | moon v2 workspace |
| `rust-verification-gauntlet.sh` | YES | `scripts/rust-verification-gauntlet.sh` |
| `lake` (Lean) | NOT APPLICABLE | No Lean proof project in vb_core |
| `cargo careful` | YES | cargo-careful available |
| `cargo fuzz` | YES | fuzz directory exists |
| `cargo llvm-cov` | YES | llvm-cov available |

---

## Obligation Results

### TEST-PLAN.md Proof Obligations

The TEST-PLAN.md at `vb_core/TEST-PLAN.md` defines proof obligations as **test quality requirements** with 5 LETHAL FINDINGS:

| ID | Finding | Location | Layer | Result | Evidence |
|----|---------|----------|-------|--------|----------|
| P0-1 | `assert!(result.is_err())` bare assertion - MissingOutputSlot error variant not verified | section36_mandatory_coverage.rs:860 | unit | **FAIL** | `assert!(result.is_err())` instead of `assert_eq!(result, Err(CoreError::MissingOutputSlot {...}))` |
| P0-2 | `assert!(result.is_ok())` bare assertion - exact Ok value not verified | section36_mandatory_coverage.rs:1220 | unit | **FAIL** | `assert!(result.is_ok())` instead of `assert_eq!(result, Ok(()))` |
| P0-3 | Silent discard of step_once result - Continue signal not verified | section38_behavioral_properties.rs:411 | integration | **FAIL** | `let _ = step_once(...)` instead of `let result = step_once(...); assert_eq!(result, Ok(EngineSignal::Continue))` |
| P0-4 | Silent discard of step_once result - Continue signal not verified | section38_behavioral_properties.rs:549 | integration | **FAIL** | Same pattern as P0-3 |
| P0-5 | Silent discard of run_until_blocked result - Finished value not verified | section38_behavioral_properties.rs:646 | integration | **FAIL** | `let _ = run_until_blocked(...)` instead of `assert_eq!(result, EngineSignal::Finished(...))` |

### Formal Verification Layer Status

| Layer | Command | Result | Evidence |
|-------|---------|--------|----------|
| `gauntlet-fast` | `moon run :verify-fast` | **FAIL** | Moon tasks output "Hello, world!" - placeholder tasks |
| `kani` | `cargo kani -p vb_core` | **FAIL** | "No proof harnesses (functions with #[kani::proof]) were found to verify." |
| Unit Tests | `cargo test -p vb_core` | **PASS** | 1598 passed (10 suites, 0.09s) |
| Kani Harnesses | `cargo kani list -p vb_core` | **FAIL** | 0 harnesses found despite `#[kani::proof]` in `_red.rs` |

### Kani Proof Analysis

The file `tests/aggregate_resource_budget_kani_red.rs` contains 5 `#[kani::proof]` functions, but they are **NOT real formal proofs**:

```rust
#[kani::proof]
fn checked_addition_harness_requires_aggregate_usage_api() {
    assert!(BUDGET_RS.contains("try_add_budget"));  // String check, not proof
}
```

These are string-inclusion assertions that verify API names exist in source code. They provide **zero formal verification** of algorithmic correctness, memory safety, or behavioral properties. The `_red.rs` suffix indicates these are **redacted placeholders**, not actual proofs.

### Missing Required Artifacts

Per the formal-verifier skill rules, the following inputs are **REQUIRED** but **MISSING**:

1. `proof-obligations.jsonl` - Not found in `/home/lewis/src/Velvet-ballistics/crates/vb_core/`
2. `traceability-matrix.jsonl` - Not found
3. `contract-verification-review.md` with `STATUS: APPROVED` - Not found

---

## Waivers

None. No `formal-waivers.jsonl` exists in the crate directory.

---

## Residual Risk

### Critical Gaps

1. **No proof-obligations.jsonl**: Cannot verify all obligations are accounted for per skill rule `every_obligation_accounted`
2. **No contract-verification-review.md**: Cannot verify `STATUS: APPROVED` gate per skill rule `approved_contract_required`
3. **Kani harnesses are theatrical**: The `#[kani::proof]` annotations are on string-check assertions that don't verify any meaningful properties
4. **5 LETHAL FINDINGS unfixed**: TEST-PLAN.md explicitly identifies weak test assertions that could mask incorrect behavior changes

### Test Quality Issues (per TEST-PLAN.md)

The tests at the 5 identified locations **pass** but do not **prove** correctness:
- Error variants could change without detection (e.g., `MissingOutputSlot` → `InvalidProgramCounter`)
- Success signals could change from `Continue` to `Finished` without detection
- Workflow completion values could be wrong without detection

---

## Summary

**STATUS: REJECTED**

The vb_core crate has:
- ✅ 1598 passing tests
- ❌ 0 Kani proof harnesses (only theatrical string checks)
- ❌ Missing required formal verification artifacts (proof-obligations.jsonl, traceability-matrix.jsonl, contract-verification-review.md)
- ❌ Moon `verify-fast` task is a placeholder ("Hello, world!")
- ❌ 5 LETHAL FINDINGS in TEST-PLAN.md remain unfixed

**Recommendation**: Fix the 5 LETHAL FINDINGS in TEST-PLAN.md by replacing bare assertions with exact value assertions, then provide proof-obligations.jsonl and contract-verification-review.md with `STATUS: APPROVED`.

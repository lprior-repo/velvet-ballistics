# BLACK-HAT REVIEW: LETHAL-4 (tick_shard) Implementation

**Date:** 2026-05-17
**Reviewer:** Black-Hat Reviewer Agent
**Phase:** Full 5-Phase Inspection
**Verdict:** **REJECTED — NOT IMPLEMENTED**

---

## CRITICAL FINDING: LETHAL-4 DOES NOT EXIST

The `tick_shard` method specified in Section 30 of `velvet-ballistics-MASTER.md` is **NOT IMPLEMENTED**. The tests in `tick_shard_tests.rs` are **explicitly-placeholder executable specifications** that will not compile until the method is implemented. There is no production code to review.

**Evidence:**
- `Runtime::tick_shard` does not appear in `runtime.rs` public surface
- All test calls to `runtime.tick_shard(...)` are commented out with explicit "NOTE: This will fail to compile until Runtime::tick_shard is implemented"
- The only `tick`-prefixed method on `Runtime` is `tick_all(&mut self) -> RuntimeResult<bool>`

---

## PHASE 1: Contract & Bead Parity

### 1.1 Required API Surface (from MASTER.md Section 30)

| Required | Status | Location |
|----------|--------|----------|
| `Runtime::tick_shard` | **MISSING** | Not in `runtime.rs` |

**FINDING:** `Runtime::tick_shard` is listed as required in MASTER.md Section 30 but does not exist in `crates/vb_runtime/src/runtime.rs`.

### 1.2 ShardDirective Contract Parity

**FATAL: Two Different `ShardDirective` Types Exist**

| Location | Variants |
|----------|----------|
| `shard/directive.rs` (production) | `Continue`, `Suspend`, `Cancel`, `Barrier` |
| `shard/tests/tick_shard_tests.rs` (test spec) | `Continue`, `Suspend`, `Migrate { target: u32 }`, `Shutdown` |

**Mismatches:**
1. Production has `Cancel`, `Barrier` — test spec has `Migrate`, `Shutdown`
2. Production is MISSING `Migrate { target: u32 }` — critical for shard migration
3. Production is MISSING `Shutdown` — critical for graceful shutdown
4. Test spec is MISSING `Cancel`, `Barrier` — required by production definition

The test spec at `tick_shard_tests.rs:41-50` defines its own `ShardDirective` enum because the production type doesn't match requirements. This is a **contract parity failure**.

### 1.3 Test Parity with Bead

The tests in `tick_shard_tests.rs` are explicitly **not executable** — every `tick_shard` call is commented out. They serve as documentation of expected behavior, not actual tests.

**FINDING:** Cannot assess test parity — there is no implementation to test.

---

## PHASE 2: Farley Engineering Rigor

**SKIPPED — No implementation exists to review.**

---

## PHASE 3: Holzman Rust (The Big 6)

**SKIPPED — No implementation exists to review.**

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

**SKIPPED — No implementation exists to review.**

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

**SKIPPED — No implementation exists to review.**

---

## CRITICAL DEFECTS SUMMARY

### Defect 1: Missing Method [SEVERITY: BLOCKING]
- **File:** `crates/vb_runtime/src/runtime.rs`
- **Issue:** `Runtime::tick_shard` is not implemented
- **Required by:** MASTER.md Section 30
- **Impact:** LETHAL-4 feature is non-functional

### Defect 2: ShardDirective Contract Mismatch [SEVERITY: BLOCKING]
- **Files:** `shard/directive.rs` vs `shard/tests/tick_shard_tests.rs`
- **Issue:** Two incompatible `ShardDirective` enums
- **Impact:** Test specification does not match production type

### Defect 3: ShardDirective Missing Variants [SEVERITY: BLOCKING]
- **Production Missing:** `Migrate { target: u32 }`, `Shutdown`
- **Test Spec Missing:** `Cancel`, `Barrier`
- **Impact:** Core tick_shard functionality cannot be implemented with current types

---

## MANDATORY FIXES

### Fix 1: Implement `Runtime::tick_shard`
```rust
pub fn tick_shard(&mut self, shard_index: u32, directive: ShardDirective) -> RuntimeResult<bool>
```
- Must accept shard index and directive
- Must return `Ok(true)` if shard alive, `Ok(false)` if shard dead/shutdown
- Must return `Err(RuntimeError::ShardNotFound)` for invalid shard index

### Fix 2: Align `ShardDirective` with Test Specification

The production `ShardDirective` in `directive.rs` must be updated to match the test specification:
- Add `Migrate { target: u32 }` variant
- Add `Shutdown` variant
- Keep `Continue`, `Suspend`
- (Optionally keep `Cancel`, `Barrier` if they are used elsewhere)

**Or:**

Update the test specification to match production if `Cancel` and `Barrier` are the correct design.

### Fix 3: Enable Compilation of `tick_shard_tests.rs`

All commented-out `runtime.tick_shard(...)` calls in `tick_shard_tests.rs` must compile and pass once Fix 1 and Fix 2 are complete.

---

## VERDICT

**REJECTED**

LETHAL-4 (`tick_shard`) is **not implemented**. The test file contains executable specifications that document expected behavior but will not compile against current production code.

The implementation must:
1. Add `Runtime::tick_shard` method to `runtime.rs`
2. Fix `ShardDirective` enum contract parity between production and test spec
3. Ensure all commented-out test calls in `tick_shard_tests.rs` compile and pass

No aesthetic or style review is relevant until the basic contract is satisfied.

---

*This review enforces 5 phases. Code cannot proceed past Phase 1 when the required method does not exist.*
